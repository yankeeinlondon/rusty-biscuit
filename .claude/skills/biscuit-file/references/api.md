## Toml

Source: `biscuit-file/lib/src/toml_impl/types.rs`

### Construction

```rust
use biscuit_file::Toml;

// From file
let toml = Toml::new("config.toml")?;

// From string
let toml = Toml::from_str("[package]\nname = \"demo\"")?;

// From reader
let toml = Toml::from_reader(reader)?;

// From existing value
let toml = Toml::from_value(value)?;

// Validate without constructing
assert!(Toml::is_valid("[package]\nname = \"demo\""));
```

### Conversion

```rust
let json: String = toml.as_json()?;              // pretty JSON
let json_value: serde_json::Value = toml.as_json_value()?;  // owned Value
let yaml: String = toml.as_yaml()?;             // requires `yaml` feature
let yaml_value: serde_yaml_ng::Value = toml.as_yaml_value()?; // requires `yaml`
let raw: &str = toml.raw();                     // original TOML text
let value: &toml::Value = toml.value();           // parsed value tree
let source: &TomlSource = toml.source();       // Path or Inline
```

### TOML-to-JSON Datetime Handling

TOML datetime types convert to RFC 3339 strings in JSON. Non-finite floats become `null`.

## Yaml

Source: `biscuit-file/lib/src/yaml/types.rs`

### Construction
```rust
use biscuit_file::Yaml;

// From file, string, or or bytes
let yaml = Yaml::new("config.yaml")?;
let yaml = Yaml::from_str("name: demo")?;
let yaml = Yaml::from_bytes(b"name: demo")?;

// From existing value
let yaml = Yaml::from_value(serde_yaml_ng::Value);

// Validate
assert!(Yaml::is_valid("name: demo"));
```

### Conversion
```rust
// To JSON (returns serde_json::Value)
let json: serde_json::Value = yaml.as_json()?;

// To JSON with custom policies
let output: ConversionOutput<serde_json::Value> = yaml.as_json_with(options)?;
// output.value = the JSON, output.warnings = any warnings

// To TOML (requires `toml` feature)
let toml: toml::Value = yaml.as_toml()?;
```

### Edge Case Policies (via `as_json_with` / `as_toml_with`)
```rust
use biscuit_file::yaml::{JsonConversionOptions, NonStringKeyPolicy, NonFiniteFloatPolicy};
use biscuit_file::yaml::{TomlConversionOptions, NullPolicy, HeteroArrayPolicy};
```

## Json5
Source: `biscuit-file/lib/src/json5/types.rs`

### Construction
```rust
use biscuit_file::Json5;

let j = Json5::new("config.json5")?;           // from file
let j = Json5::from_str("{ key: 'value' }")?;    // from string
let j = Json5::from_bytes(b"{ key: 'value' }")?; // from bytes

// Validate
assert!(Json5::is_valid("{ key: 'value' }"));       // JSON5
assert!(Json5::is_valid_json(r#"{"key":"value"}"#)); // strict JSON only
```

### Conversion
```rust
let json: String = j.as_json()?;               // pretty JSON
let json_compact: String = j.as_json_compact()?;   // single-line JSON
let json_value: &serde_json::Value = j.as_json_value(); // reference
let json5: String = j.as_json5();            // pretty idiomatic JSON5
let json5_compact: String = j.as_json5_compact(); // single-line JSON5
let yaml: String = j.as_yaml()?;             // requires `yaml` feature
let toml: String = j.as_toml()?;             // requires `toml` feature
```

### Standalone JSON5 Formatters
```rust
use biscuit_file::json5::{to_json5_pretty, to_json5_compact};

let pretty: String = to_json5_pretty(&serde_json_value);
let compact: String = to_json5_compact(&serde_json_value);
```

## PDF
Source: `biscuit-file/lib/src/pdf/types.rs`

### Construction
```rust
use biscuit_file::Pdf;

let pdf = Pdf::new("document.pdf")?;
let pdf = Pdf::from_bytes(bytes)?;
let pdf = Pdf::from_bytes_with_config(bytes, config)?;
```

### Extraction
```rust
let text: String = pdf.as_text()?;      // requires `extract` feature
let md: PdfMarkdown = pdf.as_markdown(Default::default())?; // text wrapped in markdown
let toc: PdfToc = pdf.toc()?;   // requires `lopdf` feature
```

### Configuration
```rust
use biscuit_file::{PdfConfig, PageRange, MarkdownOptions, ImageMode, BackendPreference};

let config = PdfConfig::default()
    .with_backend(BackendPreference::Extract)
    .with_password("secret")
    .with_page_range(PageRange::new(1, 5))
    .with_max_pages(10)
    .with_normalize_text(false)
    .with_remove_headers_footers(false);
```

