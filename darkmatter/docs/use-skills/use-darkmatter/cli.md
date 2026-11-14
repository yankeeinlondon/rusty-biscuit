# The `md` CLI

`md` is the Darkmatter command-line tool. Running `md FILE` is the same as
`md render FILE`. Every command takes the same `--output` and `--width`
options and uses the same exit statuses.

## Install

```sh
just -f darkmatter/justfile install      # from the rusty-biscuit repo root
cargo install --path darkmatter/cli      # equivalent
md --version
```

Shell completions complete subcommands, flags, file names, **and frontmatter
property paths and values from the document's schema**:

```bash
source <(COMPLETE=zsh md)    # ~/.zshrc   (bash: COMPLETE=bash; fish: COMPLETE=fish md | source)
```

### Shell completions

On Windows PowerShell, `md` is a built-in alias for `mkdir`. This line
removes the alias and registers completion:
`Remove-Item Alias:md -Force -ErrorAction SilentlyContinue; $env:COMPLETE = "powershell"; md | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE`

## Commands

| Command | What it does | Default `--output` |
|---|---|---|
| `md render FILE` (or `md FILE`) | render for reading | `terminal` (`plain` when piped) |
| `md compose FILE [k=v…]` | run the compose pipeline | `markdown` (always) |
| `md clean FILE [--save]` | reformat without changing meaning | `markdown` |
| `md get FILE PATH` | read one frontmatter value | `json` |
| `md set FILE PATH VALUE [--save]` | set one frontmatter value | `markdown` |
| `md rm FILE PATH… [--save]` | remove frontmatter values | `terminal` |
| `md toc FILE` | table of contents | `terminal` |
| `md delta A B [-v]` | semantic diff of two documents | `terminal` |
| `md hash FILE\|DIR` | Markdown-aware hashes; `--save` stores them, `--diff` reports drift | `terminal` |
| `md graph FILE [--validate]` | dependency (transclusion/reference) graph | `terminal` |
| `md validate refs FILE` | broken links, images, transclusions, CSS imports | `terminal` |
| `md schema validate FILE…` | frontmatter (or a whole YAML file) against its schema | `terminal` |
| `md schema detect FILE…` | infer a schema from documents (`--output json-schema` too) | `terminal` |
| `md schema triggers FILE` | which schema triggers apply and why | `terminal` |
| `md schema about` | SimplifiedSchema language reference | `terminal` |
| `md edit FILE` | open in `$EDITOR` and wait (creates a missing file) | `terminal` |
| `md code-block INPUT` | render one code block (`--file`/`--content`) | `terminal` |
| `md list-themes` | theme names for `style.page.theme` / `style.page.code.theme` | `terminal` |

These commands read stdin when no file is given (or the file is `-`):
`render`, `compose`, `clean`, `toc`, `get`, `set`, and `hash`. Example:
`md compose doc.md | md`.

## Output, streams, and exit status

`--output` takes a **formatted** target (`terminal`, `plain`, `markdown`,
`markdown-plus`, `html`) or a **data** target (`json`, `json5`, `yaml`,
`toml`). `md compose` accepts only `markdown` and the data targets.

| Family | stdout | stderr |
|---|---|---|
| data | exactly one value, whether the run succeeded or failed | nothing (warnings are in `diagnostics`) |
| formatted | the deliverable | warnings, problem reports, `-v`, `--perf`, lifecycle output |

The reports from `validate refs`, `graph --validate`, `schema validate`, and
`hash --diff` are the deliverable, so they go to stdout.

| Exit | Meaning |
|---|---|
| 0 | ran and succeeded |
| 1 | ran and failed (a broken reference, an invalid schema, a `--deny` that blocked, `hash --diff` found drift, …); a data output still prints its value |
| 2 | usage error: nothing ran and no value was printed; the parser's message is on stderr |
| 130 | Ctrl-C |

```sh
if out=$(md compose doc.md --output json); then
  jq -r .document <<<"$out"
elif [ $? -eq 2 ]; then
  echo "bad command line" >&2
else
  jq -r '.diagnostics[] | "\(.code) \(.source.file):\(.source.line) \(.message)"' <<<"$out"
fi
```

### The compose/render envelope

