# `icon` CLI

The `icon` binary (`biscuit-icon-cli`) is a small command-line front end
for the library. It lists icon sets, lists / renders icons matching a
filter, generates dynamic shell completions, and clears the local cache.

## Synopsis

```bash
icon [GLOBAL FLAGS] [show [ID…] [--from <csv>] [--svg|--code-block|--css] [--meta] [--list] [--pick]]
icon [GLOBAL FLAGS] sets [FILTER]
icon [GLOBAL FLAGS] domain [ARG]
icon [GLOBAL FLAGS] cache list
icon [GLOBAL FLAGS] cache clear [FILTER]
icon [GLOBAL FLAGS] completions <shell>
```

When no subcommand is given, the CLI runs the default `show` command, so:

```bash
icon mdi:home           # ≡ icon show mdi:home
icon home               # ≡ icon show home
icon --from mdi home    # ≡ icon show --from mdi home
```

## Subcommands

### `show [ID…] [--from <csv>] [--svg|--code-block|--css] [--meta] [--list] [--pick]` (default)

Show icons by explicit id, filter, or interactive picker.

- **One or more `prefix:name` ids** → direct lookup-and-render for each id.
  `--from` is still applied; supplying a `prefix` that is not in `--from`
  is an error. Multiple ids are rendered one per line.
- **Single substring without `:`** → inexact match: list offline matches,
  then paginate the online `/search` results (up to 100 hits) and merge,
  deduped, with the offline results. Each merged hit is fetched and cached.
  In a TTY with exactly one match, the match is auto-rendered; with two
  or more, the interactive picker is launched (unless `--list` forces a
  text list). In non-TTY, all matches are listed one per line.
- **No ids** → list every offline icon (built-in domain catalog + the local
  SQLite cache). The empty-query online search is skipped.

#### Format flags (mutually exclusive)

| Flag | Effect |
|------|--------|
| `--svg` | Emit raw `icon.svg()` text. |
| `--code-block` | Emit the SVG wrapped in a Markdown fenced code block, rendered through `darkmatter` with terminal syntax highlighting. |
| `--css` | Emit the icon as a CSS `url('data:image/svg+xml,…')` string (percent-encoded). |
| (none) | Terminal ladder render (Nerd Font → Unicode → image → identifier text). |

More than one format flag is an error.

`--meta` forces a metadata table regardless of arg count: columns `Set`,
`Icon`, `Categories`, `Tags`, plus `Author` and `License` when populated.

`--list` forces a plain text list (one match per line) even in a TTY.
`--pick` forces the interactive picker; errors in non-TTY.

`--from <csv>` restricts the result set to a comma-separated list of
prefixes. It applies to both offline listing and online search (the
client passes `?prefix=` for one prefix, `?prefixes=` for several).