## Portable Path Text
Source: `biscuit-file/lib/src/path_text.rs` (unfeatured — available with `--no-default-features`)

```rust
use std::path::Path;
use biscuit_file::{to_portable_string, try_portable_string};

// Portable text when a faithful slash-separated spelling exists;
// otherwise the NATIVE spelling, unchanged.
let s: String = to_portable_string(Path::new(r"docs\file.md")); // "docs/file.md"

// Same policy, with the fallback exposed so a caller can branch on it.
let maybe: Option<String> = try_portable_string(Path::new(r"\\server\share\f.md")); // None
```

Which to use:

| Consumer | Function | Why |
|----------|----------|-----|
| Markdown link destination, generated URL-adjacent text | `try_portable_string` | CommonMark eats backslash escapes; a native spelling does not survive a parse. Error or preserve on `None`. |
| Diagnostics, completion candidates, YAML scalars | `to_portable_string` | Native text is still correct output for these. |

Declined (`None` / native fallback) on Windows: UNC, device-namespace, and any
verbatim path `dunce::simplified` will not reduce — reserved DOS names,
trailing dot/space, over-`MAX_PATH`, and literal `.`/`..` names under `\\?\`.
No lexical `.`/`..` collapse happens; `dunce`'s refusal is authoritative.

Lossy by design: non-Unicode data becomes U+FFFD (`Path::to_string_lossy`), and
on Unix a literal `\` in a filename renders as `/`.

Never use rendered text as a path-identity key — use `PathIdentity` (below).
A short root can simplify while its long descendant cannot.

## Path Identity
Source: `biscuit-file/lib/src/file_reference/portable/path_identity.rs` (feature `file-reference`)

```rust
use std::path::Path;
use biscuit_file::{PathIdentity, RelativeRoute};

let root = PathIdentity::new(Path::new("/opt/config"));
PathIdentity::new(Path::new("/opt/config-old/a")).starts_with(&root); // false: whole components
let target = PathIdentity::new(Path::new("/repo/assets/logo.png"));
let route: Option<RelativeRoute> = target.relative_from(&PathIdentity::new(Path::new("/repo/docs")));
// route.parent_hops() == 1, route.forward() == ["assets", "logo.png"]; None across drives/shares
```

The single prefix/relative-route implementation (Darkmatter's link
normalization uses it; never write another `ComparisonKey`). Lexical and
lossless: collapses `.`/`..` on ordinary paths (never above a root; a relative
path keeps leading `..`), keeps `.`/`..` literal under `\\?\`, equates a
verbatim drive/share with its legacy spelling (even when too long for `dunce`),
folds only the drive letter, and never canonicalizes or equates symlink aliases.
`relative_from(dir)` always treats `dir` as a directory. The Windows grammar is
a portable UTF-16 parser (`portable::path_identity::windows`), so its tests run
on every host; a Windows-only test pins it to std's `Prefix` classification.

Resolution and context selection normalize native paths through
`resolve::normalize_components`, which applies the same rules
(`portable::path_identity::normalize_native` shares `PathIdentity`'s `apply`)
and then reduces a dot-free verbatim path with `dunce`. Never hand-roll another
`..`-popping loop: `Vec::pop` on components removes the root, and
`PathBuf::push` silently collapses `.`/`..` onto a verbatim buffer.
`diff_paths` routes through `PathIdentity` after that normalization.

Crate-internal: `portable::text::{render_reference, render_absolute}` is the
generated-reference text seam. It renders through `try_portable_string`, then
re-parses and rejects `Unrenderable` (non-Unicode), `ChangesComponents` (Unix
`\`, literal verbatim dots, Windows names that change without `\\?\`),
`GrammarMismatch` (`{{…}}` in a name, a leading sigil in a bare name), and
`NoPortableSpelling`.

## Portable References: `PortablePath`
Source: `biscuit-file/lib/src/file_reference/portable/{evaluate,strategy,env_anchor,diagnostics}.rs` (feature `file-reference`)

```rust
use biscuit_file::{FileReference, PortabilityPreference as P, PortablePath, PortablePathError};

let found = PortablePath::from_path("/repo/foo.md")          // or ::from_reference(FileReference)
    .with_ctx(&ctx)                                          // clone; no discovery, no live reads
    .with_portable_env(["CONFIG_DIR"])                       // names only; values from ctx env
    .with_strategy(P::DEFAULT_STRATEGY.iter().cloned())      // replace; [] matches nothing
    .file_reference()?;                                      // does real work (resolve + verify)
