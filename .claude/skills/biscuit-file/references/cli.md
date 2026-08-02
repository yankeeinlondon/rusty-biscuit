## CLI Reference (`bf`)

Binary name: `bf`, installed to `~/.cargo/bin/` via `just install`.

### Format Conversion

```sh
bf [OPTIONS] [FILE]

# Convert between formats
bf config.toml --json              # TOML → JSON
bf data.yaml --toml                # YAML → TOML
bf settings.json --json5            # JSON → JSON5
bf config.json5 --yaml              # JSON5 → YAML

# Compact output (JSON/JSON5 only)
bf data.json --json5 --compact    # single-line JSON5

# STDIN piping (requires --input-format)
cat data.yaml | bf --input-format yaml --json

# Markdown frontmatter extraction
bf post.md --json                  # extracts YAML/TOML frontmatter as JSON

# PDF extraction
bf document.pdf                  # plain text (default)
bf document.pdf --md              # as Markdown
```

### Output Format Flags

Mutually exclusive: `--json` (default for data), `--json5`, `--yaml`, `--toml`, `--text` (default for PDF), `--md`.

`--compact`: single-line output (JSON/JSON5 only, ignored for others).
 `--input-format`: override auto-detection (required for STDIN).
 `--debug`: enable debug logging.

### File Reference Resolution

```sh
bf reference @docs/spec.md           # magic: search repo root, HOME
 
bf reference !README.md              # package: search Cargo workspace area
bf reference %foo.md                  # recursive: walk directories for a match
bf ref ./Cargo.toml                   # alias for 'reference'
bf reference --relative-cwd @docs/spec.md  # relative to CWD
 
bf reference -v ~/vault1 -v ~/vault2 vault:note.md  # custom vault roots
```

Exit codes: `0` = found, `1` = not found (well-formed but no match), `2` = error.

 `bf reference` can be abbreviated as `bf ref`.

Resolved paths print through `biscuit_file::to_portable_string`, so stdout is
`/`-separated on Windows too (`C:/repo/Cargo.toml`). A UNC, device, or
unreducible verbatim path has no faithful portable spelling and prints
natively. Assert on the portable form in tests; never on a leading `/`, which
only a Unix-rooted path has.

### CLI Architecture

Binary: `biscuit-file/cli/src/main.rs`

- Uses `clap` for arg parsing with `ArgGroup` for mutually exclusive output flags
 
- Uses `color-eyre` for error handling
  Format conversion delegates to library types: `Toml`, `Yaml`, `Json5`, `Pdf`
 Markdown frontmatter extracted via `extract_frontmatter()` helper, detects `---` (YAML) or `+++` (TOML) delimiters
 PDF defaults to text output; data formats default to JSON
 `FileReference` subcommand builds reference with vaults, calls `resolve()` or `resolve_relative(None)`.
