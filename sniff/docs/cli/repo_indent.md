---
blast_radius:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/indentation.rs
  - sniff/lib/src/filesystem/indentation/mod.rs
---

# The `sniff repo indent` Subcommand

Tells you which indentation to use when you write a file of a given type in a
repository: spaces or tabs, how many columns per level, which configuration
decided that, and whether the repository's other configurations agree.

Use it before generating a YAML, TOML, JSON, or source file, so the output
matches what the repository's editors and formatters expect.

```text
sniff repo indent [<FILE-EXT>] [-b/--base <DIR>] [--json]
```

How the answer is chosen (EditorConfig first, then the file type's formatter
configuration, then a built-in language convention, then four spaces) is
described in [Indentation resolution](../topics/indentation.md). This page
covers the command: its arguments, its output, and its JSON.

## One file type

Pass the extension, with or without its leading dot (`yaml` and `.yaml` are
the same):

```text
$ sniff repo indent rs
4 spaces  (.editorconfig [*], [*.rs])
also: rustfmt.toml tab_spaces  4 spaces
consensus: yes
```

- **The first line** is the result and, in parentheses, the source that
  decided it. An EditorConfig source lists its matching sections; a formatter
  source lists the keys it read (`rustfmt.toml tab_spaces`), or `defaults`
  when the file sets no indentation and the formatter's own defaults apply
  (`biome.json defaults`). A language convention reads `gofmt convention`,
  and `default` is Sniff's four-space fallback.
- **When the style and the width come from different sources**, both are
  named. Here `.editorconfig` sets only `indent_style`, so the width comes
  from the next source:

  ```text
  2 spaces  (style: .editorconfig [*]; width: rustfmt.toml tab_spaces)
  ```

- **`also:` lines** list every other source that has an opinion, in rank
  order, with the values it states. In a terminal, a value that disagrees
  with the result is highlighted.
- **`consensus:`** is `yes` when every source agrees on each value it states.
  A missing consensus is not an error: the command still exits 0. It tells you
  that running one of the other formatters later may re-indent the file.

A disagreement looks like this, with a bare `biome.json` (Biome defaults to
tabs) next to an `.editorconfig` that asks for four spaces:

```text
$ sniff repo indent ts
4 spaces  (.editorconfig [*], [*.{js,ts,jsx,tsx,vue,json,yaml,yml,toml}])
also: biome.json defaults  tabs (width 2)
consensus: no
```

### Notes under the result

YAML does not allow tabs for indentation. When the sources ask for tabs for
a YAML file, the result uses spaces at the same width and says so:

```text
$ sniff repo indent yaml
2 spaces  (.editorconfig [*.yaml])
note: tabs requested; using spaces because YAML does not allow tabs for indentation
consensus: yes
```

A configuration file Sniff could not use (malformed, or a value of the wrong
type) never fails the command. It is listed after the result, with the key
when one is at fault, and the result comes from the remaining sources:

```text
$ sniff repo indent rs
4 spaces  (default)
consensus: yes
skipped configuration:
- rustfmt.toml: is not valid TOML: TOML parse error at line 1, column 14 …; skipped
```

## The whole repository

Without an extension, the command prints the repository **default** (only the
generic `[*]` EditorConfig sections, then the fallback), followed by every
file type whose result differs from it:

```text
$ sniff repo indent
default  4 spaces        (default)
dart     2 spaces        (dart format convention)
go       tabs (width 8)  (gofmt convention)
mk       tabs (width 8)  (make convention)
yaml     2 spaces        (.editorconfig [*.{yaml,yml}])
yml      2 spaces        (.editorconfig [*.{yaml,yml}])
```

That list is shortened, and comes from a repository whose `.editorconfig`
has only a `[*.{yaml,yml}]` section. A generic `[*]` section that sets both the style
and the width outranks every convention, so in a repository with one, the
list holds only the types its other sections or formatter files change:

```text
$ sniff repo indent
default  4 spaces  (.editorconfig [*])
yaml     2 spaces  (.editorconfig [*], [*.{yaml,yml}])
yml      2 spaces  (.editorconfig [*], [*.{yaml,yml}])
```

