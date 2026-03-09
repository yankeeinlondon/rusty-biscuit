# Better File Associations

## Overview

`sniff` should stop treating "detected languages" as a loose bag of file labels and instead move to a structured file-classification model.

Today the codebase has two separate heuristics:

- `filesystem/languages.rs` recursively classifies files by language and then excludes a hardcoded list of non-programming languages when choosing `primary`
- `filesystem/repo.rs` performs a shallow package-root scan for configuration, documentation, `.editorconfig`, and command-runner files

That split creates a few problems:

- the `languages` list still contains configuration and documentation formats
- package-level file categorization is shallow and extension-based
- executable files are not modeled clearly
- framework files such as `.vue` are not first-class concepts
- `sniff language` reports more than programming languages even though users read it as a code-language report

This document proposes a unified file-association system for the library and CLI.

## Goals

- Define a clear, explicit segmentation of file types.
- Distinguish programming languages from configuration, styling, documentation, data, images, binaries, and executable binaries.
- Represent framework files as first-class file types.
- Add programming-language metadata describing how a language executes or compiles.
- Make `sniff language` report only programming-language information.
- Add a new `sniff files` command for broader file-type reporting.
- Support framework files contributing to primary-language detection, including `.vue` mapping to JavaScript or TypeScript.
- Make primary/secondary language selection deterministic and explainable.

## Non-Goals

- This design does not attempt to solve full semantic parsing of every framework file.
- This design does not require perfect MIME detection for every file format in the first implementation.
- This design does not redesign unrelated filesystem features such as git or repo dependency graphs.

## Proposed Library Model

### Core Association Enum

The library should introduce a first-class association enum for broad file grouping.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileAssociation {
    ProgrammingLanguage,
    FrameworkFile,
    Configuration,
    Styling,
    Documentation,
    Data,
    Image,
    Binary,
    BinaryExecutable,
    Archive,
    Font,
    Audio,
    Video,
    Unknown,
}
```

The additional variants beyond the user sketch are useful because they prevent `Binary` from becoming an unhelpful catch-all for archives, fonts, and media.

### Programming Language Execution Type

Programming languages should gain explicit runtime/build metadata.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgrammingLanguageType {
    CompiledBinary,
    CompiledIntermediate,
    Script,
    ShellScript,
}
```

Examples:

- Rust, Go, C, C++, Zig: `CompiledBinary`
- Java, Kotlin, Scala, Clojure, C#, F#, WebAssembly text/source: `CompiledIntermediate`
- JavaScript, TypeScript, Python, Ruby, Lua, PHP: `Script`
- Shell, Bash, Zsh, Fish, PowerShell, Batch: `ShellScript`

### Programming Language Identity

The existing code uses raw strings for languages. For structured behavior, the library should define a stable language enum and only convert to display strings at the edge.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Shell,
    PowerShell,
    // ...
}
```

This avoids repeated string allowlists like the ones currently duplicated in the CLI output layer.

### Framework Identity

Framework files should not be flattened into ordinary language files.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameworkKind {
    Vue,
    Svelte,
    Astro,
    AngularTemplate,
    RemixRouteModule,
    NextAppRouter,
    Unknown,
}
```

Not every framework file needs a variant on day one. The initial implementation can start with single-file component formats and grow from there.

### File Type Descriptor

Each classified file should carry enough metadata to answer both `sniff language` and `sniff files`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeDescriptor {
    pub association: FileAssociation,
    pub language: Option<ProgrammingLanguage>,
    pub language_type: Option<ProgrammingLanguageType>,
    pub framework: Option<FrameworkKind>,
    pub is_text: bool,
}
```

This descriptor is the registry output for a file type rule. Runtime classification may attach more detail, described below.

### Per-File Classification Result

The scanning layer should return richer results than a single label string.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileClassification {
    pub path: PathBuf,
    pub association: FileAssociation,
    pub language: Option<ProgrammingLanguage>,
    pub language_type: Option<ProgrammingLanguageType>,
    pub framework: Option<FrameworkKind>,
    pub related_languages: Vec<ProgrammingLanguage>,
    pub confidence: ClassificationConfidence,
    pub source: ClassificationSource,
}
```