found.reference();   // &FileReference (also AsRef / into_reference())
found.strategy();    // &P that matched; P::AbsolutePath => caller warns
found.attempts();    // &[Attempt { strategy, outcome, rejected }], one per preference tried
found.findings();    // &[Finding] about the returned reference
```

- Default: `AuthoredIntent(ALL)`, `SameDirRelative`, `ChildDir`, `PeerDir`,
  `ImmediateParentDir`, `RepoRoot(None)`, `EnvRootedPath`, `HomeDir`,
  `AbsolutePath`. Opt-in: `ParentDir`, `ExternalRelativePath` (verified with
  `allow_external_relative()`), `RepoMultiPath(filter)` (`^`), `MagicPath(filter)`
  (`@`; a filter must name an `@` root and spellings come from it).
- Filters are eligibility only (`RepoRoot(Some("docs"))` still writes `&docs/x.md`);
  bad syntax is `InvalidConfiguration(InvalidFilter)` before any preference runs.
- Reference inputs: intent forms (`~ @ ^ & vault:`, URL, `%`, leading portable
  `{{VAR}}`) are kept by `AuthoredIntent`; position forms are resolved
  (`resolve_detailed`) and rewritten. Minimal churn keeps `./x.md` / bare `x.md`
  already in the chosen form; results are fixed points. A multi-candidate miss,
  boundary escape, missing anchor, or probe failure is `UnresolvableInput`
  (caller keeps the link); URL/`%` without `AuthoredIntent` is
  `NormalizationUnsupported`.
- Every candidate is rendered through the crate-internal `text` seam and
  verified by resolving it in the same context; `@`/`^` need an existing file
  found first (`Shadowed` otherwise), single-location forms may be missing.
- Portable names: `PORTABLE_ENV_VARIABLES` (comma list in the evaluation's env)
  ∪ `with_portable_env`; invalid names → `Finding::InvalidPortableVariableName`.
  Value must be host-absolute and a whole-component prefix (`EnvAnchorProblem`);
  deepest wins, then name order.
- Errors are `Clone` (`ProbeError` keeps path, `ErrorKind`, OS code); every
  variant has `attempts()` (empty before any preference ran) and `findings()`.
  A candidate lookup that fails with I/O ends evaluation as `ProbeFailed`, but
  the failing preference is still the last attempt, with outcome
  `AttemptOutcome::ProbeFailed(error)` and its earlier `rejected` candidates;
  a target probe failure has no such attempt.
  The `reference` in `UnresolvableInput`/`NormalizationUnsupported` is boxed.
- `with_ctx` + `with_cwd`/`with_base_dir` → `InvalidConfiguration`. Without a
  context: capture cwd/home/env once, `find_git_root(cwd)`; `with_base_dir`
  inside a repository must equal its root.
- A relative directory → `InvalidConfiguration(RelativeDirectory { path })`
  before any preference runs, from `with_cwd`/`with_base_dir` or from a
  `with_ctx` context whose `validate()` returns `RelativeContextDirectory`.
- PortablePath compares paths **lexically**. A caller holding canonical paths
  (`/private/var/…`, `\\?\C:\…`) next to a context opened with another
  spelling must re-spell the target first (Darkmatter's
  `link_normalization::in_context_spelling`). A Windows variable whose value is
  verbatim (`\\?\C:\…`) never anchors `{{VAR}}/…`: interpolation
  concatenates text, so verification rejects it (`os` skill, windows.md).
- Consumer pattern (Darkmatter compose finalization): split `#frag`/`?q`/`:line`
  off the parsed destination (skip a `\\?\` prefix's `?`), `from_path` +
  `with_ctx(&source_ctx)`, reattach the suffix; keep the destination and warn
  on any error. Darkmatter writes an `EnvRootedPath` result as `{{{VAR}}}/…`
  so recomposing its output is stable.

## File Detection
Source: `biscuit-file/lib/src/detect.rs`

```rust
use biscuit_file::{detect_file_type, detect_file_type_from_bytes, FileType};

let ft: FileType = detect_file_type("config.toml")?;
let ft: FileType = detect_file_type_from_bytes(b"%PDF-1.7\n");

// FileType variants: Toml, Yaml, Json, Json5, Markdown, Pdf, Unknown
assert_eq!(ft.extension(), Some("toml"));
assert_eq!(ft.mime_type(), Some("application/toml"));
assert_eq!(ft.as_data_format(), Some(DataFormat::Toml));
```
