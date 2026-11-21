---
created: 2026-11-20
researched_on: 2026-11-20
related: 2026-11-20-contextual-indents
---

# Formatter Indentation Reference

This is a research companion to `spec.md`. It lists common code formatters,
the configuration files they read, and how each one represents indentation.
Every row was checked against the tool's official documentation and, where
the docs were thin, its source on the default branch as of the date above.
Anything not confirmed is marked **unverified**.

"Lookup" describes the formatter's own discovery. Sniff deliberately reads
only the worktree root, so where a formatter walks parents or starts from
the working directory, Sniff's answer can differ for nested configs.

## Summary

| Formatter | Languages | Config files (own precedence) | Style key | Width key | Default | Reads `.editorconfig` |
| --------- | --------- | ----------------------------- | --------- | --------- | ------- | --------------------- |
| rustfmt | Rust | `.rustfmt.toml`, `rustfmt.toml` | `hard_tabs = true` | `tab_spaces` | 4 spaces | No |
| gofmt | Go | none | fixed tabs | fixed (alignment assumes 8) | tabs | No |
| zig fmt | Zig | none | fixed spaces | fixed 4 | 4 spaces | No |
| Prettier | JS/TS, JSON, CSS/SCSS/Less, HTML, Vue, GraphQL, Markdown/MDX, YAML | `package.json`/`package.yaml` `prettier` key, `.prettierrc`, `.prettierrc.{json,yml,yaml,json5}`, JS/TS configs, `.prettierrc.toml` | `useTabs` | `tabWidth` | 2 spaces | Yes (config wins) |
| Biome | JS/TS, JSON/JSONC, CSS, GraphQL, YAML, Markdown (HTML experimental) | `biome.json`, `biome.jsonc`, `.biome.json`, `.biome.jsonc` | `formatter.indentStyle: "tab"\|"space"` | `formatter.indentWidth` | **tabs**, width 2 | Only with `formatter.useEditorconfig: true` (default false) |
| Deno fmt | JS/TS, JSON/JSONC, Markdown, CSS/SCSS/Less, HTML, YAML, ipynb | `deno.json`, `deno.jsonc` | `fmt.useTabs` | `fmt.indentWidth` | 2 spaces | Yes, by default (`fmt.useEditorConfig`) |
| dprint | per plugin (TS/JS, JSON, …) | `dprint.json`, `dprint.jsonc`, `.dprint.json`, `.dprint.jsonc`, `.config/dprint.json(c)` | `useTabs` | `indentWidth` | plugin's (TS and JSON: 2 spaces) | No (unverified) |
| oxfmt | JS/TS, JSON/JSON5, YAML, TOML, HTML, Vue, Svelte, CSS, Markdown, GraphQL | `.oxfmtrc.json`, `.oxfmtrc.jsonc`, `oxfmt.config.ts`, `oxfmt.config.mts` (one per directory) | `useTabs` | `tabWidth` | 2 spaces | Yes, for unset fields |
| Ruff | Python | `.ruff.toml`, `ruff.toml`, `pyproject.toml` `[tool.ruff]` | `format.indent-style = "tab"\|"space"` | `indent-width` | 4 spaces | No |
| Black | Python | `pyproject.toml` (no indent keys) | fixed spaces | fixed 4 | 4 spaces | No |
| YAPF | Python | `.style.yapf` `[style]`, `setup.cfg` `[yapf]`, `pyproject.toml` `[tool.yapf]` | `use_tabs` | `indent_width`, `based_on_style` | pep8 preset: 4 spaces | No |
| autopep8 | Python | `pyproject.toml` `[tool.autopep8]`, `setup.cfg`, `tox.ini`, `.pep8`, `.flake8` | keeps tabs unless W191 | `indent-size` | 4 spaces | No |
| clang-format | C, C++, ObjC, Java, JS, Proto, C#, JSON, Verilog | `.clang-format`, `_clang-format` | `UseTab` (`Never` = spaces) | `IndentWidth` (`TabWidth` separate) | LLVM preset: 2 spaces | No |
| StyLua | Lua, Luau | `stylua.toml`, `.stylua.toml` | `indent_type = "Tabs"\|"Spaces"` | `indent_width` | **tabs**, width 4 | Yes, fallback |
| shfmt | Shell | `.editorconfig` only | `indent_size = 0` (tabs) | `indent_size` / `-i N` | tabs | Yes, only source |
| Taplo | TOML | `.taplo.toml`, `taplo.toml` | `formatting.indent_string` | `formatting.indent_string` | 2 spaces | No |
| Tombi | TOML | `.tombi.toml`, `tombi.toml`, `.config/tombi.toml`, `pyproject.toml` `[tool.tombi]` | `format.rules.indent-style` | `format.rules.indent-width` | 2 spaces | No |
| yamlfmt | YAML | `.yamlfmt`, `.yamlfmt.yml`, `.yamlfmt.yaml`, `yamlfmt.yml`, `yamlfmt.yaml` | spaces only | `formatter.indent` | 2 spaces | No |
| yamllint (linter) | YAML | `.yamllint`, `.yamllint.yaml`, `.yamllint.yml` | spaces only | `rules.indentation.spaces` (int or `consistent`) | `consistent` | Not documented |
| swift-format | Swift | `.swift-format` | `indentation: {"tabs": N}` | `indentation: {"spaces": N}`, `tabWidth` | 2 spaces | No |
| SwiftFormat | Swift | `.swiftformat` (cascades root to leaf) | `--indent tab` | `--indent N`, `--tab-width` | 4 spaces | No |
| ktlint | Kotlin | `.editorconfig` only | `indent_style` | `indent_size` | 4 spaces | Yes, only source |
| ktfmt | Kotlin | none (`.editorconfig` with `--enable-editorconfig`) | spaces only | style flag or `indent_size` | Meta style: 2 spaces | Opt-in |
| google-java-format | Java | none | fixed spaces | fixed 2 (`--aosp`: 4) | 2 spaces | No |
| palantir-java-format | Java | none | fixed spaces | style flag | CLI: Google 2; Gradle/Spotless: Palantir 4 | No |
| scalafmt | Scala, sbt | `.scalafmt.conf` (current dir, then git root) | spaces only | `indent.main` (+ per-site keys) | 2 spaces | No |
| RuboCop | Ruby | `.rubocop.yml`, `.config/.rubocop.yml`, `.config/rubocop/config.yml` | `Layout/IndentationStyle: EnforcedStyle` | `Layout/IndentationWidth: Width` | 2 spaces | Not documented |
| php-cs-fixer | PHP | `.php-cs-fixer.php`, `.php-cs-fixer.dist.php` (PHP code) | `setIndent("\t")` | `setIndent("  "\|"    ")` | 4 spaces | Not documented |
| CSharpier | C#, XML project files | `.csharpierrc`, `.csharpierrc.json`, `.csharpierrc.yaml`, else `.editorconfig` | `useTabs` | `indentSize` | 4 spaces (XML 2) | Yes |
| dotnet format | C#, VB | `.editorconfig` only | `indent_style` | `indent_size` | 4 spaces (unverified) | Yes, only source |
| mix format | Elixir | `.formatter.exs` (no indent keys) | fixed spaces | fixed 2 | 2 spaces | No |
| dart format | Dart | `analysis_options.yaml` `formatter:` (no indent keys) | fixed spaces | fixed 2 | 2 spaces | No |
| terraform fmt | Terraform | none | fixed spaces | fixed 2 | 2 spaces | No |
| nixfmt | Nix | none (CLI `--indent N`) | spaces only | CLI only | 2 spaces | No |
| alejandra | Nix | `alejandra.toml` (experimental, current dir) | `indentation = "Tabs"` | `"TwoSpaces"\|"FourSpaces"` | 2 spaces | No |
| ormolu | Haskell | none | fixed spaces | fixed 2 | 2 spaces | No |
| fourmolu | Haskell | `fourmolu.yaml` | spaces only | `indentation` | 4 spaces | No |
| elm-format | Elm | none | fixed spaces | fixed 4 | 4 spaces | No |
| gleam format | Gleam | none | fixed spaces | fixed 2 | 2 spaces | No |
| sqlfluff | SQL | `setup.cfg`, `tox.ini`, `pep8.ini`, `.sqlfluff`, `pyproject.toml` (later wins) | `indent_unit = space\|tab` | `tab_space_size` | 4 spaces | Not documented |
| mdformat | Markdown | `.mdformat.toml` (no indent keys) | n/a | list indent = marker width | n/a | No |

