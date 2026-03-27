# Blast Radius Tech Design

This document defines the implementation-ready technical design for the `2026-03-27-blast-radius` feature in `sniff`. It is derived from:

- `sniff/features/2026-03-27-blast-radius/spec.md`
- the current CLI parser in `sniff/cli/src/args.rs`
- the current command execution flow in `sniff/cli/src/commands.rs`
- the current document metadata extraction in `sniff/lib/src/filesystem/docs.rs`
- the current git and repo detection in `sniff/lib/src/filesystem/git.rs` and `sniff/lib/src/filesystem/repo.rs`
- the current filesystem/text renderers in `sniff/cli/src/output/`

The design goal is to turn blast-radius data into a first-class repository workflow: users should be able to list changed source files, filter documents by blast-radius metadata, and ask `sniff` which documents are likely stale because relevant source files changed.

## Overview

This feature introduces three related capabilities:

1. richer `sniff repo` file-listing subcommands for changed files and changed source code
2. blast-radius-aware filtering and improved stream handling for `sniff docs`
3. a new top-level `sniff blast-radius` command that maps changed source files to candidate documents

These capabilities overlap heavily in the implementation. They all need:

- a consistent way to derive changed file sets from git
- a consistent definition of "source code" for change filtering
- document metadata that understands `blast_radius` frontmatter
- a reusable terminal path renderer for OSC8 links, `--list`, `--csv`, and `--no-path`

The design therefore treats blast-radius as a small cross-cutting subsystem rather than as three unrelated CLI patches.

## Goals

1. Add repository subcommands that can list dirty, staged, and unstaged source code files with path filtering and monorepo scoping.
2. Preserve and extend existing changed-file workflows instead of creating a second incompatible path-list implementation.
3. Teach `sniff docs` to understand blast-radius metadata and render verbose metadata with provenance.
4. Add `sniff blast-radius [scope]` to identify documents whose declared `blast_radius` intersects the changed source files in scope.
5. Keep terminal output polished: OSC8 links in styled mode, predictable plain output via the existing global `--plain` switch, and explicit stdout/stderr behavior where the spec requires it.
6. Reuse existing repo/package detection so monorepo scoping stays consistent with the rest of `sniff`.

## Non-Goals

1. Automatically generating or rewriting `blast_radius` frontmatter.
2. Introducing policy evaluation beyond direct blast-radius path matching.
3. Reworking generic filesystem JSON output outside the commands touched by this feature.
4. Redesigning monorepo detection or package discovery.
5. Adding remote or network behavior.

## Current Baseline

The current codebase already contains most of the raw ingredients, but they are not yet composed for this feature.

### CLI baseline

- `sniff repo` already normalizes parser output into `RepoAction`.
- `sniff repo staged-files`, `unstaged-files`, and `untracked-files` already exist, but only as simple early-return git file lists.
- `sniff docs` already supports `--readme`, `--plan`, `--src`, and `--has-prompt`.
- there is no top-level `blast-radius` command

### Git/file-change baseline

- `sniff/lib/src/filesystem/git.rs` exposes `FileChange`, `FileStatus`, and `get_commit_files`.
- `commands.rs` currently filters staged and unstaged lists inline instead of through a shared query layer.
- "source code" currently exists only as an ad hoc extension whitelist in `print_package_area_has_source_code_changes` inside `sniff/cli/src/output/filesystem.rs`.

### Document baseline

- `MarkdownMeta` currently stores `filepath`, `relative`, `package`, `title`, `prompt`, `model`, `last_updated`, and `content_hash`.
- `docs.rs` parses frontmatter, but it does not preserve:
  - `blast_radius`
  - the list of frontmatter keys
  - title provenance
  - updated provenance
- `render_docs_section` currently renders everything into a single stdout string and appends inline metadata rather than nested verbose details.

## Key Design Decisions

### 1. Centralize blast-radius logic in the library

