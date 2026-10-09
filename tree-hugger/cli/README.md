# Tree Hugger CLI

Tree Hugger provides the `hug` CLI for exploring symbols, imports, and diagnostics across multiple
languages.

## Installation

```bash
cargo install --path tree-hugger/cli
```

## Usage

```bash
hug symbols
hug functions --language rust src/main.rs greet
hug imports --json "tests/fixtures/**/*.js"
```

## Commands

- `functions` - List function, method and constructor definitions
- `types` - List type definitions (structs, unions, enums, classes, interfaces, traits, type aliases)
- `symbols` - List all discovered symbols
- `imports` - List imported symbols
- `classes` - List classes, structs, interfaces and traits with members partitioned by static/instance
- `refs` - List the references to one declaration within its file, or its callers or callees (see [Refs Command](#refs-command))
- `lint` - Run lint and syntax diagnostics

## Options

- `--language <LANG>` - Override language detection (`js`/`javascript`, `ts`/`typescript`, `cpp`/`c++`, `c-sharp`/`c#`)
- `--exclude-files <GLOB>` - Exclude files matching glob patterns (in addition to the defaults below)
- `--exclude-symbols <GLOB>` - Exclude symbol names matching glob patterns
- `--json` - Output as JSON
- `--json-full` - Output every scanned file's complete index as JSON, diagnostics included (`functions`, `types`, `symbols` and `classes` only; symbol selectors such as name filters, `--exported`, `--prelude` or `--static-only` are rejected)
- `--plain` - Disable colors and hyperlinks
- `--verbose` - Print the cache stats line (persistent-cache hits, partial hits and misses, plus files parsed, definition captures and diagnostics runs) and any recoverable cache failure to stderr
- `--no-cache` - Neither read nor write the persistent symbol cache
- `--group-by-file` - Group symbol output by file
- `--group-by-module` - Group symbol output by module (the file's directory)
- `--sort file|name|kind` - Order of the listed declarations (`functions`, `types`, `symbols` and `classes` only). `file` (the default) lists by file, then source position; `name` by the case-sensitive name; `kind` by kind (modules and types, then callables, then values). Ties fall back to file, position, kind and ID. Give it once; `lint` and `imports` reject it
- `--jsonl` - One JSON object per selected declaration (JSON Lines); a `classes` line adds the container's `members` list
- `--count` - Print only the number of selected declarations (`classes`: containers)
- `--stats` - Counts by kind, visibility and language plus doc coverage, with overall totals and automatic package sections; with `--json`, an additive stats envelope including `package_root_dir` and `groups`. Captures structure-only package metadata first; a discovery failure exits `1` before output. See [Symbol Reports](../docs/symbol-reports.md#--stats).
- `--format plain|table|tsv|markdown|prose|ctags` - Select a terminal listing or a UTF-8 export; ctags writes unsorted format-2 numeric-line tags with Hug kind/container fields (see [Symbol Reports](../docs/symbol-reports.md#tags-output))
- `--signatures full|short|none` - The declaration header as written, the compact form (default; field lists past four end in `… +N`), or names only
- `--tree` - Nest members under their container (`functions`, `types`, `symbols`); type rows end with their declared bases (`extends Base; implements Shape; unclassified bases External`)

For exports, use TSV for eight fixed columns, Markdown for a literal list,
Prose for styled markup with local source links, or ctags for navigation.
See [Text exports](../docs/symbol-reports.md#text-exports) for escaping and
option combinations. Markdown and Prose keep the listing's layout: package
sections, `--group-by-file`/`--group-by-module` groups, `--tree` nesting
and the `classes` method and field sections.

### Selectors

`functions`, `types`, `symbols` and `classes` share these record selectors.
They combine with AND; the values of a repeated selector are alternatives.
Names use smart case: a pattern without an uppercase letter ignores case.

- `--file <PATH|GLOB>` / `--name <PATTERN>` (`-n`) - Say whether a filter is a file or a name instead of letting `hug` guess from a positional token; `--exact` matches names whole and literally
- `--kind <KIND>` - Only these kinds (snake_case: `function`, `type_alias`, `enum_variant`, …); narrows the command's kinds, never widens them
- `--scope item|member|local`, `--all-scopes` - Only these scopes; `symbols` hides locals (`item` and `member` by default), the other commands show every scope
- `--visibility <LEVEL>` - Exactly this visibility (`public`, `crate`, `private`, …, `unknown`)
- `--exported` - Only exported declarations
- `--in <PATTERN>` - Only members of a matching container (`classes`: the container itself)
- `--exclude-tests`, `--tests-only` - Drop, or keep only, test code and helpers in test files
- `--undocumented`, `--deprecated` - No doc comment / marked deprecated
- `--async`, `--returns <GLOB>`, `--param-type <GLOB>`, `--min-params <N>` - Signature shape; keeps only functions, methods and constructors

```bash
hug symbols --kind trait --exported            # exported traits
hug functions --in Widget --undocumented        # undocumented Widget members
hug functions --async --returns 'Result<*'      # async functions returning a Result
hug symbols --all-scopes --name '*/lib.rs#*'    # everything in any lib.rs, locals included
```

Every selector, with examples, is in `docs/symbol-reports.md`.

Exactly one output mode is active (terminal, `--json`, `--json-full`,
`--jsonl`, `--count`, `--stats` or `--format`); `--stats` alone composes,
with `--json` or a `--format`. Interactive output wraps at the terminal width
(a positive `COLUMNS` overrides it); `--plain`, `--format plain` and
redirected output never wrap. See `docs/symbol-reports.md`.

The old `--sort-by-kind` and `--sort-by-module` flags are gone: `--sort kind`
replaces the first, and the default `--sort file` (portable path, then source
position) covers the second.

## Package Stats

```bash
hug symbols --stats
hug functions --stats --json
hug classes --stats --format plain
```

Stats automatically shows **All selected declarations**, then package sections
labeled `name (path)`, ordered by path and name, with **Unassigned** last.
File and declaration selectors keep their usual meanings; `classes` counts
containers. The deepest discovered package owns each selected file, but only
a manifest-backed name identifies a named group. Unnamed owners and files
outside the discovery root use Unassigned. Packages without selected files
are absent; selected files with no matching declarations or failed analysis
retain a zero group with undefined coverage (`n/a`, or JSON `null`).

JSON keeps overall `stats` and adds `groups`, whose entries have the same stats
shape and `package: {"name", "path"}`, or `package: null` for Unassigned.
Summing the groups' category counts, coverage counts and synthetic-export
counts reproduces the overall fields. Overall coverage uses the overall denominator, rather
than averaging group ratios. Synthetic exports count under the exporting
prelude's package and contribute to neither known nor unavailable doc coverage.

`package.path` uses `/` separators relative to `package_root_dir`, with `.`
for a root package. Discovery starts at the active Git worktree root, or the
scan root outside Git; `root_dir` remains the scan root. For example, scanning
`crates/utils` inside `/work/repo` keeps that scan root while reporting the
group path `crates/utils` relative to `/work/repo`. The discovery-root field is
present even for an empty successful `--changed` report with `groups: []`;
ordinary empty scans still fail.

Discovery failures exit `1` with repair guidance on stderr and no report.
After discovery, file failures retain partial totals, appear once in the
top-level `errors`, and exit `1`. No discovered packages is a successful
Unassigned result. Manifest names display literally; `--format plain` emits
no terminal escapes or hyperlinks. Listings, count, JSON Lines and full-index
JSON do not acquire a package catalog for grouping.

These additive fields retain `report_schema_version: 1` because existing
fields keep their meanings. Terminal output intentionally gains headings and
sections; see the [before/after release example](../docs/release-notes.md#automatic-package-sections-in-stats)
and [full report contract](../docs/symbol-reports.md#--stats).

## Default Scan Exclusions

A whole-package scan (no path/glob argument) honors `.gitignore` and additionally
skips test *data* directories, whose contents are inputs rather than compiled
source: `**/fixtures/**`, `**/__fixtures__/**`, `**/snapshots/**`, `**/testdata/**`.
Naming such a file explicitly (e.g. `hug lint tests/fixtures/sample.rs`) overrides
the exclusion. Normal test code (`tests/*.rs`) is still scanned.

For `lint`, a multi-file scan only prints files that have diagnostics; if every
file is clean it prints a single `No lint diagnostics in N files.` line. A
single-file lint still prints `(no diagnostics)` as explicit confirmation.
"Multi-file" counts the selected files, so a run whose other files failed to
read still hides a clean file. Text `lint` and `imports` print each file as it
is analyzed, so `hug imports | head` stops the scan early.

## Output Format

### Tags Output

`--format ctags` writes an unsorted format-2 tags file with numeric line
addresses. Duplicate names and overloads are retained; paths are relative to
the scan root. Kind names and container fields use recorded Hug metadata.
Values are written byte for byte, backslashes included. Records Neovim would
not read back as written (control characters, `!_` pseudo-tag names, file
names it would expand, and paths whose route from the scan root holds a name
that is not valid Unicode, such as a Linux `bad\xff.java`) are skipped with a
warning and a failing exit status; a scan root with such a name, or a file
really named with `�`, is tagged normally. See [Tags output](../docs/symbol-reports.md#tags-output)
for the supported subset.

### Pretty Output (default)

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

Declarations are listed by file, then source position; `--sort name` and
`--sort kind` change that, also within `--group-by-file` and
`--group-by-module` groups. The word before the name is the declaring keyword
for types, variables, fields and constants (`struct`, `let`, `const`, `my`,
`local`, ...), read from the record, and the kind otherwise.

The listing is printed as files are analyzed, so `hug symbols | head` stops
the scan after a window or two (four files per worker thread). Flat `--sort name` and `--sort kind` listings
are the exception, as is `--prelude`: each package is printed once it is fully
analyzed. When the scanned files span two or more packages, the listing has
one section per package (`(no symbols)` for a package without declarations);
that is decided from the files before any is analyzed, so it never delays
output.

#### Visibility labels

The label is the access declared or implied at the declaration:
`public`, `protected`, `private`, `internal`, `package`, `crate`,
`module-private`, `protected internal` and `private protected`. When the
level is unknown (parameters and locals, Python/Perl/shell code, a Rust
`impl` of a trait declared in another file such as `fmt` above) no label is
printed rather than a guess. The rules per language are in
[Symbol Kinds, Visibility and `static`](../docs/symbol-kinds-and-visibility.md).

#### Containers

A member ends with `[in <kind> <name>]` naming its container, or
`[in <name>]` when the container's kind is not known from the file. Members of
a Rust `impl` block are grouped under the implemented type, so a method in
`impl Point` or `impl Display for Point` prints `[in type Point]`, never
`[in impl Point]`.

### Changed files

Use `--changed` on `functions`, `types`, `symbols`, or `classes` to report
whole changed files' current on-disk declarations. Git-reported file-type
replacements count too; an explicit `--file leaf.rs` can select a symlink to
source in the same checkout, while default walks retain symlink exclusions:

```bash
hug functions --changed                 # staged, unstaged, nonignored untracked files
hug symbols --changed=main              # also compare the local main tip directly to HEAD
hug symbols --changed --file 'src/**'    # intersect with the normal file selection
hug --json-full symbols --changed       # complete indexes of selected changed files
hug functions --changed --count         # 0 when no eligible changed files remain
```

Bare `--changed` uses `HEAD` and selects uncommitted changes only. A reference
requires `=`; `--changed main` leaves `main` as a positional filter. Reference
comparison uses local commit tips, without fetching or finding a common
ancestor. Staged content and historical source are never reported.

Package scope, language rules, ignores, fixture exclusions, and file and
name selectors still apply. Valid empty selections exit `0`: text shows
`(no symbols)` (the `classes` terminal/plain outline emits no text), JSON
Lines emits no lines, count prints `0`, stats have zero totals, and JSON has
empty `files`/`indexes` with `errors: []`.

Changed aliases must stay in the active checkout: targets in another checkout,
nested repository, or submodule are excluded, even with an explicit alias or
target selector. Directory and file aliases cannot cross those boundaries
and return eligibility to an outer source file, including targets such as
`inner/../clean.rs` when `inner` is a nested repository. Ordinary directory
parents remain valid: `ordinary/../clean.rs` can select `clean.rs` when
`ordinary` belongs to the active checkout. Parents after directory aliases
follow the alias destination, matching filesystem resolution.

Git failures and conflicts anywhere in the repository exit `1` before any
report. A truncated index produces a repository/index error on stderr and empty
stdout; an absent or valid empty index remains allowed. `--changed` conflicts with `--prelude` (exit `2`) and is unavailable
on non-declaration commands. See [Changed files](../docs/symbol-reports.md#changed-files)
for selection precedence, unborn repositories, and the selection flow.

### JSON Output

With `--json`, `functions`, `types`, `symbols` and `classes` print one report:

```json
{
  "report_schema_version": 1,
  "schema_version": { "major": 5, "minor": 0 },
  "root_dir": "/path/to/project",
  "files": [
    {
      "file": "src/rust.rs",
      "language": "Rust",
      "completed_passes": ["Parse"],
      "completed_capabilities": ["declarations"],
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

(Records are shown abridged.)

- `symbols` holds only the command's records, as schema 5.1 `SymbolRecord`s,
  ordered by `--sort` within each file. File entries are always in portable
  path order, and a file with no match has no entry; an empty selection is
  `files: []`.
- `classes` entries add `members`: each container's displayed methods and
  constructors, keyed by the container's ID (`--static-only` and
  `--instance-only` apply). Fields stay in the container's `kind_data`.
- Records carry their declared facts on every JSON format: generic
  parameters and constraints (`type_info`), `modifiers`, `attributes` and the
  written bases (`relations.extends`/`implements`/`unclassified_bases`). The `metrics` facet,
  including a callable's body size `metrics.lines` and cyclomatic complexity
  `metrics.cyclomatic`, is in `--json-full`, and in `--json`/`--jsonl` only
  when `--undocumented` or `--min-params` asks for metrics.
- Records carry the declaration header (`source.signature_text`) but not the
  declaration's source text, attached comments or work the command did not
  ask for (references, semantics); `completed_passes` and
  `completed_capabilities` say what was computed. `--json-full` has the same
  envelope with complete per-file `indexes` (every kind, every pass,
  diagnostics and `declaration_text`) in place of `files`.
- Every path is `/`-separated and relative to `root_dir`, including a file
  outside it (`../vendor/lib.rs`); record IDs are unchanged by that. A file on
  another Windows drive keeps its absolute path (`D:/vendor/lib.rs`).
- `errors` is always present and always last. Each failed file is one entry
  (`file`, `kind` — the error variant in snake case such as `io` — and
  `message`), is also written to stderr, and makes `hug` exit `1`.
- Records carry no timestamps, so two runs over the same files print the same
  bytes, with or without the cache and whatever the number of workers.

`imports` and `lint` keep their own JSON shape: `files` (per-file summaries
of imports or diagnostics), `symbol_indexes` and `adapter_metadata`.

[Symbol Reports](../docs/symbol-reports.md) covers the declaration commands
in depth: what each command analyzes, the flow from query to output, ordering
and grouping, streaming, failed files and exit status, and `--verbose`.

## Symbol Kinds

The CLI colors each kind:

| Kind | Display | Color |
|------|---------|-------|
| Function, Method, Constructor | `function`, `method`, `constructor` | Green |
| Type, TypeAlias, Class, Interface | `type`, `type alias`, `class`, `interface` | Magenta |
| Enum, EnumVariant | `enum`, `enum variant` | Cyan |
| Field, Property | `field`, `property` | Cyan |
| Trait | `trait` | Yellow |
| Module, Namespace | `module`, `namespace` | Yellow |
| Variable, Parameter | `variable`, `parameter` | Blue |
| Constant | `constant` | Bright blue |
| Macro | `macro` | Red |

Some lines show the declaring keyword from the source instead of the kind
name (`struct`, `union`, `const`, `static`, `let`).

## Examples

```bash
# List all types in the nearest package (default all-file scan)
hug types

# List functions in TypeScript files, excluding tests
hug functions "src/**/*.ts" --exclude-files "**/test/**"

# List symbols while excluding generated helper names
hug symbols "src/**/*.rs" --exclude-symbols "*_generated*"

# Export all symbols as JSON for tooling
hug symbols "lib/**/*.rs" --json > symbols.json

# Check imports in JavaScript files
hug imports "src/**/*.js" --plain

# List classes with their members
hug classes "src/**/*.ts"

# Filter classes by name
hug classes "src/**/*.java" --name Controller

# Show only static members
hug classes "src/**/*.ts" --static-only

# Run lint diagnostics
hug lint "src/**/*.rs"

# Run only lint rules (skip syntax errors)
hug lint "src/**/*.rs" --lint-only

# Run only syntax diagnostics
hug lint "src/**/*.rs" --syntax-only

# List registered lint rules
hug lint --list-rules
```

## Classes Command

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

Options:
- `--name <FILTER>` - Filter classes by name (substring match)
- `--static-only` - Show only static members
- `--instance-only` - Show only instance members

## Refs Command

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

The full contract is in [`docs/references.md`](../docs/references.md#hug-refs).

## Lint Command

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

[semantic] error [undefined-symbol]: Reference to undefined symbol `foo`
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

`lint` exits `1` when an error-severity diagnostic remains or a file could not
be analyzed. Like the declaration commands, it ends quietly with `0` when its reader closes
early (`hug lint 'src/**' | head`); `imports` does the same.

`hug refs --id` matches equivalent Rust raw/NFC and TypeScript/JavaScript
escaped identifier spellings while displaying the token as written. Imported
string export names decode as one name; file-local resolution still supplies
no external declaration ID. See [references](../docs/references.md).