## Representation patterns

Formatters express indentation in five ways. A resolver needs a mapping for
each one it supports.

1. **Boolean plus width:** `useTabs`/`tabWidth` (Prettier, oxfmt, Deno,
   dprint, CSharpier), `hard_tabs`/`tab_spaces` (rustfmt), `use_tabs`/
   `indent_width` (YAPF).
2. **Style enum plus width:** Biome (`"tab"|"space"`), Ruff, Tombi, StyLua
   (`"Tabs"|"Spaces"`), RuboCop, sqlfluff, clang-format (`UseTab` has five
   values; only `Never` means spaces).
3. **A literal indent string:** Taplo `indent_string`, php-cs-fixer
   `setIndent`. Count the spaces, or detect tabs.
4. **A single tagged value:** swift-format `{"spaces": N}` or `{"tabs": N}`,
   SwiftFormat `--indent N|tab`, alejandra `"TwoSpaces"|"FourSpaces"|"Tabs"`.
5. **Preset names that imply a width:** clang-format `BasedOnStyle`, YAPF
   `based_on_style`, ktfmt and palantir-java-format style flags.

Formatters with no setting at all (gofmt, zig fmt, Black, mix, dart,
terraform, ormolu, elm-format, gleam, google-java-format) fit the spec's
built-in convention tier: the language answers, not a file.