The new command family should not reimplement git filtering in `commands.rs`. A new library module will own:

- changed-file scope resolution
- source-code filtering
- monorepo package/package-area scoping
- document candidate matching against `blast_radius`

Recommended module:

- `sniff/lib/src/filesystem/blast_radius.rs`

This keeps the CLI thin and makes the matching logic unit-testable without invoking the binary.

### 2. Upgrade `staged-files`; do not add a duplicate command

`sniff repo staged-files` already exists. The feature should extend that command with the new list formatting and monorepo filtering rules rather than adding another staged-files variant.

The resulting repo file-list surface becomes:

- `sniff repo dirty-source-code [FILTER...]`
- `sniff repo staged-source-code [FILTER...]`
- `sniff repo unstaged-source-code [FILTER...]`
- `sniff repo dirty-files [FILTER...]`
- `sniff repo staged-files [FILTER...]`
- existing `sniff repo unstaged-files`
- existing `sniff repo untracked-files`

`unstaged-files` and `untracked-files` remain because they already exist and are still useful, even though only `unstaged-source-code` is explicitly added by the spec.

### 3. Keep source-code detection path-based, but move it out of the renderer

The current extension whitelist is in the wrong place, but it exists for a reason: changed-file evaluation must still work for deleted files, which cannot be fully content-classified.

The design is:

- create a shared helper in the library that classifies a changed path as "source-like"
- base it primarily on existing file-type registry knowledge
- preserve explicit path-based fallback behavior for deleted files and extensions such as frontend styling/framework files

Recommended inclusion set for blast-radius/source-code purposes:

- `FileAssociation::ProgrammingLanguage`
- `FileAssociation::FrameworkFile`
- `FileAssociation::Styling`
- explicit HTML path fallback to preserve current frontend behavior

This intentionally excludes Markdown and general documentation.

### 4. `sniff docs` needs split-stream rendering, not just a text tweak

The docs command must place:

- the heading and blank padding on stderr
- the document list on stdout
- the footer and padding on stderr

This cannot be modeled cleanly as a single `String`. The design introduces a small output abstraction for commands that need split streams.

Recommended type in `sniff/cli/src/output/mod.rs`:

```rust
pub struct TextOutput {
    pub stdout: String,
    pub stderr: String,
}
```

Most existing renderers can stay single-string and be wrapped as `TextOutput { stdout, stderr: String::new() }`. `sniff docs` and the new path-list commands can opt into richer control.

### 5. `sniff blast-radius --package/--package-area` scopes the changed file set, not document location

This is the most useful interpretation for monorepos:

- first derive the source files in scope
- then apply package/package-area reduction to that file set
- then find any documents whose declared `blast_radius` intersects that scoped change set

This allows a root-level document to be returned when it references files from a selected package.

## CLI Surface

## `sniff repo` additions

Recommended additions to `RepoSubcommand`:

```rust
DirtySourceCode(FileListArgs),
StagedSourceCode(FileListArgs),
UnstagedSourceCode(FileListArgs),
DirtyFiles(FileListArgs),
StagedFiles(FileListArgs),
```

Where `FileListArgs` is a reusable clap args struct:

```rust
#[derive(clap::Args, Debug, Clone)]
pub struct FileListArgs {
    #[arg(long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
    pub package: Option<String>,

    #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
    pub package_area: Option<String>,

    #[arg(long, conflicts_with = "csv")]
    pub list: bool,

    #[arg(long, conflicts_with = "list")]
    pub csv: bool,

    #[arg(long)]
    pub no_path: bool,

    #[arg(long)]
    pub no_error: bool,

    #[arg(long, value_name = "MESSAGE")]
    pub on_error: Option<String>,

    pub filter: Vec<String>,
}
```

Behavior:

- substring filters are OR-ed together
- `--package` or `--package-area` is applied first
- positional filters are applied after package/package-area narrowing
- no matches:
  - exit `1` with no output by default
  - exit `0` with no output if `--no-error`
  - render `--on-error` through `Prose`
  - if both `--on-error` and `--no-error` are present, write the message to stdout
  - otherwise write the message to stderr