```json
{
  "file": "doc.md", "outcome": "success|failure",
  "document": "…composed markdown… or null",
  "diagnostics": [ { "code": "dm.reference.missing_hyperlink", "severity": "error",
                     "message": "…", "source": { "file": "…", "line": 3, "column": 5,
                     "section": "slug", "inserted": false }, "span": {"start":14,"end":39},
                     "reason": "…", "construct": "hyperlink", "reference": "./x.md",
                     "failure": "no-match", "included_via": [] } ],
  "summary": { "references": 3, "documents": 1, "ok": 3, "broken": 0 },
  "lifecycle_output": [], "read_set": [], "perf": { … only with --perf … }
}
```

- `code` is a stable `dm.<family>.<kind>` identifier. DMLS (the editor
  language server) uses the same codes. Match on `code`, never on `message`.
- `severity` describes the problem and `outcome` describes the run. A
  problem recovered with `--allow` keeps `severity: "error"` while the run
  succeeds.
- `source.line` and `source.column` are 1-based positions in the **authored**
  file, including transcluded children. Either can be `null` when no position
  is known, for example for a setter value given on the command line.
- `md get` and `md schema detect` use a bare shape,
  `{ outcome, diagnostics, value }`.
- Each command's JSON Schema is in `darkmatter/cli/schemas/output/<cmd>.yaml`.

### Width

`--width N | N% | auto | source | unlimited`. `render` wraps to the terminal
by default. `compose` **never wraps** unless you ask: by default it joins a
paragraph's soft line breaks. `source` keeps the authored line breaks.
Code blocks and tables are never reflowed. `OUTPUT_COLUMNS` sets the width
that `auto` uses.

## File references

The way a reference is written decides which directories are searched. The
first existing file wins, and a miss never falls back to a different form.

| Written | Searches |
|---|---|
| `./x.md`, `../x.md` | the base directory only |
| `x.md`, `docs/x.md` | the base directory, then the repository root |
| `/abs/x.md`, `C:\x.md` | that path |
| `~/x.md` | home only |
| `@x.md` | magic roots: the local tree (package, area, repo root), then home and registered roots; add more with `md --magic-root DIR <cmd>` |
| `&x.md` | repository root only (**quote it**: `'&x.md'`) |
| `^x.md` | package, then package area, then repo root (never home) |
| `vault:x.md` | registered vault roots, then `$VAULT` |
| `https://…` | remote; for compose, the host needs `--allow host=HOST` |

- `%` before any form (`%@README.md`) searches recursively.
- `{{{VAR}}}` in a path segment reads an environment variable
  (`@configs/{{{APP}}}/x.toml`).
- The base directory of a reference typed on the command line is where you
  ran `md`. Inside a document, it is that document's folder.
- A relative reference cannot leave its repository. `&` and `^` fail outside
  a repository.
- A bare `@`, `&`, or `^` is a parse error. To name a file that starts with one
  of these characters, write `./` first (`./@`).
- A failed reference prints a `failure:` row with one of `invalid-reference`,
  `missing-context`, `no-match`, `io`, or `unsupported-remote`.

## Property paths (`get`, `set`, `rm`, compose setters, render style setters)

Keys are joined with `.`. Escape a literal dot as `\.` and a backslash as
`\\`. A number is an array index when the value at that point is an array,
and a key when it is a mapping.

```sh
md get doc.md style.page.max-width
md get doc.md items.1.name           # second element
md get doc.md 'a\.b'                 # the key "a.b"
md set doc.md author.name Ann --save # creates missing parents; keeps siblings
md set doc.md items '[1, 2, 3]'      # replace a whole array (paths never append)
md rm  doc.md items.0 --save
```

`get` returns `""` for a missing path, with exit status 0. `set` and `rm`
refuse a path they cannot follow (exit 1) and write nothing. Examples: an
index past the end of an array, a number under a missing parent, or a path
that goes through a scalar value.

### Command-line values (`set`, compose setters, style setters)

1. The text is first read as JSON5. `{a: 1}` and `["x","y"]` become objects
   and arrays. `"5"` is the string `5`. `null` is null. Any other text is
   plain text, so `[x, y]` stays text; write `'["x","y"]'` for an array.
