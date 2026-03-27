# Blast Radius Implementation Plan

Derived from `spec.md` and `tech-design.md`. Each phase lists the files touched, what changes, and acceptance criteria.

---

## Phase 1: Library foundations

Extend the library with the shared types and logic that all CLI changes depend on.

### 1.1 Extend `MarkdownMeta` with blast-radius and provenance fields

**File:** `sniff/lib/src/filesystem/docs.rs`

Add to `MarkdownMeta`:

```rust
pub has_blast_radius: bool,
pub blast_radius: Option<Vec<PathBuf>>,    // repo-relative paths
pub frontmatter_keys: Vec<String>,
pub title_source: TitleSource,
pub updated_source: UpdatedSource,
```

Add enums:

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
```

Update `parse_markdown_meta`:

- Parse `blast_radius` from frontmatter as a YAML sequence of strings, normalize to repo-relative `PathBuf`s. Non-string entries silently ignored.
- Set `has_blast_radius = frontmatter.contains_key("blast_radius")`.
- Collect `frontmatter_keys` as the sorted set of keys present.
- Track `title_source` during `extract_title` (currently returns only the string; return a tuple or set the enum alongside).
- Track `updated_source` during `resolve_last_updated`.

Add serde skip/default annotations on new fields to preserve backward compatibility of JSON output where appropriate.

**Update exports in** `sniff/lib/src/filesystem/mod.rs`:

- Export `TitleSource`, `UpdatedSource`.

**Acceptance criteria:**

- Existing `docs.rs` unit tests still pass.
- New unit tests cover: blast_radius parsing (valid list, empty list, missing key, non-string entries), frontmatter_keys extraction, title_source for each variant, updated_source for each variant.

---

### 1.2 Add `is_source_code_path` shared helper

**File:** `sniff/lib/src/filesystem/blast_radius.rs` (new)

```rust
pub fn is_source_code_path(path: &Path) -> bool;
```

Implementation:

1. Try `registry::lookup_exact_filename` on the filename; if matched check association.
2. Try `registry::lookup_extension` on the extension; if matched check association.
3. Accept if association is `ProgrammingLanguage`, `FrameworkFile`, or `Styling`.
4. Explicit fallback: accept `.html`/`.htm` extensions (current CLI behavior preserved).
5. Return `false` otherwise.

**File:** `sniff/cli/src/output/filesystem.rs`

- Remove `SOURCE_CODE_EXTENSIONS` const and local `is_source_code_file` function.
- Update `print_package_area_has_source_code_changes` to use `sniff::filesystem::blast_radius::is_source_code_path` instead.

**Update exports in** `sniff/lib/src/filesystem/mod.rs`:

- Add `pub mod blast_radius;`
- Export `is_source_code_path`.

**Acceptance criteria:**

- Unit tests in `blast_radius.rs` cover: `.rs`, `.ts`, `.vue`, `.css`, `.html`, `.md` (rejected), `.json` (rejected), `.png` (rejected), no-extension (rejected).
- `print_package_area_has_source_code_changes` still works (integration via `just test -p sniff-cli`).

---

### 1.3 Add changed-path collection

**File:** `sniff/lib/src/filesystem/blast_radius.rs`

Add types:

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
    pub paths: Vec<PathBuf>,  // repo-relative, sorted
}
```

Add function:

```rust
pub fn collect_changed_paths(base_dir: &Path, query: &ChangedPathQuery) -> Result<ChangedPathResult>;
```

Implementation:

1. Discover repo via `git2::Repository::discover(base_dir)`.
2. Get `file_changes` from `detect_git` (for Dirty/Staged/Unstaged) or `get_commit_files` (for LastCommit).
3. Filter by scope:
   - `Dirty`: Staged + Modified + Both + Untracked (deduplicated on path).
   - `Staged`: Staged + Both.
   - `Unstaged`: Modified + Both (exclude Untracked, matching existing CLI semantics).
   - `LastCommit`: Resolve HEAD, call `get_commit_files`.
4. If `kind == SourceCode`, filter through `is_source_code_path`.
5. If `package` or `package_area` is set, call `detect_repo` to get `RepoInfo`, resolve package root(s), filter paths to those under the package root. Return error if not a monorepo.
6. Apply substring filters (OR logic, case-insensitive).
7. Sort and deduplicate result paths.

**Acceptance criteria:**

- Unit tests cover each scope variant, source-code filtering, package scoping, and substring filter OR logic.

---

### 1.4 Add blast-radius document matching

**File:** `sniff/lib/src/filesystem/blast_radius.rs`

```rust
pub fn find_blast_radius_documents(
    base_dir: &Path,
    scope: ChangeScope,
    package: Option<&str>,
    package_area: Option<&str>,
) -> Result<Vec<MarkdownMeta>>;
```