### Existing command compatibility

- `staged-files` adopts the new output controls
- `unstaged-files` and `untracked-files` keep their current semantics for now
- no legacy command is removed in this feature

## `sniff docs` additions

Recommended new flag:

```rust
#[arg(long)]
blast_radius: bool,
```

Extended docs filter:

```rust
pub struct DocsFilter {
    pub readme: bool,
    pub plan: bool,
    pub src: bool,
    pub has_prompt: bool,
    pub blast_radius: bool,
    pub filter: Vec<String>,
}
```

`blast_radius` behaves as an additional AND filter: only documents with a `blast_radius` property present in frontmatter are returned, even if the parsed list is empty.

## New top-level command

Recommended top-level parser shape:

```rust
Commands::BlastRadius {
    #[arg(value_enum, default_value_t = BlastRadiusScopeArg::Dirty)]
    scope: BlastRadiusScopeArg,

    #[arg(long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
    package: Option<String>,

    #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
    package_area: Option<String>,

    #[arg(long, conflicts_with = "csv")]
    list: bool,

    #[arg(long, conflicts_with = "list")]
    csv: bool,

    #[arg(long)]
    no_path: bool,

    #[arg(long)]
    no_error: bool,

    #[arg(long, value_name = "MESSAGE")]
    on_error: Option<String>,
}
```

`BlastRadiusScopeArg` values:

- `dirty`
- `staged`
- `last-commit`

If omitted, scope defaults to `dirty`.

No-result behavior mirrors the repo file-list commands:

- exit `1` with no output by default
- exit `0` with no output if `--no-error`
- render `--on-error` through `Prose`
- if both `--on-error` and `--no-error` are present, write the message to stdout
- otherwise write the message to stderr

## Library Design

### New module: `filesystem::blast_radius`

Recommended public types:

```rust
pub enum ChangeScope {
    Dirty,
    Staged,
    Unstaged,
    LastCommit,
}

pub enum ChangedPathKind {
    AllFiles,
    SourceCode,
}

pub struct ChangedPathQuery {
    pub scope: ChangeScope,
    pub kind: ChangedPathKind,
    pub package: Option<String>,
    pub package_area: Option<String>,
    pub filters: Vec<String>,
}

pub struct ChangedPathResult {
    pub repo_root: PathBuf,
    pub paths: Vec<PathBuf>, // repo-relative
}

pub fn collect_changed_paths(base_dir: &Path, query: &ChangedPathQuery) -> Result<ChangedPathResult>;
pub fn find_blast_radius_documents(base_dir: &Path, scope: ChangeScope, package: Option<&str>, package_area: Option<&str>) -> Result<Vec<MarkdownMeta>>;
```

### Changed file collection algorithm

#### Dirty

Use working tree status:

- staged
- modified
- both
- untracked

Deduplicate on repo-relative path.

#### Staged

Select paths where `FileStatus` is:

- `Staged`
- `Both`

#### Unstaged

Follow existing `unstaged-files` semantics, not the broader English reading of "not staged":

- include `Modified`
- include `Both`
- exclude `Untracked`

This is a deliberate compatibility choice because the current CLI already uses that meaning for "unstaged files".

#### Last commit

Resolve `HEAD` and use `get_commit_files(repo, head_sha)`.

Deleted files are still included by path because they still matter for blast-radius matching.

### Source-code filtering algorithm

Add a shared helper:

```rust
pub fn is_source_code_path(path: &Path) -> bool;
```

Implementation order:

1. exact filename / basename pattern lookup from the file-type registry
2. extension lookup from the file-type registry
3. feature-specific fallback for source-like text extensions currently treated as code in the CLI helper
4. return false otherwise

This helper should replace the current local constant in `print_package_area_has_source_code_changes`, so the feature removes duplicated definitions of "source code".