## Details worth knowing

### Discovery quirks

- **Start directory differs.** Most formatters search upward from the file
  being formatted. yamlfmt, Taplo and the Tombi CLI start from the working
  directory instead. scalafmt checks only the current directory and the git
  root.
- **Nondeterministic same-directory precedence.** yamlfmt checks names in Go
  map order. Taplo documents `.taplo.toml` first but returns whichever the
  directory listing yields first. Treat two config files in one directory
  as a diagnostic.
- **Cascading versus nearest-wins.** SwiftFormat and sqlfluff merge every
  config from root to leaf. rustfmt, Ruff, Prettier, clang-format, fourmolu
  and swift-format use the nearest file only. dprint and Biome support
  nested configs that opt into inheritance (`"inherit": true`;
  `root: false` with `extends: "//"`).
- **Stops at an irrelevant file.** autopep8 stops its parent search at the
  first `pyproject.toml`, even one without `[tool.autopep8]`. Ruff, by
  contrast, skips a `pyproject.toml` without `[tool.ruff]`.
- **One config per directory.** oxfmt refuses both a JSON and a TS config in
  the same directory.

### EditorConfig interaction

- **Config wins over EditorConfig:** Prettier, Deno, oxfmt (fallback for
  unset fields only), CSharpier, StyLua (fallback only when no StyLua config
  exists).
- **EditorConfig is the only source:** shfmt, ktlint, dotnet format.
- **Opt-in:** Biome (`formatter.useEditorconfig`, default `false`), ktfmt
  (`--enable-editorconfig`).
- **Ignored:** rustfmt, Ruff, clang-format, Taplo, Tombi, yamlfmt, swift-format,
  SwiftFormat, scalafmt, and the fixed-style formatters.

Biome is the important case: with no `.editorconfig` support enabled, a
Biome project indents with tabs whatever `.editorconfig` says. The spec
puts EditorConfig first anyway, which matches the project's stated intent
but can differ from what `biome format` produces.

### Per-language and per-path overrides

| Formatter | Mechanism |
| --------- | --------- |
| Prettier, oxfmt | `overrides: [{ files, excludeFiles?, options }]`, globs relative to the config file; a pattern without `/` matches the basename at any depth |
| Biome | language blocks (`javascript.formatter`, `json.formatter`, `css.formatter`, `graphql.formatter`, `html.formatter`, `markdown.formatter`, `yaml.formatter`); `overrides: [{ includes, formatter, … }]`, last match wins. `yaml.formatter` has `indentWidth` but no `indentStyle` |
| dprint | plugin blocks (`typescript`, `json`, …) over the globals; top-level `overrides: [{ files, … }]`, later wins |
| clang-format | multi-document YAML keyed by `Language:`; `BasedOnStyle: InheritParentConfig` chains to the parent directory |
| Taplo | `[[rule]]` with `include`/`exclude`/`keys`, last match wins |
| Tombi | `[[overrides]]` with `files.include`/`files.exclude` and `format.rules` |
| scalafmt | `fileOverride { "glob:**.sbt" { … } }`, longest match wins |
| SwiftFormat | `[Section]` blocks with `--filter`, and inline `// swiftformat:options` |
| RuboCop | per-cop `Include`/`Exclude`; nearest `.rubocop.yml` per directory |
| Deno, Ruff, rustfmt, swift-format, yamlfmt | none within one file |

### Width semantics that are not "columns per level"

- **gofmt** indents with one tab per level; alignment is computed as if a
  tab is 8 columns. Sniff's reporting width of 4 is a display choice.
- **clang-format** separates `IndentWidth` (level) from `TabWidth` (tab
  stop). With `UseTab: ForIndentation` and `IndentWidth: 4, TabWidth: 8`,
  one level is half a tab.
- **swift-format** `{"tabs": N}` means N tabs per level; `tabWidth`
  (default 8) is only used for line-length math.
- **StyLua** with tabs uses `indent_width` only as a line-width heuristic.
- **Continuation indents** differ from block indents in google-java-format
  (2/4), palantir (4/8 for its own style), ktfmt Meta (2/4), dart (2/4) and
  clang-format LLVM (2/4). Sniff reports the block indent only.

### clang-format presets

