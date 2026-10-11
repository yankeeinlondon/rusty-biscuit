# Using the `hug` CLI

`hug` lists the declarations, imports, references and diagnostics in source
files written in 16 languages, and finds files that have grown too large. This
page is the reference for every command and option. Where a command has a
detailed contract of its own, this page summarizes it and links to it.

```bash
hug symbols                                   # every declaration in the nearest package
hug functions --in Widget --undocumented      # undocumented members of Widget
hug refs parse --callers 'src/**'             # who calls `parse`
hug lint 'src/**/*.rs'                        # lint and syntax diagnostics
hug god-files --high-risk                     # files that most need splitting
```

## Install

```bash
cargo install --path tree-hugger/cli
```

Then enable shell completions (see [completions](#completions)).

## Commands

| Command | What it does |
|---------|--------------|
| `functions` | List function, method and constructor definitions |
| `types` | List type definitions (structs, unions, enums, classes, interfaces, traits, type aliases) |
| `symbols` | List all discovered symbols (locals hidden unless `--all-scopes`) |
| `classes` | List classes, structs, interfaces and traits with members partitioned by static/instance |
| `refs` | List the references to one declaration within its file, or its callers or callees |
| `imports` | List imported symbols |
| `lint` | Run lint and syntax diagnostics |
| `god-files` | Identify oversized source files that are strong candidates for refactoring |
| `completions` | Generate a shell-completion script |

`functions`, `types`, `symbols` and `classes` are the **declaration
commands**. They share the selectors and output modes described in
[Declaration commands](#declaration-commands).

## Options for every command

These may appear before or after the command name:

- `--json` - Output as JSON
- `--json-full` - Output every scanned file's complete index as JSON, diagnostics included (`functions`, `types`, `symbols` and `classes` only; symbol selectors such as name filters, `--exported`, `--prelude` or `--static-only` are rejected)
- `--plain` - Disable colors and hyperlinks
- `--verbose` - Print the cache stats line (persistent-cache hits, partial hits and misses, plus files parsed, definition captures and diagnostics runs) and any recoverable cache failure to stderr

## Choosing files

Every command except `completions` and `god-files` takes file filters as
positional path or glob arguments. The declaration commands and `imports`
also accept `--file`:

- `--file <PATH|GLOB>` - Scan this file or glob (repeatable). Unlike a positional filter it is never taken for a symbol name, and a file it names is scanned even when ignored or under a fixtures directory

These work on every command that takes file filters:

- `--language <LANG>` - Override language detection (`js`/`javascript`, `ts`/`typescript`, `cpp`/`c++`, `c-sharp`/`c#`); directory scans are restricted to that language's extensions
- `--exclude-files <GLOB>` - Exclude files matching glob patterns (in addition to the defaults below)
- `--no-cache` - Neither read nor write the persistent symbol cache; the output is the same as a cached run's

Filters are globs, so a bare directory selects nothing: write `'src/**'`,
not `src/`.

### Default scan exclusions

A whole-package scan (no path/glob argument) honors `.gitignore` and additionally
skips test *data* directories, whose contents are inputs rather than compiled
source: `**/fixtures/**`, `**/__fixtures__/**`, `**/snapshots/**`, `**/testdata/**`.
Naming such a file explicitly (e.g. `hug lint tests/fixtures/sample.rs`) overrides
the exclusion. Normal test code (`tests/*.rs`) is still scanned.

## Declaration commands

`functions`, `types`, `symbols` and `classes` share everything in this
section. [Symbol Reports](./symbol-reports.md) covers them in depth: what each
command analyzes, the flow from query to output, every output mode, ordering,
failed files and exit status, and `--verbose`.

### Selectors

Selectors combine with AND; the values of a repeated selector are
alternatives. Names use smart case: a pattern without an uppercase letter
ignores case.

- `--file <PATH|GLOB>` / `--name <PATTERN>` (`-n`) - Say whether a filter is a file or a name instead of letting `hug` guess from a positional token; `--exact` matches names whole and literally
- `--exclude-symbols <PATTERN>` - Exclude symbol names matching these patterns
- `--kind <KIND>` - Only these kinds (snake_case: `function`, `type_alias`, `enum_variant`, …); narrows the command's kinds, never widens them
- `--scope item|member|local`, `--all-scopes` - Only these scopes; `symbols` hides locals (`item` and `member` by default), the other commands show every scope
- `--visibility <LEVEL>` - Exactly this visibility (`public`, `crate`, `private`, …, `unknown`)
- `--exported` - Only exported declarations (declarations only; forwarding is in `--json-full`'s `reexports`, and `--prelude` adds synthetic Rust prelude records)
- `--prelude` - Only symbols explicitly exported by the prelude module
- `--in <PATTERN>` - Only members of a matching container (`classes`: the container itself)
- `--exclude-tests`, `--tests-only` - Drop, or keep only, test code and helpers in test files
- `--undocumented`, `--deprecated` - No doc comment / marked deprecated
- `--async`, `--returns <GLOB>`, `--param-type <GLOB>`, `--min-params <N>` - Signature shape; keeps only functions, methods and constructors
- `--changed[=<REF>]` - Only files Git reports as changed (see [Changed files](#changed-files))

```bash
hug symbols --kind trait --exported            # exported traits
hug functions --in Widget --undocumented        # undocumented Widget members
hug functions --async --returns 'Result<*'      # async functions returning a Result
hug symbols --all-scopes --name '*/lib.rs#*'    # everything in any lib.rs, locals included
```

Every selector, with examples, is in
[Selecting declarations](./symbol-reports.md#selecting-declarations).

### Output modes

Exactly one output mode is active (terminal, `--json`, `--json-full`,
`--jsonl`, `--count`, `--stats` or `--format`); `--stats` alone composes,
with `--json` or a `--format`. Interactive output wraps at the terminal width
(a positive `COLUMNS` overrides it); `--plain`, `--format plain` and
redirected output never wrap.

- `--jsonl` - One JSON object per selected declaration (JSON Lines); a `classes` line adds the container's `members` list
- `--count` - Print only the number of selected declarations (`classes`: containers)
- `--stats` - Counts by kind, visibility and language plus doc coverage, with overall totals and automatic package sections (see [Package stats](#package-stats))
- `--format plain|table|tsv|markdown|prose|ctags` - Select a terminal listing or a UTF-8 export (see [Exports](#exports))
- `--tree` - Nest members under their container (`functions`, `types`, `symbols`); type rows end with their declared bases (`extends Base; implements Shape; unclassified bases External`)

These shape the listing:

- `--sort file|name|kind` - Order of the listed declarations. `file` (the default) lists by file, then source position; `name` by the case-sensitive name; `kind` by kind (modules and types, then callables, then values). Ties fall back to file, position, kind and ID. Give it once; `lint` and `imports` reject it. See [Ordering](./symbol-reports.md#ordering)
- `--signatures full|short|none` - The declaration header as written, the compact form (default; field lists past four end in `… +N`), or names only
- `--group-by-file` - Group symbol output by file
- `--group-by-module` - Group symbol output by module (the file's directory)
- `--comments` - Show symbol-level documentation comments

### The default listing

Each symbol line shows its visibility, kind, name (with signature or members),
container and location:

```
  - public struct PublicStruct { public_field: u32, crate_field: u32, private_field: u32 } src/rust.rs:50:1
  - public method new_associated_function() -> Self [in type PublicStruct] src/rust.rs:94:5
  - crate field crate_field [in type PublicStruct] src/rust.rs:52:16
  - method fmt(self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result [in type PublicStruct] src/rust.rs:130:5
  - private enum variant PrivateEnumVariant [in enum PrivateEnum] src/rust.rs:72:5
  - private type alias PrivateAlias src/rust.rs:30:1
```

The word before the name is the declaring keyword for types, variables,
fields and constants (`struct`, `let`, `const`, `my`, `local`, ...) and the
kind otherwise. A missing visibility label means the level is not known (as
for `fmt` above, a Rust `impl` of a trait declared in another file), and a
member ends with `[in <kind> <name>]` naming its container.
[Terminal Output](./symbol-reports/terminal-output.md) explains the labels,
containers, kind colors, wrapping and the plain, table and outline layouts.

The listing is printed as files are analyzed, so `hug symbols | head` stops
the scan after a window or two. When the scanned files span two or more
packages, the listing has one section per package.

### Exports

Four `--format` values write a UTF-8 file instead of a terminal listing:

```bash
hug functions --format tsv > functions.tsv          # eight fixed tab-separated columns
hug symbols --format markdown --comments > api.md   # the listing as headings and nested lists
hug functions --format prose > functions.prose      # styled Prose markup with local source links
hug symbols --format ctags > tags                   # an unsorted format-2 tags file for Neovim
```

Markdown and Prose keep the listing's layout (package sections, groups,
`--tree` nesting and the `classes` sections). ctags skips a record Neovim
would not read back as written, warns, and exits `1`, so an incomplete tags
file never looks complete. Escaping, option combinations and the supported
tags subset are in [Text Exports and Tags](./symbol-reports/exports.md).

### JSON output

With `--json`, `functions`, `types`, `symbols` and `classes` print one report:

```json
{
  "report_schema_version": 1,
  "schema_version": { "major": 5, "minor": 1 },
  "root_dir": "/path/to/project",
  "files": [
    {
      "file": "src/rust.rs",
      "language": "Rust",
      "completed_passes": ["Parse"],
      "completed_capabilities": ["declarations", "flags"],
      "symbols": [
        {
          "id": "…",
          "kind": "Method",
          "identity": { "name": "new_associated_function", "container_name": "PublicStruct", "container_kind": "Type", "keyword": "fn" },
          "source": { "file_path": "src/rust.rs", "signature_text": "pub fn new_associated_function() -> Self" },
          "visibility": { "level": "Public" },
          "modifiers": [{ "type": "Static" }]
        }
      ]
    }
  ],
  "errors": []
}
```

(Records are shown abridged.) A file with no match has no entry, every path
is relative to `root_dir`, and `errors` is always present and always last.
`--jsonl` writes one self-contained object per declaration, and
`--json-full` writes every scanned file's complete index. The envelopes,
what a record leaves out and the path rules are in
[JSON Reports](./symbol-reports/json-reports.md); the record fields are
defined in [Symbol Records](./symbol-records.md).

### Package stats

`--stats` replaces the listing with counts by kind, visibility and language
plus doc coverage, first for **All selected declarations**, then for each
package that holds a selected file:

```text
$ hug --plain symbols --stats
All selected declarations
…
Doc coverage: 2 of 6 documented (33.3%); 0 unavailable

shapes (.)
…
Doc coverage: 2 of 6 documented (33.3%); 0 unavailable
```

```bash
hug symbols --stats
hug functions --stats --json
hug classes --stats --format plain
```

Selectors apply first, and `classes` counts containers. With `--json`, the
envelope keeps the overall `stats` and adds one `groups` entry per package.
[Package Stats](./symbol-reports/package-stats.md) covers package
assignment, doc coverage, discovery failures and the JSON shape.

### Changed files

`--changed` narrows a declaration command to the files Git reports as
changed, and lists their current on-disk declarations:

```bash
hug functions --changed                 # staged, unstaged, nonignored untracked files
hug symbols --changed=main              # also compare the local main tip directly to HEAD
hug functions --changed --count         # 0 when no eligible changed files remain
```

Bare `--changed` selects uncommitted changes only; a reference needs `=`
(`--changed main` leaves `main` as a positional filter). The changed set
intersects the ordinary file selection, an empty result exits `0`, and Git
failures exit `1` before any report. `--changed` conflicts with `--prelude`.
[Changed Files](./symbol-reports/changed-files.md) has the selection rules,
symlink and nested-repository handling, and every error case.

## classes

The `classes` command summarizes Class, Type (struct, union, record),
Interface and Trait symbols. Methods and constructors are partitioned by
whether they need an instance, and so are the fields in the container's type
metadata. Storage `static` (a C++ function-local `static`, for example) is
never listed as a static member. Rust `impl` members appear under their type,
with associated functions under *Static Methods*; a Java interface constant
appears under *Static Fields*:

```bash
hug classes "src/**/*.ts"
```

Output shows classes with members partitioned:

```
src/service.ts (TypeScript)
  class UserService [5:1]
    Static Methods (2)
      - public getInstance() [7:3]
      - private validateConfig(config: Config) [12:3]
    Instance Methods (3)
      - public getUser(id: string): User [20:3]
      - private fetchData() [25:3]
    Static Fields (1)
      - private instance: UserService
    Instance Fields (2)
      - private cache: Map<string, User>
      - public readonly config: Config
```

For Rust, inherent and trait-impl methods are listed under the type they
implement:

```
src/rust.rs (Rust)
  type PublicStruct [50:1]
    Static Methods (2)
      - public new_associated_function() -> Self [in type PublicStruct] [94:5]
      - crate crate_associated_function() -> () [in type PublicStruct] [108:5]
    Instance Methods (4)
      - public instance_method(self) -> u32 [in type PublicStruct] [102:5]
      - private private_instance_method(self) -> () [in type PublicStruct] [106:5]
      - public required_trait_method(self) -> u32 [in type PublicStruct] [120:5]
      - fmt(self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result [in type PublicStruct] [130:5]
    Instance Fields (3)
      - public public_field: u32
      - crate crate_field: u32
      - private private_field: u32
```

Besides the declaration-command options:

- `--name <FILTER>` - Filter classes by name
- `--static-only` - Show only static members
- `--instance-only` - Show only instance members

## refs

`hug refs` lists where one declaration is referenced, with how sure the
analysis is about each reference:

```sh
hug refs parse 'src/**'                                  # incoming calls and usages
hug refs 'src/parser.rs#Parser::parse' --callers 'src/**' # incoming calls only
hug refs --id <ID> --callees 'src/**'                     # calls its body makes
```

- The first value is the declaration's name (exact, smart case, every kind
  and scope, locals included); the rest are file filters. `--id` selects by
  record ID instead, and then every value is a file filter.
- A name matching several declarations is an error (exit 1) listing each
  one's location and ID; an unknown name or ID is an empty report.
- References are resolved within one file, so a caller in another file is
  never listed, even under the same name. Every report says so.
- TypeScript type-only imports do not compete with same-named local value
  declarations: `import type { F }` and `function F() { F(); }` report an
  established local self-call; an `F` type annotation still names the import.
- Text and `--json` only; `--exclude-files`, `--language` and `--no-cache`
  work as for the other commands.
- Only the declaring file's occurrences are analyzed: the other files are
  read for their declarations alone, so their cache entries stay small.

`hug refs --id` matches equivalent Rust raw/NFC and TypeScript/JavaScript
escaped identifier spellings while displaying the token as written. Imported
string export names decode as one name; file-local resolution still supplies
no external declaration ID.

The full contract is in [`hug refs` in full](./references/hug-refs.md).

## imports

```bash
hug imports "src/**/*.js" --plain
hug imports --json "tests/fixtures/**/*.js"
```

Positional values are always file filters, never import names; use `--name`
to match names. `imports` takes `--file`, `--name`, `--exact`,
`--exclude-symbols`, `--comments`, `--group-by-file` and `--group-by-module`
besides the file-choice options. Text output is printed as each file is
analyzed, so `hug imports | head` stops the scan early.

## lint

The `lint` command runs pattern-based and semantic diagnostics:

```bash
hug lint "src/**/*.rs"
```

Output shows categorized diagnostics:

```
src/main.rs (Rust)
[lint] warning [unwrap-call]: Explicit `.unwrap()` call
  --> src/main.rs:42:15
    |
 42 |     let value = result.unwrap();
    |                        ^^^^^^

[semantic] warning [undefined-symbol]: Reference to undefined symbol 'foo'
  --> src/main.rs:50:5

[syntax] error: Missing semicolon
  --> src/main.rs:60:20
```

Options:
- `--lint-only` - Show only lint diagnostics (pattern rules and semantic analysis)
- `--syntax-only` - Show only syntax diagnostics (parse errors)
- `--deny <RULE|CATEGORY>` - Treat matching lint rules as errors
- `--warn <RULE|CATEGORY>` - Treat matching lint rules as warnings
- `--allow <RULE|CATEGORY>` - Treat matching lint rules as info
- `--strict` - Promote warnings to errors
- `--experimental-semantics` - Enable experimental semantic diagnostics
- `--list-rules` - Print registered rule metadata and exit

A `--deny`, `--warn` or `--allow` value that is not a registered rule ID or
alias, such as a typo or a rule that has been removed, is still accepted but
matches nothing, so `hug` prints a warning on stderr and carries on. Each
distinct name is reported once per run, however many flags repeat it; `all`
and `category:<name>` are not rule names and are never reported. The warning
does not change the exit status:

```text
$ hug lint --deny unwrap-cal --warn unwrap-cal src/main.rs
warning: unknown rule 'unwrap-cal'
src/main.rs (Rust)
  (no diagnostics)
```

For `lint`, a multi-file scan only prints files that have diagnostics; if every
file is clean it prints a single `No lint diagnostics in N files.` line. A
single-file lint still prints `(no diagnostics)` as explicit confirmation.
"Multi-file" counts the selected files, so a run whose other files failed to
read still hides a clean file; `N` counts the files that were linted. Text output is printed as each file is
analyzed.

`lint` exits `1` when an error-severity diagnostic remains or a file could not
be analyzed. Like the declaration commands, it ends quietly with `0` when its reader closes
early (`hug lint 'src/**' | head`); `imports` does the same.

## god-files

`god-files` finds the files that have grown too large and suggests where to
split them:

```bash
hug god-files                  # scan the current directory
hug god-files crates/core      # scan one directory
hug god-files --high-risk      # only the high-risk band
hug --json god-files           # a JSON array, one object per candidate
```

Size is measured in **effective SLOC**: physical lines minus blank and
comment-only lines. A file is a candidate from 400 effective SLOC
(**moderate** risk) and is **high** risk from 1,000. High-risk files are
listed first, then by descending size.

For each candidate the report shows:

- its largest symbol blocks (up to eight, each at least 15 SLOC), with each
  symbol's doc summary
- the number of top-level symbols, maximum nesting depth, import count,
  TODO/FIXME count and comment density
- refactor hints: one symbol dominating the file, many unrelated top-level
  symbols, deep nesting, high coupling (many imports), or low code density

```
High risk
- the tree_file/relations.rs file is 1792 lines of code
  - the largest blocks in this file are composed by these symbols:
    - method resolve_path  [1805–1934]  120 sloc — …
    - …and 24 more
  - top-level symbols: 54 · max depth: 5 · imports: 44 · TODO/FIXME: 0 · comments: 24%
  - likely refactor: 54 unrelated top-level symbols — split by responsibility
```

`god-files` takes an optional directory, not file filters, and accepts only
`--high-risk` and the [options for every command](#options-for-every-command).

## completions

`hug completions <SHELL>` prints a registration script for `bash`, `zsh`,
`fish`, `powershell` or `elvish`. The script calls back into the installed
`hug` for each completion, so completions always match the installed version
and can offer context-aware candidates such as source files in the current
directory.

```bash
echo 'source <(hug completions bash)' >> ~/.bashrc          # Bash
echo 'source <(hug completions zsh)' >> ~/.zshrc            # Zsh
hug completions fish > ~/.config/fish/completions/hug.fish  # Fish
hug completions powershell >> $PROFILE                      # PowerShell
```

## Exit status

| Status | Meaning |
|--------|---------|
| `0` | Success, including an empty selection and a reader that closed early (`hug … \| head`) |
| `1` | A file could not be analyzed, `lint` found an error-severity diagnostic, `refs` matched several declarations, a ctags record was skipped, Git or package discovery failed, or stdout could not be written (`error: cannot write output`) |
| `2` | Invalid usage, such as conflicting options |

A command that fails prints one `error: <message>` line on stderr, with any
control characters in it shown as `\u{…}` escapes:

```text
$ hug symbols 'missing/**'
error: No supported source files found in `/home/me/project`
$ hug god-files nope-dir
error: Failed to read `nope-dir`: god-files scan directory does not exist
```

[Exit status](./symbol-reports.md#exit-status) in Symbol Reports gives the
exact rules for the declaration commands.

## Related pages

- [Symbol Reports](./symbol-reports.md): the declaration commands' full contract
- [Scope-Aware References](./references.md): occurrences, confidence and `hug refs`
- [Symbol Kinds, Visibility and `static`](./symbol-kinds-and-visibility.md): per-language kinds and visibility
- [Symbol Records](./symbol-records.md): the JSON record fields
- [Cache Design](./cache-design.md): the persistent symbol cache