### Monorepo scoping

Package and package-area filtering should use `RepoInfo`:

- `package` maps to a package root path
- `package_area` maps to one or more package roots sharing the same area prefix
- non-monorepo usage of either flag returns a meaningful error

Recommended error text:

- `--package requires a monorepo with discovered packages`
- `--package-area requires a monorepo with discovered package areas`

### Document metadata expansion

`MarkdownMeta` should be extended so verbose docs output and blast-radius matching do not need to re-parse files in the renderer.

Recommended additions:

```rust
pub enum TitleSource {
    FrontmatterTitle,
    H1Heading,
    H2Heading,
    H3Heading,
    None,
}

pub enum UpdatedSource {
    UpdatedProperty,
    FileMetadata,
}

pub struct MarkdownMeta {
    // existing fields...
    pub has_blast_radius: bool,
    pub blast_radius: Option<Vec<PathBuf>>,      // repo-relative paths
    pub frontmatter_keys: Vec<String>,
    pub title_source: TitleSource,
    pub updated_source: UpdatedSource,
}
```

Parsing rules:

- `blast_radius` must be a YAML sequence
- only string entries are kept
- paths are normalized relative to the repo root when possible
- invalid or non-string entries are ignored rather than failing the document
- `frontmatter_keys` is the sorted set of keys present in frontmatter

`title_source` values for verbose rendering:

- `title property`
- `H1 heading`
- `H2 heading`
- `H3 heading`
- `none`

The current implementation already falls back to H2/H3, and this feature should preserve that behavior rather than silently dropping existing titles.

For updated provenance:

- `updated property` if `last_updated` or `updated_at` is present
- `file metadata` otherwise

## CLI Output Design

### Shared path-list renderer

Add a reusable renderer in `sniff/cli/src/output/filesystem.rs` or a new output helper module.

Recommended API:

```rust
pub enum PathListFormat {
    Lines,
    BulletList,
    Csv,
}

pub fn render_path_list(
    repo_root: &Path,
    paths: &[PathBuf],
    format: PathListFormat,
    no_path: bool,
) -> String;
```

Rendering rules:

- styled mode:
  - OSC8 link target is absolute path
  - visible text is repo-relative
  - directory segments are dimmed
  - basename is bold
- `--no-path`:
  - show only basename
  - still emit OSC8 link
  - no dim/bold directory treatment needed
- `--plain`:
  - unchanged renderer output is passed through existing `emit_text`, which strips OSC8/ANSI escape codes

### `sniff docs` rendering

Replace the single-string docs renderer with a split-stream variant:

```rust
pub(crate) fn render_docs_output(docs: &[MarkdownMeta], verbose: u8) -> TextOutput;
```

Behavior:

- stderr:
  - leading blank line
  - `Docs (# documents)` heading
  - trailing blank line after heading
  - footer and its padding
- stdout:
  - document list only

Verbose item layout:

- top-level bullet: styled linked filepath
- nested bullets:
  - `<b>title:</b> {title} (<dim><i>from {source}</i></dim>)`
  - `<b>updated:</b> {date} (<dim><i>from {source}</i></dim>)`
  - `<b>frontmatter properties:</b> <i>{props}</i>`

Footer text when not verbose:

- `Use --verbose / -v to include metadata for documents`

## `sniff blast-radius` matching algorithm

1. collect changed source-code paths for the requested scope
2. if package/package-area is present, reduce that changed path set using repo metadata
3. collect repository documents from `detect_docs`
4. keep only documents with `has_blast_radius == true`
5. normalize each document blast-radius entry to repo-relative paths
6. return documents where `document.blast_radius ∩ changed_source_paths != ∅`
7. sort returned documents by repo-relative document path

Matching is exact on normalized repo-relative path strings. This avoids accidental partial matches.

## JSON Behavior

The spec is text-focused, but `sniff` has a global `--json` switch. This design fills that gap explicitly.