Implementation:

1. Call `collect_changed_paths` with `kind: SourceCode` and the given scope/package/package_area (no substring filters).
2. Call `detect_docs(repo_root)` to get all documents.
3. Keep documents where `has_blast_radius == true`.
4. For each retained document, check if `document.blast_radius ∩ changed_source_paths != ∅` (exact match on normalized repo-relative path strings).
5. Sort by repo-relative document path.
6. Return matched documents.

**Acceptance criteria:**

- Unit tests cover: document matched, document not matched, empty blast_radius list, no changed files, scope variants.

---

## Phase 2: CLI argument parsing

### 2.1 Add `FileListArgs` shared args struct

**File:** `sniff/cli/src/args.rs`

Add:

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

Add completion helpers:

```rust
fn repo_package_candidates() -> Vec<CompletionCandidate>;
fn repo_package_area_candidates() -> Vec<CompletionCandidate>;
```

Implementation: discover repo from CWD, call `detect_repo`, return package names/areas. Return empty vec if not in a monorepo.

**Acceptance criteria:**

- Compiles. Completion helpers return values in a monorepo and empty vec outside one.

---

### 2.2 Add new `RepoSubcommand` variants

**File:** `sniff/cli/src/args.rs`

Add to `RepoSubcommand`:

```rust
DirtySourceCode(FileListArgs),
StagedSourceCode(FileListArgs),
UnstagedSourceCode(FileListArgs),
DirtyFiles(FileListArgs),
```

Upgrade existing `StagedFiles` variant to use `FileListArgs` (it currently only has `package: Option<String>`).

Add corresponding variants to `RepoAction`:

```rust
DirtySourceCode(FileListArgs),
StagedSourceCode(FileListArgs),
UnstagedSourceCode(FileListArgs),
DirtyFiles(FileListArgs),
StagedFiles(FileListArgs),  // upgraded from just package
```

Update `to_repo_action()` to map the new subcommands.

**Acceptance criteria:**

- `sniff repo dirty-source-code --help` works.
- `sniff repo staged-files --list` parses correctly.
- `--list` and `--csv` conflict properly.
- Parser tests cover all new subcommands.

---

### 2.3 Add `--blast-radius` flag to `sniff docs`

**File:** `sniff/cli/src/args.rs`

Add to `Commands::Docs`:

```rust
#[arg(long)]
blast_radius: bool,
```

Add to `DocsFilter`:

```rust
pub blast_radius: bool,
```

Update `docs_filter()` to populate the new field.

**Acceptance criteria:**

- `sniff docs --blast-radius` parses.
- `DocsFilter` includes the field.

---

### 2.4 Add top-level `sniff blast-radius` command

**File:** `sniff/cli/src/args.rs`

Add `BlastRadiusScopeArg` enum:

```rust
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum BlastRadiusScopeArg {
    #[default]
    Dirty,
    Staged,
    LastCommit,
}
```

Add to `Commands`:

```rust
BlastRadius {
    #[arg(value_enum, default_value_t = BlastRadiusScopeArg::Dirty)]
    scope: BlastRadiusScopeArg,

    #[arg(long, value_name = "PKG", add = ...)]
    package: Option<String>,

    #[arg(long, value_name = "AREA", add = ...)]
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

Add `OutputFilter::BlastRadius` variant.

**Acceptance criteria:**

- `sniff blast-radius --help` works.
- `sniff blast-radius staged --package foo` parses.
- Default scope is `dirty`.

---

## Phase 3: CLI output rendering

### 3.1 Add `TextOutput` split-stream type

**File:** `sniff/cli/src/output/mod.rs`

```rust
pub struct TextOutput {
    pub stdout: String,
    pub stderr: String,
}
```

This is used by commands that need split stdout/stderr. Most existing commands continue returning a plain `String` (wrapped at the call site).

**Acceptance criteria:**

- Type exists and is used by docs and path-list renderers.

---

### 3.2 Add shared path-list renderer

**File:** `sniff/cli/src/output/filesystem.rs`

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

- Styled mode: OSC8 link target is absolute path. Visible text is repo-relative. Directory segments dimmed, basename bold. Uses `format_doc_filepath`-like pattern.
- `--no-path`: show only basename, still emit OSC8 link, no dim/bold.
- Format `Lines`: one path per line.
- Format `BulletList`: prefix each line with `- `.
- Format `Csv`: comma-separated, single line.
- `--plain` stripping handled downstream by existing `emit_text`.

**Acceptance criteria:**

- Snapshot tests for styled output, `--no-path`, BulletList, Csv formats.

---

### 3.3 Refactor `sniff docs` rendering

**File:** `sniff/cli/src/output/filesystem.rs`

Replace `render_docs_section` with:

```rust
pub(crate) fn render_docs_output(docs: &[MarkdownMeta], verbose: u8) -> TextOutput;
```

Changes:

- **stderr**: leading blank line + header (`Docs (N documents)`) + trailing blank + footer (when not verbose) + padding.
- **stdout**: document list only.
- **Footer text** changed to: `Use --verbose / -v to include metadata for documents`
- **Verbose layout**: nested bullets below each doc filepath:
  - `<b>title:</b> {title} (<dim><i>from {source}</i></dim>)`
  - `<b>updated:</b> {date} (<dim><i>from {source}</i></dim>)`
  - `<b>frontmatter properties:</b> <i>{props}</i>`

Title source display values:
- `TitleSource::FrontmatterTitle` -> "title property"
- `TitleSource::H1Heading` -> "H1 heading"
- `TitleSource::H2Heading` -> "H2 heading"
- `TitleSource::H3Heading` -> "H3 heading"
- `TitleSource::None` -> `<yellow>none</yellow>`

Updated source display values:
- `UpdatedSource::UpdatedProperty` -> "updated property"
- `UpdatedSource::FileMetadata` -> "file metadata"

**File:** `sniff/cli/src/output/mod.rs`

Update `filter_docs` to handle the new `blast_radius` filter flag: when set, only keep documents where `has_blast_radius == true`. Applied as AND with other filters.

**Acceptance criteria:**

- Footer text updated.
- Verbose output shows nested metadata below each doc.
- Heading/footer go to stderr; doc list goes to stdout.
- `--blast-radius` filters correctly.
- Snapshot tests for verbose nested layout and footer.

---

## Phase 4: Command wiring

### 4.1 Wire repo file-list commands

**File:** `sniff/cli/src/commands.rs`

Add dispatch for `RepoAction::DirtySourceCode`, `StagedSourceCode`, `UnstagedSourceCode`, `DirtyFiles`, and the upgraded `StagedFiles`.

Each command:

1. Calls `sniff::filesystem::blast_radius::collect_changed_paths` with the appropriate `ChangeScope` and `ChangedPathKind`.
2. Handles monorepo errors (package/package_area on non-monorepo).
3. Determines `PathListFormat` from `FileListArgs` (list/csv/default lines).
4. Calls `render_path_list`.
5. Handles no-results:
   - Default: `std::process::exit(1)` with no output.
   - `--no-error`: exit 0, no output.
   - `--on-error <msg>`: render through `Prose`, write to stderr.
   - `--on-error` + `--no-error`: render through `Prose`, write to stdout, exit 0.
6. Handles `--plain` via existing `emit_text`.
7. JSON mode: output `{ "scope": "...", "kind": "...", "paths": [...] }`.

**Acceptance criteria:**

- `sniff repo dirty-source-code` lists changed source files.
- `sniff repo dirty-files` lists all changed files.
- `sniff repo staged-source-code --list` renders bullet list.
- `sniff repo staged-files --csv` renders comma-separated.
- Exit 1 on no results; exit 0 with `--no-error`.
- `--on-error` message renders to correct stream.
- `--package` and `--package-area` scope correctly.
- `--json` produces structured output.

---

### 4.2 Wire `sniff docs` updates

**File:** `sniff/cli/src/commands.rs`

Update the docs rendering path:

1. Call `render_docs_output` instead of `render_docs_section`.
2. Write `TextOutput.stderr` to stderr.
3. Write `TextOutput.stdout` to stdout via `emit_text`.

**Acceptance criteria:**

- `sniff docs` heading goes to stderr, doc list to stdout.
- `sniff docs --blast-radius` filters correctly.
- `sniff docs -v` shows nested metadata.

---

### 4.3 Wire `sniff blast-radius` command

**File:** `sniff/cli/src/commands.rs`

Add early-return dispatch for `Commands::BlastRadius`:

1. Map `BlastRadiusScopeArg` to `ChangeScope`.
2. Call `find_blast_radius_documents`.
3. Extract document paths.
4. Determine `PathListFormat` from flags.
5. Render with `render_path_list`.
6. Handle no-results identically to repo file-list commands.
7. JSON mode: output `{ "scope": "...", "documents": [...] }`.

**Acceptance criteria:**

- `sniff blast-radius` returns documents whose blast_radius intersects dirty source files.
- `sniff blast-radius staged` scopes to staged files.
- `sniff blast-radius last-commit` scopes to HEAD commit files.
- `--package`/`--package-area` scopes the changed file set.
- No-result behavior matches spec.
- `--json` produces structured output.

---

## Phase 5: Testing

### 5.1 Library unit tests

**File:** `sniff/lib/src/filesystem/blast_radius.rs` (inline `#[cfg(test)]` module)

Tests for:

- `is_source_code_path`: positive cases (`.rs`, `.ts`, `.vue`, `.css`, `.html`), negative cases (`.md`, `.json`, `.png`, no extension).
- `collect_changed_paths`: each scope variant, source-code vs all-files filtering, package scoping, filter OR logic.
- `find_blast_radius_documents`: matching logic, empty intersections, missing blast_radius.

**File:** `sniff/lib/src/filesystem/docs.rs` (extend existing test module)

Tests for:

- `blast_radius` parsing: valid YAML list, empty list, missing key, non-string entries silently dropped.
- `frontmatter_keys` extraction.
- `title_source` for each variant (FrontmatterTitle, H1, H2, H3, None).
- `updated_source` for each variant (UpdatedProperty, FileMetadata).

---

### 5.2 CLI parser tests

**File:** `sniff/cli/src/args.rs` (extend existing test module)

Tests for:

- New repo subcommands parse correctly.
- `--list` and `--csv` conflict.
- `--no-error` and `--on-error` coexist.
- `sniff docs --blast-radius` parses.
- `sniff blast-radius` defaults to dirty scope.
- `sniff blast-radius staged` parses.
- `sniff blast-radius --package foo` parses.

---

### 5.3 Snapshot tests

**File:** `sniff/cli/tests/snapshots.rs` (extend)

Add snapshots for:

- Styled path output (dim directory, bold basename, OSC8 link).
- `--no-path` output (basename only, OSC8 link).
- Docs verbose nested metadata layout.
- Docs footer text.
- Bullet list format.
- CSV format.

---

### 5.4 Integration tests

**File:** `sniff/cli/tests/cli.rs` (extend)

Tests that create temp git repos and verify:

- `sniff repo dirty-source-code` with actual dirty files.
- `sniff repo staged-files --list` with staged files.
- No-result exit code behavior (exit 1 default, exit 0 with `--no-error`).
- `--on-error` stream placement (stderr by default, stdout with `--no-error`).
- `sniff blast-radius` with a document containing `blast_radius` frontmatter matching dirty source files.
- `sniff blast-radius` with no matching documents returns exit 1.

---

## Phase 6: Documentation

### 6.1 Update docs

**Files:**

- `sniff/cli/README.md` - Add new commands to usage examples.
- `sniff/lib/README.md` - Document new public types and `blast_radius` module.
- Help text in `args.rs` - Add clear help strings for all new commands, flags, and examples.

---

## Execution order and dependencies

```
Phase 1.1 (MarkdownMeta extensions)
  └─► Phase 1.2 (is_source_code_path)
       └─► Phase 1.3 (collect_changed_paths)
            └─► Phase 1.4 (find_blast_radius_documents)

Phase 2.1 (FileListArgs) ─┐
Phase 2.3 (docs flag)     ├─► Phase 3 (all rendering) ─► Phase 4 (all wiring)
Phase 2.4 (blast-radius)  ┘

Phase 5 (testing) runs incrementally after each phase.
Phase 6 (docs) runs last.
```

Phases 1 and 2 can be partially parallelized: 2.1 has no dependency on Phase 1 types. Phases 2.3 and 2.4 only need the enum types from 1.1. Phase 3 needs both Phase 1 (for `MarkdownMeta` fields) and Phase 2 (for CLI args). Phase 4 needs everything.

---

## File change summary

| File | Action |
|------|--------|
| `sniff/lib/src/filesystem/docs.rs` | Extend `MarkdownMeta`, add `TitleSource`, `UpdatedSource`, update parsing |
| `sniff/lib/src/filesystem/blast_radius.rs` | **New**: `is_source_code_path`, `collect_changed_paths`, `find_blast_radius_documents` |
| `sniff/lib/src/filesystem/mod.rs` | Add `pub mod blast_radius`, update exports |
| `sniff/cli/src/args.rs` | Add `FileListArgs`, new `RepoSubcommand` variants, `BlastRadiusScopeArg`, `Commands::BlastRadius`, `DocsFilter.blast_radius`, completion helpers |
| `sniff/cli/src/output/mod.rs` | Add `TextOutput`, update `filter_docs` for blast_radius, add `OutputFilter::BlastRadius` |
| `sniff/cli/src/output/filesystem.rs` | Add `render_path_list`, replace `render_docs_section` with `render_docs_output`, remove `SOURCE_CODE_EXTENSIONS`/`is_source_code_file` |
| `sniff/cli/src/commands.rs` | Wire all new commands, split-stream docs output, no-result exit behavior |
| `sniff/cli/tests/cli.rs` | Integration tests |
| `sniff/cli/tests/snapshots.rs` | Snapshot tests |
| `sniff/cli/README.md` | Document new commands |
| `sniff/lib/README.md` | Document new library types |
