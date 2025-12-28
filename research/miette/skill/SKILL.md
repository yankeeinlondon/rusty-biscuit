## --- FILE: SKILL.md ---

## name: miette
description: Use when building Rust CLIs/parsers/linters that need user-facing, source-span-aware, pretty diagnostics (codes, help text, URLs, labels), including converting/wrapping standard errors into rich reports and choosing terminal/JSON rendering.

# Miette (Rust diagnostics) — Claude Code Skill

## Activate this skill when the user wants

* Pretty, user-facing error reports in Rust (CLI tools, parsers, config validation).
* Errors with **spans/labels** highlighting source snippets.
* A migration from `anyhow`/plain `Result` to `miette`.
* JSON/LSP-style diagnostics for CI/editor tooling.
* Help on **gotchas**: missing `fancy` feature, missing source text, span bounds, mixing `anyhow`.

## Quick decision tree

* Need **source snippets + labels** → `miette` (often with `thiserror` derive).
* Need app backtraces / general context, no snippets → consider `eyre`/`color-eyre`.
* Need ultra-flexible manual rendering → consider `ariadne`.
* Small/internal library errors → prefer `thiserror` only; convert to `miette` at boundaries.

## Core patterns (copy/paste)

### 1) CLI boundary: return `miette::Result` and convert IO errors

````rust
use miette::{IntoDiagnostic, Result, WrapErr};

fn main() -> Result<()> {
    let text = std::fs::read_to_string("config.toml")
        .into_diagnostic()
        .wrap_err("Failed to load configuration file")?;
    println!("{text}");
    Ok(())
}
````

### 2) Define a rich diagnostic with spans + help + code

````rust
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("Invalid syntax")]
#[diagnostic(code(my_app::syntax_error), help("Check for a missing ';'"))]
struct ParseError {
    #[source_code]
    src: NamedSource<String>,
    #[label("unexpected token here")]
    span: SourceSpan,
}
````

### 3) Attach related diagnostics for extra context

````rust
#[derive(Debug, Error, Diagnostic)]
#[error("Syntax error")]
#[diagnostic(code(my_parser::syntax_error))]
struct MainErr {
    #[source_code] src: String,
    #[label("problem here")] span: SourceSpan,
    #[related] related: Vec<HelpErr>,
}

#[derive(Debug, Error, Diagnostic)]
#[error("expected a semicolon")]
#[diagnostic(help("add ';' at the end of the expression"))]
struct HelpErr;
````

## Must-do setup

* Enable fancy terminal rendering:

````toml
[dependencies]
miette = { version = "7", features = ["fancy"] }
thiserror = "2"
````

## Gotchas (fast checks)

* Output looks “plain” → `fancy` feature likely off. See: [Gotchas](gotchas.md).
* “Source code not available” → you didn’t provide `#[source_code]` text (or spans out of bounds).
* Mixing `anyhow` and `miette` → explicitly `.into_diagnostic()` at boundaries.

## Handlers (rendering targets)

* Terminal pretty output: `GraphicalHandler` (via `fancy`)
* Logs/CI plain: `NarratableHandler`
* CI/editor tooling: `JSONReportHandler`

Details: [Rendering & Handlers](rendering.md)

## Deep dives

* [Derive diagnostics & spans](derive-diagnostics.md)
* [Wrapping existing errors (anyhow-style)](wrapping.md)
* [Integration: clap / nom / config validation](integrations.md)
* [When to use vs not](when-to-use.md)
* [Gotchas & fixes](gotchas.md)

---

--- FILE: derive-diagnostics.md ---

# Derive diagnostics & spans (miette + thiserror)

## Goal

Create error types that:

* implement `std::error::Error` (via `thiserror`)
* implement `miette::Diagnostic` (rich metadata for rendering)
* optionally include **source text** and **highlight spans**

## Recommended pattern

Use both derives:

````rust
use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("...")]
struct MyError { /* fields */ }
````

## Metadata you should consider

* `#[diagnostic(code(...))]` stable identifier for docs/search
* `#[diagnostic(help("..."))]` actionable suggestion
* `#[diagnostic(url("..."))]` link to docs for that code
* Severity is supported by the trait; use it when you have warnings/advice.

Example:

````rust
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("Invalid configuration")]
#[diagnostic(
    code(my_app::invalid_config),
    help("Fix the highlighted field and re-run."),
    url("https://docs.example.com/errors#invalid_config")
)]
pub struct ConfigDiag {
    #[source_code]
    pub src: NamedSource<String>,

    #[label("this value is not allowed")]
    pub span: SourceSpan,
}
````

## Spans: byte offsets, not char indices

`SourceSpan` is `(offset, len)` in **bytes**. Safer approaches:

* Use parser-provided offsets.
* If you compute offsets yourself, ensure you’re counting bytes (`.as_bytes()`), not Unicode scalar values.

Constructors:

````rust
use miette::SourceSpan;

let s1: SourceSpan = (10usize, 3usize).into();
let s2: SourceSpan = SourceSpan::from(10..13);
````

## Multiple highlights

Add multiple `#[label]` fields to a single diagnostic:

````rust
#[derive(Debug, Error, Diagnostic)]
#[error("Type mismatch")]
struct TypeMismatch {
    #[source_code] src: String,
    #[label("expected i32 here")] expected: SourceSpan,
    #[label("found string here")] found: SourceSpan,
}
````

## Related diagnostics (secondary notes)

Use `#[related]` for “also relevant” diagnostics:

````rust
#[related]
related: Vec<OtherDiag>,
````

This is great for “primary error + suggestions + follow-ups” without losing structure.

---

--- FILE: wrapping.md ---

# Wrapping existing errors (IntoDiagnostic / WrapErr)

## Goal

Propagate errors with `?` while getting miette reports automatically.

## Use `miette::Result` at the boundary

In binaries/CLIs:

````rust
pub type Result<T> = miette::Result<T>;
````

## Convert “normal” errors into diagnostics

* `IntoDiagnostic` turns `std::io::Error` (and many others) into `miette::Report`.
* `WrapErr` adds context text (like `anyhow::Context`).

````rust
use miette::{IntoDiagnostic, Result, WrapErr};

fn read_cfg(path: &std::path::Path) -> Result<String> {
    std::fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to read {}", path.display()))
}
````

## `miette::Report` as an adapter

`miette::Report` wraps any `std::error::Error` and exposes `Diagnostic` data where possible.
Use when you don’t want to define a struct:

````rust
fn do_work() -> miette::Result<()> {
    let _ = std::fs::read_to_string("missing.txt").into_diagnostic()?;
    Ok(())
}
````

## Library guidance

Libraries should usually avoid forcing miette on downstream users.
Common compromise:

* Use `thiserror` internally
* Convert to `miette` **in the binary** / UX boundary
* Or expose an opt-in feature for `miette`

If you do use miette in a library, consider:

````rust
pub type Result<T, E = miette::Report> = core::result::Result<T, E>;
````

## Mixing with `anyhow`

If you already have `anyhow::Result` somewhere:

* Convert at the boundary: `anyhow_err.into()` is not enough for spans.
* Prefer not to mix in the same module to avoid type ambiguity.
* Use `.into_diagnostic()` for dependency errors wherever possible.

---

--- FILE: rendering.md ---

# Rendering & handlers (terminal / plain / JSON)

## Goal

Choose output format based on environment (interactive terminal vs CI vs tooling).

## Terminal “fancy” output

Requires:

````toml
miette = { version = "7", features = ["fancy"] }
````

Then printing `{:?}` for a `miette::Report` typically yields the graphical report in supported terminals.

## Plain text (logs/CI)

Use a non-graphical handler when ANSI/Unicode is undesirable.
Conceptually:

* `NarratableHandler`: plain narrative format

If you need deterministic output in logs, render explicitly with a handler rather than relying on terminal detection.

## JSON output (CI/editor integration)

Use `JSONReportHandler` to emit machine-readable diagnostics (LSP-friendly).

Pattern:

````rust
use miette::{Diagnostic, JSONReportHandler};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("Linter violation")]
#[diagnostic(code(lint::no_todo))]
struct LintError;

fn main() {
    let err = LintError;

    if std::env::var("CI").is_ok() {
        let mut out = String::new();
        JSONReportHandler::new().render_report(&mut out, &err).unwrap();
        println!("{out}");
    } else {
        println!("{:?}", miette::Report::new(err));
    }
}
````