Global flags and positional ids are all covered by dynamic shell
completions, so `<TAB>` after `icon --from m` offers the matching
prefixes (see [Completion behavior](#completion-behavior) below).

### `sets [FILTER]`

List every Iconify set in a striped four-column table. Output is rendered
via `biscuit-terminal` `Table` (or `TwoColumn` when rows are tall and the
terminal is wide enough):

| Column  | Content | Alignment | Notes |
|---------|---------|-----------|-------|
| `Set`     | Human-readable collection title | Left  | Wraps at 30 columns |
| `Prefix`  | Iconify collection prefix       | Left  | Wraps at 20 columns |
| `Total`   | Upstream icon count              | Right | Right-aligned; shows `Unknown` when the API did not report one |
| `Cached`  | Number of matching rows in SQLite | Right | Right-aligned with thousands separators |

The `Total` column shows `Unknown` (not `0`) when Iconify's `/collections`
response omits the `total` field. Known counts use
`ColumnType::Integer` so the table component formats thousands
separators locale-independently.

When all rows fit within the terminal's height budget, the output is a
single table. Otherwise the output is split column-major into two
balanced tables only when the terminal is at least `MIN_SPLIT_WIDTH = 99`
columns wide (the layout constants live in `cli/src/sets_table.rs`).

`filter` is a substring matched case-insensitively against the prefix and
the title. The full online catalog is fetched (and cached) on every
invocation; if the network is unreachable and there are no offline rows
for the filter, the command errors.

### `domain [ARG]`

Curated domain icons (offline-only, no network).

- **No arg** → table of the 16 curated enum-set names with variant counts.
- **Enum name** (e.g. `emoji`) → list variants with columns `Variant`,
  `Glyph`, `Iconify ID`.
- **`enum:variant`** (e.g. `emoji:happy`) → render the single curated icon
  infallibly when it matches.
- **Substring** → filter the 16 set names; error when no match.

### `cache list`

List cached icons with metadata. Columns: `Set`, `Icon`, `Display`
(omitted when the terminal cannot render visuals), `Categories`, `Tags`.

### `cache clear [FILTER]`

Drop cached icons. Without a filter, both `icons` and `sets` tables are
wiped. With a filter, only `icons` rows whose `prefix:name` contains the
filter (case-insensitive) are deleted; `sets` is left intact. The cache
file itself is preserved (the schema is kept intact). There is no TTL —
entries persist until you clear them.

### `completions <shell>`

Generate a static completion script for `<shell>` (one of `bash`, `zsh`,
`fish`, `elvish`, `powershell`). This is the *static* entry point, which
is what `icon completions bash > ~/.local/share/bash-completion/completions/icon`
gives you. Completion of dynamic values (set prefixes and icon names) is
handled at runtime — see below.

## Global Flags

| Flag | Effect |
|------|--------|
| `-v`, `--verbose` (repeatable) | Increases user-facing diagnostic detail. With one or more, deduplicated `Caused by:` lines are appended to the error report. Does not affect `tracing`. |
| `-d`, `--debug` (repeatable) | Enables `tracing` output on stderr. `-d` → `warn,biscuit_icon=info,icon=info`; `-dd` → `info,biscuit_icon=debug,icon=debug`; `-ddd` → `debug,biscuit_icon=trace,icon=trace`. |
| `--nerd` | Prefer Nerd Font glyphs when the icon defines one. Equivalent to setting `ICON_NERD_FONT=1`. |

## Environment Variables

| Variable | Effect |
|----------|--------|
| `ICON_NERD_FONT=1` | Equivalent to `--nerd` (consumed by `clap`'s `env` attribute). |
| `ICONIFY_BASE_URL` | Override the Iconify API base URL (consumed by the CLI's `client_from_env`; the library API is unaffected). Useful for self-hosted instances (see [iconify.md § Self-hosting](iconify.md#self-hosting)). |
| `BISCUIT_TERM_WIDTH`, `BISCUIT_TERM_HEIGHT` | Override the detected terminal dimensions for the `sets` table layout (set both for a reproducible render in tests). |
| `RUST_LOG` | When set, takes precedence over `--debug`; `tracing-subscriber` parses it directly. |

## Dynamic Completion Behavior

`icon` uses `clap_complete` "unstable-dynamic" to provide value
completion at runtime, in addition to the static `completions <shell>`
script. Two completion providers:

- **`icon_name_completer`** — for the `FILTER` positional and the
  `--from`-prefixed list: returns `prefix:name` candidates that contain
  the current token (case-insensitive). The first 100 results are
  returned, merged from the built-in domain catalog and the local
  SQLite cache.
- **`set_name_completer`** — for `--from <csv>` and the `sets` filter:
  returns up to 50 prefix candidates, merged from the built-in
  domain catalog, the cached `sets` table, and the cached `icons`
  prefixes. When the current value contains commas (e.g.
  `--from mdi,ic`), only the active segment after the last comma is
  completed; the already-entered prefix is preserved in each candidate.

The shell invokes the binary once per token with a known completion
environment variable set; the response is parsed and offered as
completions. Completion must never fail the shell — the providers
silently degrade to "no completions" on any cache error.

## Error Reporting

`color-eyre` is installed at the top of `main`, and `render_error`
formats the report through `biscuit_terminal::components::prose::Prose`:

```
<red><b>Error:</b></red> <message>
```

With `--verbose`, deduplicated `Caused by:` lines follow on indented
`<dim>` lines. The CLI exits with status `1` on any error.

The rendered icon for the matched id follows the same ladder as
`Icon::render` (Nerd Font → Unicode → image → identifier). For
network hits, the body is fetched on demand and persisted; the next
invocation finds it in the cache and never touches the network.

## Examples

```bash
# Render a single Iconify icon by its identifier
icon mdi:home

# Search the catalog (offline + online) and render each result
icon happy

# Limit the search to a few sets
icon --from mdi,lucide home

# Raw SVG output
icon show mdi:home --svg

# Metadata table
icon show mdi:home --meta

# List every curated set with a per-set icon count
icon sets

# Find a set by title
icon sets lucide

# Use Nerd Font for icons that have a glyph
icon --nerd --from mdi devops

# Curated domain sets
icon domain
icon domain emoji
icon domain emoji:happy

# Cache maintenance
icon cache list
icon cache clear
icon cache clear home

# Install static bash completions
icon completions bash > ~/.local/share/bash-completion/completions/icon
```

## Building / Installing

From the package area root:

```bash
just build       # debug build
just install     # build --release and install `icon` to ~/.cargo/bin
just cli -- mdi:home   # run from source (cargo run -p biscuit-icon-cli)
```