2. Plain text is then converted to the type the schema declares at that path.
   For example, `1.10` stays the text `"1.10"` under `string` or `unknown`, but
   becomes the number `1.1` under `number`. With **no** declared type, `md`
   guesses: `5` → number, `true` → boolean, `02134` → string. Quote a value to
   force a string: `'"5"'`.

## `md compose` essentials

```sh
md compose doc.md --state '{name: "Alice"}'  # defaults: fill missing/null keys only
md compose doc.md --set '{"name":"Bob"}'     # override top-level keys (replaced whole)
md compose doc.md author.name=Charlie        # setter: one path; highest priority
md compose doc.md --fm                       # keep frontmatter in the output
md compose doc.md --width 80                 # reflow prose
md compose doc.md --timeout 30               # shell timeout in seconds (default 10)
md compose doc.md --perf                     # stage timings (stderr, or "perf" in data)
md compose doc.md --show                     # also open the result in the default app
```

Priority, lowest to highest: the document's frontmatter, then `--state`, then
`--set`, then `path=value` setters. Setters also reach transcluded children,
and they can change which schema trigger matches.

Before composing, `md compose FILE` checks references automatically. If any
reference is broken, it prints the reference report and exits 1, unless an
`--allow` recovery covers the problem.

### Policy verbs

| Verb | Use |
|---|---|
| `--allow <recovery\|all>` | recover past a known problem: `missing-hyperlinks`, `missing-image-refs`, `missing-transclusions`, `shell-timeout`, `ctx-override`, `invalid-frontmatter-assignment`, `reassigned-frontmatter-property`, `untyped-map-variables`, `untyped-filter-variables` |
| `--allow host=HOST` | allow network reads from a hostname, `*.wildcard`, or IP. An IP or CIDR also grants `ping` probes. `all` never grants a host |
| `--deny <id\|code\|all>` | make diagnostics blocking, e.g. `--deny all`, `--deny dm.expression.undeclared_property`, `--deny style` (render) |
| `--disable <feature>` | `baseline-schema`, `schema-triggers`, `collection-blocks` (compose only) |

The usual CI invocation is `md compose doc.md --deny all --output json`.
Some problems have no recovery and always fail: a call that can be proven
wrong, a parse error, a missing CSS/script/font import.

### Shell commands

Commands come from body `::shell` lines, `::shell-block` blocks,
frontmatter `$(…)` values, and lifecycle `shell` actions. They are all
collected **before** anything runs, including the commands in branches whose
conditions are false, and approved in one batch.

- A built-in blacklist always refuses some commands: `rm`, `mv`, `cp`, `curl`,
  `ssh`, `sudo`, `git push`/`reset`/`checkout`, package installs, output
  redirection (`>`), and others.
- Approvals are stored next to the document, in
  `.darkmatter-shell-whitelist` and `.darkmatter-shell-blacklist`. A rule can
  be an exact command, a whole program, or a typed template such as
  `template git log origin/{{{ string }}}..HEAD`.
- An unknown command prompts on a terminal. In a data output or another
  non-interactive run, it is refused, and the run fails.
- `md compose doc.md --shell` lists every command and network probe without
  running anything. Use it to fill the whitelist before CI.

### Remote content

```sh
md compose doc.md --allow host=example.com                       # ::file https://example.com/x.md
md compose doc.md --allow host=example.com --cache-root .        # cache raw HTTP bodies across runs
md compose doc.md --remote-freshness strict|fallback|optimistic  # stale-cache behavior (default: fallback)
md compose doc.md --remote-refresh                               # revalidate every cached body
```

The cache stores only raw HTTP bodies. Composed results are never cached.

## `md render` styling

```sh
md doc.md style.max-width=72 style.page.theme=github   # per-run style setters
md doc.md --style ./print.yaml                         # JSON/JSON5/YAML/TOML style file (repeatable)
md doc.md --output html > doc.html
md doc.md --deny style                                 # fail on unknown/deprecated style keys
```

Put render options **after** `render` (or anywhere on a bare `md FILE`).
They are never allowed before another subcommand. `--width`, `-v`, and
`--debug` may go anywhere. `--magic-root` and `--completions` go before the
subcommand.