Where:

- `language` is the direct language when the file itself is a normal code file
- `framework` marks `.vue`, `.svelte`, `.astro`, etc.
- `related_languages` captures framework-associated languages such as TypeScript for `Component.vue`
- `confidence` allows later UI and tests to distinguish exact vs inferred mappings
- `source` records whether the result came from an exact filename rule, extension rule, shebang, container-file parser, or binary sniffing

Suggested supporting enums:

```rust
pub enum ClassificationConfidence {
    Exact,
    High,
    Medium,
    Low,
}

pub enum ClassificationSource {
    ExactFilename,
    Extension,
    Shebang,
    EmbeddedLanguageHint,
    BinarySignature,
    Fallback,
}
```

## Registry Design

### Authoritative Lookup Tables

The library should own an internal registry that maps:

- exact filenames
- exact basename patterns
- file extensions
- optional binary signatures

to `FileTypeDescriptor`.

Proposed module layout:

```txt
sniff/lib/src/filesystem/
├── file_types/
│   ├── mod.rs
│   ├── model.rs          # enums and structs
│   ├── registry.rs       # static lookup tables
│   ├── classify.rs       # per-file classification pipeline
│   ├── aggregate.rs      # report aggregation
│   └── framework.rs      # .vue/.svelte/.astro parsing helpers
├── languages.rs          # language-only aggregation built on file_types
└── repo.rs               # package scanning uses file_types outputs
```

The registry should be authoritative for `sniff` reporting. During migration, `hyperpolyglot` can remain as a fallback for unknown text files, but it should stop being the primary model.

### Rule Kinds

The registry needs more than extension-only matching because many important files are extensionless.

1. Exact filename rules
   - `Dockerfile`
   - `Makefile`
   - `Justfile`
   - `Brewfile`
   - `Procfile`
2. Extension rules
   - `rs`, `ts`, `tsx`, `js`, `jsx`, `py`, `go`, `css`, `scss`, `md`, `toml`, `yaml`, `png`, `pdf`
3. Executable container rules
   - `.app` on macOS
   - `.exe`, `.dll`, `.so`, `.dylib`
4. Shebang rules
   - `#!/usr/bin/env bash`
   - `#!/usr/bin/env python`
   - `#!/usr/bin/env node`
5. Binary signature rules
   - PDFs, images, common media, archives, ELF/Mach-O/PE files

### Extension Coverage

The registry should aim to cover every file extension that `sniff` wants to report meaningfully, but it should not depend exclusively on extensions:

- some important files have no extension
- many binaries are better identified by signature than suffix
- shebangs matter for extensionless scripts

The practical rule should be:

- extension coverage is comprehensive for known text formats
- exact filename and signature tables fill the gaps
- unknown files remain representable as `Unknown`

## Classification Pipeline

The classification pipeline should be a single reusable path for repo-level and package-level scanning.

### Step 1: File Discovery

Walk files recursively while respecting:

- `.gitignore`
- global git ignores
- `.git/info/exclude`
- the existing excluded dependency/build directories

This behavior is already correct in the current language scanner and should be preserved.

### Step 2: Cheap Structural Classification

For each file:

1. exact filename lookup
2. extension lookup
3. shebang lookup for extensionless or ambiguous text files
4. binary signature lookup for files still unresolved

This pass should produce a `FileClassification` for most files without opening full contents.

### Step 3: Framework Enrichment

For `FrameworkFile` results, run a small targeted parser.

Examples:

- `.vue`
  - inspect `<script lang="ts">`, `<script setup lang="ts">`, or plain `<script>`
  - if explicit `lang="ts"` or `lang="tsx"`, relate file to `TypeScript`
  - otherwise relate file to `JavaScript`
- `.svelte`
  - same approach for `<script lang="ts">`
- `.astro`
  - use frontmatter/script hints when present, otherwise default to the JavaScript ecosystem

This should be implemented as a lightweight content parser, not a full AST.