| `BasedOnStyle` | `IndentWidth` | `TabWidth` | `UseTab` |
| -------------- | ------------- | ---------- | -------- |
| `LLVM` (default) | 2 | 8 | `Never` |
| `Google` | 2 | 8 | `Never` |
| `Chromium` | 2 (Java 4) | 8 | `Never` |
| `Mozilla` | 2 | 8 | `Never` |
| `WebKit` | 4 | 8 | `Never` |
| `Microsoft` | 4 | 4 | `Never` |
| `GNU` | 2 | 8 | `Never` |
| `clang-format` | 2 | 8 | `Never` |

## Consequences for the spec

Applied to `spec.md`: dprint's filename order (`dprint.json`,
`dprint.jsonc`, `.dprint.json`, `.dprint.jsonc`), StyLua's (`stylua.toml`
before `.stylua.toml`), Biome's `.biome.json`/`.biome.jsonc`, and Prettier's
`package.yaml` key.

Open choices:

- **Biome** also formats YAML and Markdown; its `yaml` block has
  `indentWidth` but no `indentStyle`. Adding those types to the Biome row
  would put Biome's tab default ahead of Deno and yamlfmt for YAML (the
  format constraint would convert it to spaces).
- **dprint** also reads `.config/dprint.json(c)`.
- **Deno** `fmt.options` is deprecated but still honored when the flat keys
  are absent; reading it is cheap and avoids a wrong answer.
- **Candidate rows not yet in the spec:** oxfmt (Prettier-compatible keys),
  Tombi (better specified than Taplo), swift-format, RuboCop, sqlfluff,
  CSharpier, fourmolu. Built-in conventions could add elm (4), gleam (2),
  haskell via ormolu (2), and `tftest.hcl`.
- **gofmt** reporting width: consider 8 to match gofmt's own alignment.

## Sources

- rustfmt: https://github.com/rust-lang/rustfmt/blob/master/src/config/mod.rs
- gofmt: https://pkg.go.dev/cmd/gofmt
- Prettier: https://prettier.io/docs/configuration, https://prettier.io/docs/options
- Biome: https://biomejs.dev/reference/configuration/, https://biomejs.dev/internals/language-support/
- Deno: https://docs.deno.com/runtime/reference/cli/fmt/, https://docs.deno.com/runtime/reference/deno_json/
- dprint: https://dprint.dev/config/, https://dprint.dev/plugins/typescript/config/
- oxfmt: https://oxc.rs/docs/guide/usage/formatter/config
- Ruff: https://docs.astral.sh/ruff/settings/
- Black: https://black.readthedocs.io/en/stable/the_black_code_style/current_style.html
- YAPF: https://github.com/google/yapf
- autopep8: https://github.com/hhatto/autopep8
- clang-format: https://clang.llvm.org/docs/ClangFormatStyleOptions.html
- StyLua: https://github.com/JohnnyMorganz/StyLua
- shfmt: https://github.com/mvdan/sh/blob/master/cmd/shfmt/shfmt.1.scd
- Taplo: https://taplo.tamasfe.dev/configuration/formatter-options.html
- Tombi: https://tombi-toml.github.io/tombi/
- yamlfmt: https://github.com/google/yamlfmt/blob/main/docs/config-file.md
- yamllint: https://yamllint.readthedocs.io/en/stable/rules.html
- swift-format: https://github.com/swiftlang/swift-format/blob/main/Documentation/Configuration.md
- SwiftFormat: https://github.com/nicklockwood/SwiftFormat
- ktlint: https://github.com/pinterest/ktlint
- ktfmt: https://github.com/facebook/ktfmt
- google-java-format: https://github.com/google/google-java-format
- palantir-java-format: https://github.com/palantir/palantir-java-format
- scalafmt: https://scalameta.org/scalafmt/docs/configuration.html
- RuboCop: https://docs.rubocop.org/rubocop/latest/configuration.html
- php-cs-fixer: https://cs.symfony.com/doc/config.html
- CSharpier: https://csharpier.com/docs/Configuration
- dotnet format: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-format
- mix format: https://hexdocs.pm/mix/Mix.Tasks.Format.html
- dart format: https://dart.dev/tools/dart-format
- terraform fmt: https://developer.hashicorp.com/terraform/cli/commands/fmt
- nixfmt: https://github.com/NixOS/nixfmt
- alejandra: https://github.com/kamadorueda/alejandra
- fourmolu: https://fourmolu.github.io/config/
- elm-format: https://github.com/avh4/elm-format
- gleam: https://github.com/gleam-lang/gleam
- sqlfluff: https://docs.sqlfluff.com/en/stable/configuration/setting_configuration.html
- mdformat: https://mdformat.readthedocs.io/