Recommended JSON shape for new path-list style commands:

```json
{
  "scope": "dirty",
  "kind": "source_code",
  "paths": ["sniff/lib/src/filesystem/git.rs"]
}
```

Recommended JSON shape for `sniff blast-radius`:

```json
{
  "scope": "dirty",
  "documents": ["sniff/docs/cli/repo_git-status.md"]
}
```

Recommended JSON shape for `sniff docs` additions:

- keep existing docs array output
- include the new metadata fields when present

## Shell Completions

The spec requires valid package and package-area completions for:

- repo file-list commands
- `sniff blast-radius`

Recommended completion helpers in `args.rs`:

- `repo_package_candidates()`
- `repo_package_area_candidates()`

Behavior:

- discover the current repo from `std::env::current_dir()`
- call `detect_repo`
- return package names or unique package areas
- return an empty candidate set when outside a repo or outside a monorepo

## Error Handling and Exit Codes

### Repo file-list commands

- success with matches: exit `0`
- no matches:
  - exit `1` with no output by default
  - exit `0` with no output if `--no-error`
- invalid monorepo scoping: exit non-zero with a human-readable error

### `sniff docs`

- no special exit code change
- stream placement must follow the spec even when zero documents match

### `sniff blast-radius`

Recommended behavior:

- matching documents found: exit `0`
- no documents matched:
  - exit `1` with no output by default
  - exit `0` with no output if `--no-error`
  - render `--on-error` through `Prose`
  - if both `--on-error` and `--no-error` are present, write the message to stdout
  - otherwise write the message to stderr

This mirrors the repo file-list commands and makes the command easy to use in automation.

## Testing Strategy

### Library tests

Add unit tests for:

- `is_source_code_path`
- changed-path collection for dirty, staged, unstaged, and last-commit scopes
- monorepo package/package-area scoping
- blast-radius parsing from frontmatter
- document candidate matching

### CLI parser tests

Extend `sniff/cli/src/args.rs` tests for:

- new repo subcommands
- new `sniff docs --blast-radius`
- new `sniff blast-radius`
- clap conflicts for `--list` vs `--csv`

### Integration tests

Add integration tests that create temp repos and verify:

- dirty/staged source-code lists
- repo/package/package-area scoping
- no-result exit behavior with and without `--no-error`
- `--on-error` stream placement
- `sniff blast-radius dirty|staged|last-commit`
- `sniff blast-radius` no-result exit behavior with and without `--no-error`
- `sniff blast-radius --on-error` stream placement

### Snapshot tests

Add snapshots for:

- styled path output
- `--no-path`
- docs verbose nested metadata
- docs footer text change
- docs stderr/stdout split behavior where the harness supports it

## Documentation Updates

This feature changes user-visible behavior and should update docs in the same change:

- `sniff/cli/README.md`
- `sniff/lib/README.md` if public library helpers are exported
- `sniff/docs/cli/` pages for the new and modified commands
- shell completion/help examples in `args.rs` help text

## Implementation Order

1. Extend `MarkdownMeta` parsing with blast-radius and provenance fields.
2. Add the shared library blast-radius module for changed-path and document matching.
3. Move source-code path logic out of the CLI renderer into shared library helpers.
4. Extend `args.rs` with the new repo subcommands, docs flag, top-level blast-radius command, and package/package-area completion helpers.
5. Add reusable path-list rendering and split-stream docs rendering in `sniff/cli/src/output/`.
6. Wire `commands.rs` to dispatch the new routes and emit the correct streams/exit codes.
7. Update docs and add tests.

## Summary

The core design choice is to treat blast-radius as a shared filesystem capability, not as a one-off command. The feature is small at the CLI surface, but it depends on consolidating three things that are currently scattered: changed-file queries, source-code classification, and document frontmatter metadata. Once those are centralized, the repo subcommands, docs filtering, and `sniff blast-radius` command all become straightforward wrappers around the same underlying model.
