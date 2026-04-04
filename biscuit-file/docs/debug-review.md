# Debug & Tracing Review: biscuit-file

**Date:** 2026-04-03
**Scope:** `biscuit-file/lib` and `biscuit-file/cli` (the `bf` binary)
**Compared against:** claudine, research (mature packages in this monorepo)

## Executive Summary

The biscuit-file package has **zero tracing instrumentation** in both the library and CLI. Neither `tracing` nor `tracing-subscriber` appear in either `Cargo.toml`. This is a significant gap compared to peer packages in this monorepo — claudine has 15+ instrumented call sites with structured spans, and research uses `#[instrument]` on all major functions plus a full `init_tracing()` setup in its CLI.

The package performs filesystem I/O, git repository discovery, Cargo workspace introspection, recursive directory walking, PDF parsing with multiple backends, and environment variable interpolation — all operations where tracing is essential for diagnosing real-world failures.

---

## Library (`biscuit-file`)

### Missing: `tracing` dependency

**Cargo.toml** has no `tracing` dependency at all. Add:

```toml
tracing = "0.1"
```

### Recommendation 1: `#[instrument]` on public API entry points

Every public constructor and conversion method should have a span. This makes it immediately clear which operation failed when a user reports an error. Use `skip` for large data and `fields` for key metadata.

**Files and functions to instrument:**

| File | Function | Suggested level | Fields |
|------|----------|----------------|--------|
| `toml_impl/types.rs` | `Toml::new()` | `debug` | `path` |
| `toml_impl/types.rs` | `Toml::from_str()` | `trace` | `input_len` |
| `toml_impl/types.rs` | `Toml::as_json()` | `trace` | — |
| `toml_impl/types.rs` | `Toml::as_yaml()` | `trace` | — |
| `yaml/types.rs` | `Yaml::new()` | `debug` | `path` |
| `yaml/types.rs` | `Yaml::from_str()` | `trace` | `input_len` |
| `yaml/types.rs` | `Yaml::from_bytes()` | `trace` | `input_len` |
| `yaml/types.rs` | `Yaml::as_json()` | `trace` | — |
| `yaml/types.rs` | `Yaml::as_toml()` | `trace` | — |
| `json5/types.rs` | `Json5::new()` | `debug` | `path` |
| `json5/types.rs` | `Json5::from_str()` | `trace` | `input_len` |
| `json5/types.rs` | `Json5::as_json()` | `trace` | — |
| `json5/types.rs` | `Json5::as_yaml()` | `trace` | — |
| `json5/types.rs` | `Json5::as_toml()` | `trace` | — |
| `pdf/types.rs` | `Pdf::new()` | `debug` | `path` |
| `pdf/types.rs` | `Pdf::from_bytes()` | `debug` | `byte_count = bytes.len()` |
| `pdf/types.rs` | `Pdf::as_text()` | `debug` | — |
| `pdf/types.rs` | `Pdf::as_markdown()` | `debug` | — |
| `pdf/types.rs` | `Pdf::toc()` | `debug` | — |

Example pattern (matches research lib style):

```rust
use tracing::{debug, instrument};

#[instrument(skip(self), fields(source = ?self.source))]
pub fn as_json(&self) -> Result<String, TomlError> {
    let json_value = self.as_json_value()?;
    let json = serde_json::to_string_pretty(&json_value)?;
    debug!(output_len = json.len(), "TOML → JSON conversion complete");
    Ok(json)
}
```

### Recommendation 2: File reference resolution needs debug/trace events

The file reference module does filesystem probing, git discovery, Cargo workspace inspection, recursive walking, and env var interpolation — all invisible today. This is the highest-value area for tracing.

**`file_reference/resolve.rs`:**

```rust
// In resolve_direct — log each candidate checked
for candidate in candidates {
    trace!(?candidate, exists = candidate.is_file(), "checking candidate");
    if candidate.is_file() {
        debug!(?candidate, "resolved file reference");
        return Ok(Some(normalize_absolute(&candidate, &ctx.cwd)));
    }
}
debug!("no candidate matched");
```

```rust
// In resolve_recursive — log search scope
debug!(root_count = roots.len(), ?needle, "starting recursive search");
// ...after walk:
debug!(match_count = matches.len(), "recursive search complete");
```

**`file_reference/context.rs`:**

```rust
// In from_ambient
debug!(?cwd, home_dir_set = home_dir.is_some(), env_var_count = env.len(),
       "built resolution context");

// In find_git_root
trace!(?from, "searching for git root");
// On success:
debug!(?workdir, "found git root");
// On not-found:
trace!("no git repository found");

// In find_package_area
trace!(?repo_root, ?cwd, "searching for package area");
debug!(?area, "found package area");  // or trace if None
```

**`file_reference/parse.rs`:**

```rust
// In parse()
trace!(raw, "parsing file reference");
// After detection:
debug!(?kind, recursive, "parsed reference");
```

### Recommendation 3: `detect.rs` — trace-level detection logging

File type detection is a common source of confusion ("why did it think my file was YAML?"). Add:

```rust
// In detect_file_type
trace!(?path, "detecting file type");
let from_bytes = detect_file_type_from_bytes(&bytes);
if from_bytes != FileType::Unknown {
    debug!(?from_bytes, "detected via magic bytes");
    return Ok(from_bytes);
}
let from_ext = detect_from_extension(path);
debug!(?from_ext, "detected via extension");
Ok(from_ext)
```

### Recommendation 4: PDF backends — warn on silent failures

In `pdf/backends.rs`, `extract_bookmarks()` returns `None` on failure with no logging. This silently swallows errors that users would want to know about:

```rust
// extract_toc — current code
let items = extract_bookmarks(&doc).unwrap_or_default();

// Suggested
let items = match extract_bookmarks(&doc) {
    Some(items) => {
        debug!(item_count = items.len(), "extracted PDF bookmarks");
        items
    }
    None => {
        warn!("could not extract PDF bookmarks — TOC will be empty");
        Vec::new()
    }
};
```

### Recommendation 5: Conversion result metrics at debug level

After successful conversions, log output size. This is invaluable for diagnosing performance issues with large files:

```rust
debug!(input_len = content.len(), output_len = text.len(), "PDF text extracted");
debug!(warnings = md.warnings.len(), assets = md.assets.len(), "PDF → Markdown complete");
```

---

## CLI (`biscuit-file-cli`)

### Missing: `tracing-subscriber` dependency

Add to `cli/Cargo.toml`:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Recommendation 6: Add tracing subscriber initialization

Follow the pattern established by claudine and research CLIs. Add an `init_tracing()` call in `main()`:

```rust
fn init_tracing(debug: bool) {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        EnvFilter::builder().parse_lossy(rust_log)
    } else if debug {
        EnvFilter::builder().parse_lossy("biscuit_file=debug,biscuit_file_cli=debug")
    } else {
        EnvFilter::builder()
            .with_default_directive(tracing::Level::WARN.into())
            .from_env_lossy()
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).with_target(true))
        .init();
}
```

### Recommendation 7: Add a `--debug` flag

The CLI has no verbosity controls. Per the CLI skill standard, at minimum add `--debug`:

```rust
/// Enable debug logging (set RUST_LOG for finer control)
#[arg(long)]
debug: bool,
```

Then in `main()`:

```rust
color_eyre::install()?;
let cli = Cli::parse();
init_tracing(cli.debug);
```

This gives users `bf --debug file.toml --json` to see what's happening, and `RUST_LOG=biscuit_file=trace bf file.toml` for full detail.

### Recommendation 8: Add debug events for CLI routing decisions

In `main()`, after input format detection and before processing:

```rust
debug!(
    ?input_format,
    ?format,
    compact,
    from_stdin,
    file = ?cli.file,
    "processing input"
);
```

In `run_reference()`:

```rust
debug!(?reference, relative, relative_cwd, vault_count = vaults.len(), "resolving reference");
```

### Recommendation 9: Missing CLI output flags

Per the CLI skill standard, the following are expected but absent:

| Flag | Status | Priority |
|------|--------|----------|
| `--debug` | Missing | **High** — needed for diagnosability |
| `--verbose` / `-v` | Missing | Medium — could show conversion metadata |
| `--quiet` / `-q` | Missing | Low — CLI output is already minimal |
| `--plain` | Missing | Medium — strip ANSI for piped output |

The `--json` and `--plain` flags mentioned as "mandatory" in the CLI skill don't apply in the same way here since `bf` doesn't produce styled terminal output — its output is always the converted data. However, `--debug` is essential.

---

## Priority Summary

| Priority | Item | Impact |
|----------|------|--------|
| **P0** | Add `tracing` dep to lib, `tracing-subscriber` to CLI | Prerequisite for everything |
| **P0** | `init_tracing()` + `--debug` flag in CLI | Users currently have no way to diagnose failures |
| **P1** | Instrument file reference resolution | Most complex module, most likely to fail in unexpected ways |
| **P1** | PDF backend warn on silent bookmark failures | Silently swallowed errors |
| **P2** | `#[instrument]` on all public constructors/conversions | Visibility into format operations |
| **P2** | File type detection trace events | Common source of user confusion |
| **P3** | Conversion result metrics | Performance diagnostics |

---

## Monorepo Tracing Convention Reference

For consistency, follow these established patterns:

- **Libraries emit, apps configure** — lib uses `tracing::{debug, info, warn, trace, instrument}`, CLI sets up the subscriber
- **RUST_LOG always works** — even without `--debug`, `RUST_LOG=biscuit_file=trace` should produce output
- **Debug output goes to stderr** — `.with_writer(std::io::stderr)` (never pollute stdout)
- **`#[instrument]` with `skip` for large data** — skip `self`, `bytes`, `content`; record `path`, `len`, variant names
- **`debug!` for operation outcomes** — what was found, sizes, counts
- **`trace!` for iteration/probing** — individual candidates checked, files walked
- **`warn!` for degraded paths** — fallbacks, silent failures, unexpected but recoverable states