### Step 4: Unknown Fallback

For text files not matched by the registry:

- optionally ask `hyperpolyglot`
- if it returns a language known to the internal model, normalize to internal enums
- otherwise keep `Unknown`

This gives the project a practical migration path while still moving the model into `sniff`.

## Aggregation Model

The scanning pipeline should feed two different aggregations:

1. broad file-association reporting for `sniff files`
2. programming-language reporting for `sniff language`

### Broad File Aggregation

Proposed structure:

```rust
pub struct FileAssociationBreakdown {
    pub total_files: usize,
    pub by_association: Vec<FileAssociationStats>,
    pub by_language: Vec<ProgrammingLanguageStats>,
    pub by_framework: Vec<FrameworkStats>,
}
```

Where `by_association` answers questions like:

- how many documentation files?
- how many images?
- how many binary executables?

and `by_language` / `by_framework` provide drill-downs.

### Language Aggregation

The language report should operate only on:

- `FileAssociation::ProgrammingLanguage`
- `FileAssociation::FrameworkFile`

It should ignore:

- configuration
- styling
- documentation
- data
- image
- binary
- binary executable
- archive
- media

That satisfies the desired behavior for `sniff language`.

## Primary and Secondary Programming Language Logic

### Definitions

- `primary language`: the programming language with the strongest code signal in the scan scope
- `secondary languages`: the remaining programming languages, ordered by descending signal

### Signal Sources

A programming-language signal can come from:

1. Direct code files
   - `.rs` -> Rust
   - `.py` -> Python
2. Framework files with explicit embedded language
   - `.vue` + `lang="ts"` -> TypeScript
3. Framework files with ecosystem-default language
   - `.vue` without `lang` -> JavaScript

### Recommended Scoring

Use deterministic weighted counts:

- direct programming-language file: `1.0`
- framework file with explicit embedded language: `1.0`
- framework file with inferred ecosystem-default language: `0.75`

The reduced weight for inferred framework mappings acknowledges that a `.vue` file is not identical to a plain `.ts` file, but it still meaningfully indicates the package’s main language family.

The weighting should be configurable in code constants, not hardcoded inside aggregation logic.

### Framework Handling

A framework file should remain visible as a framework file in `sniff files`, but it should still contribute to language totals in `sniff language`.

Example:

```txt
Component.vue
association: FrameworkFile
framework: Vue
related_languages: [TypeScript]
```

This file counts as:

- one Vue framework file
- one TypeScript language signal

### Tie-Breaking

Primary-language selection must be deterministic.

Recommended ordering:

1. higher weighted signal
2. higher direct-file count
3. higher explicit-framework count
4. lexical language name

That avoids the current unstable tie behavior that comes from sorting only by file count.

### Output Semantics

The language report should explicitly separate:

- total files scanned
- files considered for language selection
- direct language files
- framework-derived language files

This keeps the result explainable.

Example summary:

```txt
Primary language: TypeScript
Secondary languages: JavaScript, Rust

TypeScript
- 18 direct files
- 7 Vue files with lang="ts"

JavaScript
- 4 direct files
- 3 Vue files using default script language
```

## `sniff language` Changes

### Behavioral Change

`sniff language` should only consider:

- programming languages
- framework files

It should no longer report configuration, documentation, styling, or data formats as "languages" in that command.

### Proposed Output Shape

For text output:

- primary language
- secondary languages
- programming language table
- optional framework breakdown

For JSON:

```rust
pub struct LanguageBreakdown {
    pub primary: Option<ProgrammingLanguage>,
    pub secondary: Vec<ProgrammingLanguage>,
    pub total_files_scanned: usize,
    pub total_language_files: usize,
    pub languages: Vec<ProgrammingLanguageStats>,
    pub frameworks: Vec<FrameworkStats>,
}
```

This is narrower and more semantically correct than the current mixed-language list.

## New `sniff files` Command

### Purpose

`sniff files` should report broad file typing, not just programming languages.

This command becomes the place to answer questions like:

- how much of this package is code vs config vs docs?
- are there binary executables in this tree?
- how many images or PDFs are present?
- what framework file types are present?

### Suggested CLI Surface

Initial version:

```txt
sniff files
sniff files --association documentation
sniff files --association image
sniff files --package
sniff files --json
```

Possible future flags:

- `--extensions`
- `--frameworks`
- `--top N`
- `--package <name>`
- `--include-unknown`

### Suggested Text Output

```txt
Files

Programming languages   184
Framework files          22
Configuration            47
Styling                  13
Documentation            18
Data                      6
Images                    9
Binary executables        1
Binary                    4
Unknown                   2
```

At higher verbosity it can show:

- per-association extensions
- top files per group
- per-framework counts
- executable file paths

## Package Metadata Changes

The current `Package` model should stop using `languages: Vec<String>` as its only file-type signal.

Recommended additions:

```rust
pub struct Package {
    pub primary_language: Option<ProgrammingLanguage>,
    pub secondary_languages: Vec<ProgrammingLanguage>,
    pub languages: Vec<ProgrammingLanguageStats>,
    pub frameworks: Vec<FrameworkStats>,
    pub file_associations: Vec<FileAssociationStats>,
    // existing fields...
}
```

The existing shallow `configuration`, `documentation`, `editor_config`, and `command_runner` fields can be preserved for compatibility initially, but they should eventually be derived from the unified file-classification pass instead of a separate root-only scan.

## Migration Strategy

### Phase 1: Introduce the New File-Type Registry

- add `filesystem/file_types`
- define the new enums and descriptor structs
- build exact-filename and extension tables
- keep `hyperpolyglot` as fallback for unknown text files

### Phase 2: Rebuild Language Detection on Top of File Classification

- make `filesystem/languages.rs` aggregate from `FileClassification`
- restrict it to programming languages and framework files
- add deterministic primary/secondary selection

### Phase 3: Add Framework Parsing

- `.vue`
- `.svelte`
- `.astro`

This phase is what allows `sniff language` to associate a framework file with JavaScript or TypeScript rather than treating it as an unstructured blob.

### Phase 4: Add `sniff files`

- add new CLI subcommand
- add JSON serialization for file-association reports
- expose framework and executable counts

### Phase 5: Migrate Repo and Package Metadata

- update `repo.rs` package creation to use unified file scans
- replace or de-emphasize shallow package-root file categorization
- update package output rendering

## Testing Strategy

### Registry Tests

- extension -> descriptor mapping
- exact filename -> descriptor mapping
- shebang classification
- binary-signature classification

### Framework Tests

- `.vue` with `lang="ts"` -> FrameworkFile + related `TypeScript`
- `.vue` without `lang` -> FrameworkFile + related `JavaScript`
- `.svelte` and `.astro` equivalents

### Aggregation Tests

- `sniff language` excludes configuration and docs
- `sniff files` includes them
- deterministic tie-breaking
- primary/secondary language ordering
- framework files contribute to language totals

### Package Tests

- mixed-language monorepo package
- framework-heavy frontend package
- binary-executable presence in package tree
- docs/config-heavy package with a small amount of real code

## Recommended Defaults

- Treat shell scripts as `ProgrammingLanguage` with `ProgrammingLanguageType::ShellScript`.
- Treat `.vue`, `.svelte`, and `.astro` as `FrameworkFile`, not plain `ProgrammingLanguage`.
- Keep styling separate from programming languages.
- Keep documentation separate from programming languages.
- Make binary executable detection explicit and visible.
- Keep unknown files representable rather than forcing every file into a misleading bucket.

## Summary

The key design change is to stop asking "what language is this file?" as the first question and instead ask "what kind of file is this?".

Once `sniff` has a unified `FileAssociation` model, the rest becomes much simpler:

- `sniff language` can become a true code-language report
- `sniff files` can report the broader makeup of a tree
- framework files can be first-class and still influence primary-language selection
- package metadata can become more accurate and less heuristic-heavy

This produces a model that is more explicit, easier to test, and better aligned with how users think about real repositories.