The file types checked are those named by the formatter table, the built-in
conventions, and the root `.editorconfig`'s literal suffix sections (`[*.rs]`,
`[*.{yaml,yml}]`). A wildcard or range inside a suffix names no file type; ask
for that type directly. A row whose sources disagree ends with
`sources disagree`. Skipped configuration is listed once at the end, even for
file types that match the default.

## Which repository

The worktree is found from the current directory, or from `-b/--base <DIR>`,
exactly as other `sniff repo` commands find it, so running from a
subdirectory works. Only the worktree root's `.editorconfig` and formatter
files are read.

Outside a Git worktree the command still succeeds: no configuration file is
read, a language convention applies where the language has one (Go gets tabs
from `gofmt`), and anything else gets four spaces. The summary then has no
overrides.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `<FILE-EXT>` | Optional. The extension to resolve, such as `yaml` or `.yaml`. After the optional dot it must start with an ASCII letter or digit, followed by ASCII letters, digits, `_`, `+`, or `-` (`c++` and `d-ts` are valid). Anything else (`..yaml`, `*.yaml`, `y/aml`, an empty string) is a usage error with exit status 2 |
| `-b/--base <DIR>` | Find the worktree from `DIR` instead of the current directory |
| `--json` | Print the result as JSON instead of text |
| `--plain` | Strip terminal styling from the text output |

An extension that starts with `-` must follow `--` so it is not read as a
flag: `sniff repo indent -- -x` (it is then rejected by the rule above).

## JSON Output (`--json`)

`--json` writes only the result to stdout: provenance, adjustment, and
diagnostics included, nothing on stderr. Keys are snake_case, styles are
`space` or `tab`, paths are relative to the worktree root and always use `/`,
and a value a source does not state is left out of that source's object.

With an extension, the output is one resolved result:

```json
{
  "indent_using": "space",
  "indent": 4,
  "authority": {
    "source": "editorconfig",
    "path": ".editorconfig",
    "via": ["*", "*.{js,ts,jsx,tsx,vue,json,yaml,yml,toml}"],
    "indent_using": "space",
    "indent": 4
  },
  "alt_authorities": [
    { "source": "biome", "path": "biome.json", "via": [], "indent_using": "tab", "indent": 2 }
  ],
  "consensus": false,
  "skipped": []
}
```

| Key | Meaning |
|-----|---------|
| `indent_using`, `indent` | The style and columns per level to write with |
| `authority` | The highest-ranked source. `source` is `editorconfig`, a formatter (`rustfmt`, `prettier`, `biome`, `clang-format`, …), `convention`, or `default`. `path` is absent for `convention` and `default`. `via` lists EditorConfig sections, the formatter keys read (empty when its defaults applied), or the convention's name |
| `alt_authorities` | The other sources, in rank order, in the same shape |
| `consensus` | Every source agrees on each value it states |
| `adjustment` | Present only when the file format changed the style: `{ "requested": "tab", "reason": "yaml_forbids_tabs" }` |
| `skipped` | Configuration that could not be used: `{ "path", "key"?, "reason" }`, where `reason` gives the line and column when the file is malformed |

Without an extension, the output is the summary, with one resolved result per
entry and the diagnostics of every query deduplicated:

```json
{
  "default": { "indent_using": "space", "indent": 4, "authority": { … }, … },
  "overrides": {
    "go": { "indent_using": "tab", "indent": 8, … },
    "yaml": { "indent_using": "space", "indent": 2, … }
  },
  "skipped": []
}
```

The same summary appears under `indent` in the bare
[`sniff repo --json`](./repo.md#aggregate-output-for-bare-sniff-repo---json)
aggregate. No other `sniff repo` command reads formatter configuration.

## Cost

One Git discovery, then one read of each candidate file that exists: a single
extension reads `.editorconfig` and only its type's formatter files, and the
summary reads every supported file once. Reads are counted under `--perf` as
`filesystem.indentation.config_parses` and the `filesystem.io.*` counters. No
formatter binary is run and no network request is made.