## Practical guidance

* CLI intended for humans → graphical handler (fancy)
* CI annotations, editor/LSP, dashboards → JSON
* Syslog / constrained terminals → narrative/plain

---

--- FILE: integrations.md ---

# Integration patterns (clap / nom / config validation)

## clap (CLI args)

Typical pattern: return `miette::Result<()>` from `main`, and convert OS errors.

````rust
use clap::Parser;
use miette::{IntoDiagnostic, Result};

#[derive(Parser)]
struct Cli {
    path: std::path::PathBuf,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let _content = std::fs::read_to_string(args.path).into_diagnostic()?;
    Ok(())
}
````

## nom (or any parser): turn offsets into `SourceSpan`

Parsers often provide an error position; use that as byte offset.

````rust
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("Invalid syntax")]
struct ParseError {
    #[source_code] src: NamedSource<String>,
    #[label("expected a semicolon here")] span: SourceSpan,
}

fn fail_at(input: &str, offset: usize) -> miette::Result<()> {
    let len = 1;
    Err(ParseError {
        src: NamedSource::new("input.txt", input.to_string()),
        span: (offset, len).into(),
    })?
}
````

### Tip: spans must be in-bounds

If `offset + len > input.len()` you’ll get confusing “source not available” style output or missing highlights.

## Config validation (TOML/YAML/JSON)

Most parsers can provide location info; if not, you can still provide:

* `help` text
* `code` and `url`
* labels if you can compute spans for the key/value

Sketch:

````rust
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Invalid configuration value")]
#[diagnostic(code(config::invalid_port), help("Port must be 1024..=65535"))]
struct BadPort {
    #[source_code] src: miette::NamedSource<String>,
    #[label("reserved port")] span: miette::SourceSpan,
}
````

---

--- FILE: gotchas.md ---

# Gotchas & fixes

## 1) “Fancy” output not showing

**Symptom:** output looks like plain `Debug` without frames/colors/snippets.

**Fix:** enable the `fancy` feature.

````toml
miette = { version = "7", features = ["fancy"] }
````

## 2) “Source code not available” / “Unable to determine source span”

Common causes:

1. You didn’t include source text in the diagnostic.
   * Add a field annotated with `#[source_code]` (often `NamedSource<String>`).
1. Your `SourceSpan` is out of bounds.
   * Ensure `offset + len <= src.len()` (bytes).
1. You computed spans using character indices rather than byte offsets.
   * Use byte offsets (parser positions are usually bytes).

## 3) Mixing `anyhow` and `miette`

**Symptom:** errors print, but you don’t get diagnostic richness and conversions are awkward.

**Fix:**

* Prefer `miette::Result` in CLIs and `.into_diagnostic()` on dependency errors.
* Avoid using `anyhow::Result` and `miette::Result` in the same module unless you must.
* Convert at boundaries explicitly.

## 4) Performance / size overhead

**Symptom:** concern about heavy formatting/deps (especially `fancy`).

**Fix / guidance:**

* Use `miette` for **user-facing** errors (CLI/parsers).
* Use lightweight errors (`thiserror`) internally; convert to `miette` at the UX boundary.
* Avoid constructing diagnostics in hot loops.

---

--- FILE: when-to-use.md ---

# When to use miette (and when not)

## Great fit

* Compilers, parsers, linters, DSLs: precise source spans + labels are core UX.
* Complex CLIs: validation failures benefit from codes/help/URLs.
* CI/build automation: JSON output enables tooling integration.
* DX-focused libraries (carefully): if you intentionally invest in user-facing diagnostics.

## Not a great fit

* Small/internal libraries where users won’t see formatted reports.
* Embedded/no-std or constrained environments (alloc/formatting heavy).
* Strict binary-size constraints (especially with `fancy`).
* Performance-critical paths with frequent errors.

## Alternatives (quick mapping)

* Want beautiful manual diagnostics only → `ariadne`
* Want rustc-style output with file DB → `codespan-reporting` / `annotate-snippets`
* Want app-level report + backtrace, not snippets → `eyre` / `color-eyre`
* Want lightweight custom error enums → `thiserror` alone