# miette (Rust) — A Deep Dive into Beautiful Diagnostics

*miette* is a Rust diagnostic library designed to produce “compiler-grade” error reports: richly structured diagnostics (codes, severity, help text, URLs, labeled source spans) rendered into human-friendly terminal output or machine-friendly JSON. It was created by Jane Lusby (Yosh) and is heavily inspired by the diagnostic experience of Rust itself and TypeScript.

This document consolidates the provided research into a single, end-to-end reference: from basic adoption to advanced rendering, source management, integration patterns, gotchas, and tradeoffs.

---

## Table of Contents

1. [What miette Is (and Isn’t)](#what-miette-is-and-isnt)
1. [Mental Model: The Four Pillars](#mental-model-the-four-pillars)
   1. [The Diagnostic Protocol (`Diagnostic`)](#1-the-diagnostic-protocol-diagnostic)
   1. [Error Wrapping (`miette::Report`)](#2-error-wrapping-miettereport)
   1. [Source Management (`SourceCode`, `NamedSource`, `SourceSpan`)](#3-source-management-sourcecode-namedsource-sourcespan)
   1. [Handlers & Rendering (Graphical / Narrated / JSON)](#4-handlers--rendering-graphical--narrated--json)
1. [Quick Start: The “Hello World” Diagnostic](#quick-start-the-hello-world-diagnostic)
1. [Core Concepts in Practice](#core-concepts-in-practice)
   1. [Severity, Codes, Help, URLs](#severity-codes-help-urls)
   1. [Labels and Spans](#labels-and-spans)
   1. [Chaining and “Related” Diagnostics](#chaining-and-related-diagnostics)
1. [Anyhow-style Dynamic Wrapping](#anyhow-style-dynamic-wrapping)
1. [Source Snippets Done Right](#source-snippets-done-right)
1. [Integrations](#integrations)
   1. [`thiserror`](#thiserror)
   1. [`clap` for CLI apps](#clap-for-cli-apps)
   1. Parsers like `nom` (and friends like `pest`)\](#parsers-like-nom-and-friends-like-pest)
1. [Rendering Modes: Terminal vs CI vs Tooling](#rendering-modes-terminal-vs-ci-vs-tooling)
1. [Gotchas, Limitations, and Practical Fixes](#gotchas-limitations-and-practical-fixes)
1. [Performance, Binary Size, and When *Not* to Use miette](#performance-binary-size-and-when-not-to-use-miette)
1. [Ecosystem & Alternatives](#ecosystem--alternatives)
1. [Licensing](#licensing)

---

## What miette Is (and Isn’t)

**miette is for user-facing diagnostics.** It’s especially well-suited to tools where users need to understand *exactly where and why* something failed—often in source-like inputs (DSLs, configs, templates, scripts).

* **miette is not just error propagation.** While it *does* support dynamic wrapping like `anyhow`/`eyre`, its distinguishing value is structured diagnostic metadata and source code rendering.
* **miette is not ideal for tiny libraries or hot paths.** The diagnostic metadata, span management, and rendering logic can add overhead and dependencies, particularly with “fancy” output enabled.

---

## Mental Model: The Four Pillars

### 1) The Diagnostic Protocol (`Diagnostic`)

Rust’s `std::error::Error` gives you:

* a message (`Display`)
* a causal chain (`source()`)

miette adds a richer protocol via the `Diagnostic` trait so renderers can produce compiler-style reports. Key metadata includes:

* **Severity**: error vs warning vs advice
* **Code**: a stable identifier like `my_app::bad_input` or `E0001`
* **Help**: “Try X…” fix guidance
* **URL**: documentation link for the diagnostic
* **Labels**: annotated spans pointing to exact locations in source input

This is the foundation that makes “pretty printing” possible.

---

### 2) Error Wrapping (`miette::Report`)

`miette::Report` is the “dynamic report” type (conceptually similar to `Box<dyn Error>` or `anyhow::Report`) that integrates with Rust’s `?` operator for ergonomic propagation.

Important behavior:

* It can wrap *any* `std::error::Error`.
* It implements `Diagnostic` for wrapped errors (via a blanket implementation), so even ordinary errors become printable as diagnostic reports (with at least message + chain).

This makes miette practical as a boundary-layer error type: convert to `Report` at the CLI/app edge and render nicely.

---

### 3) Source Management (`SourceCode`, `NamedSource`, `SourceSpan`)

To render highlighted snippets, miette needs access to the original input text.

* **`SourceSpan`** identifies a region with a **byte offset + length**.
  * Example: `(offset, len).into()` or `SourceSpan::from(4..5)`
* **`SourceCode` trait** abstracts “something that can provide source text.”
* **`NamedSource`** is a common wrapper that pairs source text with a filename (or logical name), useful for reports and multi-file tooling.

This separation is intentional: miette does **not** implicitly read files or maintain global source databases by default. You provide the source (or configure rendering appropriately).

---

### 4) Handlers & Rendering (Graphical / Narrated / JSON)

miette separates “what the diagnostic *is*” from “how to display it.”

Common handlers described in the research:

* **Graphical (fancy terminal)**: colors, underlines, arrows, Unicode frames
  * Requires enabling the `fancy` feature.
* **Narratable (plain text)**: useful for logs, minimal terminals, or CI where ANSI is undesirable
* **JSON**: machine-readable output (notably useful for CI systems and tooling; referenced as LSP-compliant in the research)

This handler abstraction is what lets you present the same diagnostics differently depending on environment.

---

## Quick Start: The “Hello World” Diagnostic

This example shows the most common approach: define a typed error using `thiserror` for `Error`/`Display`, and derive `miette::Diagnostic` to attach diagnostic metadata and spans.

 > 
 > **Note:** The “fancy” terminal output is off by default—see [Gotchas](#gotchas-limitations-and-practical-fixes).

````rust
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[error("oops!")]
#[diagnostic(
    code(my_app::bad_input),
    help("try checking your input again")
)]
struct MyBadInput {
    #[source_code]
    src: String,

    #[label("This bit here is bad")]
    bad_bit: SourceSpan,
}

fn main() {
    let src = "source text goes here".to_string();
    let err = MyBadInput {
        src,
        bad_bit: SourceSpan::from(4..5),
    };

    // Printing the Report uses miette's diagnostic formatting (esp. with fancy handler enabled).
    println!("{:?}", miette::Report::new(err));
}
````

What you get conceptually:

* A message (`oops!`)
* A stable code (`my_app::bad_input`)
* Help text
* A labeled highlight for the span

---

## Core Concepts in Practice

### Severity, Codes, Help, URLs

These fields exist to improve user experience and to make errors searchable and documentable.

Example (a CLI/network-oriented diagnostic):

````rust
use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("Failed to connect to the production database")]
#[diagnostic(
    code(cli::db_connection_timeout),
    url("https://docs.my-app.com/errors/db-timeout"),
    help("Check if your VPN is active or if the database credentials in .env are correct.")
)]
struct ConnectionError;

fn run_app() -> miette::Result<()> {
    Err(ConnectionError)?
}
````

Practical payoff:

* Users can paste the **code** into a search box.
* The **URL** can go straight to docs.
* **Help** reduces support burden.

---

### Labels and Spans

Labels are where miette starts feeling like a compiler: you’re not just telling users “bad input,” you’re pointing to the *exact characters*.

Key detail: **spans are byte-based**, not char-based—be mindful with Unicode input.

Example (parser/config-style):

````rust
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("Invalid syntax")]
struct ParseError {
    #[source_code]
    src: NamedSource<String>,

    #[label("Expected a semicolon here")]
    bad_bit: SourceSpan,
}

fn parse_stuff(input: &str) -> miette::Result<()> {
    let offset = 10;
    let len = 1;

    Err(ParseError {
        src: NamedSource::new("config.txt", input.to_string()),
        bad_bit: (offset, len).into(),
    })?
}
````

---

### Chaining and “Related” Diagnostics

miette supports attaching **related diagnostics** to one parent diagnostic. This is useful when you want to present “primary failure” plus contextual hints, secondary failures, or follow-on notes.

````rust
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[error("Syntax Error")]
#[diagnostic(code(my_parser::syntax_error))]
struct ParseError {
    #[source_code]
    src: String,

    #[label("unexpected token")]
    span: SourceSpan,

    #[related]
    related: Vec<MietteHelper>,
}

#[derive(Debug, Diagnostic, Error)]
#[error("expected a semicolon here")]
#[diagnostic(help("add a ';' at the end of the expression"))]
struct MietteHelper;

fn main() {
    let src = "let x = 5".to_string();

    let err = ParseError {
        src,
        span: SourceSpan::from(8..9),
        related: vec![MietteHelper.into()],
    };

    println!("{:?}", miette::Report::new(err));
}
````

Use this pattern when:

* you want multiple actionable items
* an error depends on a prior condition (template engines, DSL evaluation, multi-stage validation)

---

## Anyhow-style Dynamic Wrapping

You don’t always want bespoke structs. miette supports “convert and wrap” flows similar to `anyhow`/`eyre`:

* **`IntoDiagnostic`** converts standard errors into `miette::Report`
* **`WrapErr`** attaches context

````rust
use miette::{IntoDiagnostic, WrapErr, Result};
use std::fs;

fn read_config() -> Result<String> {
    fs::read_to_string("config.toml")
        .into_diagnostic()
        .wrap_err("Failed to load configuration file")
}

fn main() {
    match read_config() {
        Ok(s) => println!("{}", s),
        Err(e) => {
            println!("{:?}", e);
        }
    }
}
````

This is ideal when:

* the underlying error type isn’t yours (IO, HTTP, serde, etc.)
* you mostly need good context and a clean chain
* you don’t have spans/source for the underlying failure

---

## Source Snippets Done Right

To highlight code, miette needs two things:

1. A **span** (`SourceSpan`) identifying where to highlight
1. The **source text** available to the renderer via `#[source_code]` (or equivalent handler configuration)

Best practice for user-input parsing:

* Store the entire source in the diagnostic (`#[source_code]`)
* Use `NamedSource` if filenames matter (CLIs, multi-file tooling)
* Ensure your offsets/lengths are valid for the underlying string

---

## Integrations

### `thiserror`

miette is frequently used as “**thiserror + diagnostics**”:

* `thiserror` provides ergonomic `Error` + `Display` derivation.
* miette provides `Diagnostic` derivation and diagnostic metadata/spans.

````rust
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("Failed to parse config")]
#[diagnostic(
    code(my_app::config_error),
    help("Check if the config file format is valid JSON or TOML")
)]
pub struct ConfigError {
    #[label("this specific character is invalid")]
    pub src_span: SourceSpan,
}
````

---

### `clap` for CLI apps

A common pattern: return `miette::Result<()>` from `main` so failures naturally render through miette’s reporting.

````rust
use clap::Parser;
use miette::{IntoDiagnostic, Result};

#[derive(Parser)]
struct Cli {
    path: std::path::PathBuf,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let _content = std::fs::read_to_string(args.path)
        .into_diagnostic()?;

    Ok(())
}
````

This encourages a clean separation:

* deep logic returns structured errors (typed or wrapped)
* `main` formats them nicely for end users

---

### Parsers like `nom` (and friends like `pest`)

Parsers naturally produce offsets. miette’s `SourceSpan` fits this model: translate parser offsets into spans, and include the source input in the error so the renderer can show a snippet.

This is the “primary” miette use case: compilers, linters, DSLs, template engines.

---

## Rendering Modes: Terminal vs CI vs Tooling

miette can target different output needs:

* **GraphicalHandler**: best for local developer terminals; high readability with color/Unicode
* **NarratableHandler**: stable plain text for logs/CI
* **JSONHandler / JSONReportHandler**: machine-readable diagnostics (useful when CI systems or tools post-process failures)

Example pattern (toggle JSON in CI):

````rust
use miette::{Diagnostic, JSONReportHandler};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("Linter violation")]
#[diagnostic(code(lint::no_todo))]
struct LintError {
    #[label("TODOs are not allowed in the main branch")]
    span: miette::SourceSpan,
}

fn main() {
    let err = LintError { span: (0, 4).into() };

    if std::env::var("CI").is_ok() {
        let mut out = String::new();
        JSONReportHandler::new().render_report(&mut out, &err).unwrap();
        println!("{out}");
    } else {
        println!("{:?}", miette::Report::new(err));
    }
}
````

---

## Gotchas, Limitations, and Practical Fixes

### Gotcha 1: “Fancy” output is off by default

**Symptom:** You expected beautiful compiler-like output, but got plain/boring formatting.

**Fix:** Enable `fancy` in `Cargo.toml`:

````toml
[dependencies]
miette = { version = "7", features = ["fancy"] }
````

Optionally add syntax highlighting via additional features (the research mentions `syntect`).

---

### Gotcha 2: “Source code not available” / spans don’t render

**Common causes:**

* You provided spans, but the diagnostic doesn’t contain source text (no `#[source_code]`).
* The source exists, but the **span is out of bounds** (e.g., `10..20` on a 5-byte string).
* You wrapped an external error into `Report` without supplying source context, and still expected snippet highlighting.

**Fixes:**

* Ensure your diagnostic struct includes the original text (often via `#[source_code] src: String` or `NamedSource<String>`).
* Validate spans are within the source bounds (byte offsets!).
* For external errors where source context is separate, consider creating a higher-level diagnostic that includes the source and labels, and store the wrapped error as a cause/related item.

---

### Gotcha 3: Mixing `anyhow` and `miette`

**Symptom:** Your library returns `anyhow::Result`, but your binary expects miette-style reports; highlighting/diagnostics don’t appear automatically.

**Fix:** Convert errors with `.into_diagnostic()` at boundaries, and consider standardizing on a miette `Result` alias if you want miette end-to-end.

Suggested alias pattern:

````rust
pub type Result<T, E = miette::Report> = core::result::Result<T, E>;
````

Guidance:

* Avoid mixing `anyhow` and `miette` in the same module when possible to reduce type ambiguity and confusion.
* If you do mix, make conversions explicit at module or crate boundaries.

---

### Gotcha 4: Performance overhead

miette is heavier than “just an error” because diagnostics may involve:

* dynamic dispatch (`Report`)
* storing extra metadata
* formatting/rending work, including span-to-line/column mapping

**Fix / best practice:**

* Use miette for *user-facing* failures (CLI boundary, parsing/validation stages).
* Keep internal “hot loop” errors lightweight (`thiserror`, plain enums, etc.) and convert to miette at the boundary layer when you actually need to present the error.

---

## Performance, Binary Size, and When *Not* to Use miette

miette is a great fit when the user needs a high-quality explanation. It is *not* always the best choice:

**Good fit**

1. **Compilers / parsers / linters**: multi-line snippets, labeled spans, related diagnostics
1. **Complex CLI tools**: validation and multi-step workflows
1. **DX-focused libraries**: when you intentionally invest in user-facing errors

**Not a good fit**

1. **Simple/internal libraries**: forcing consumers into miette is often overkill
1. **Embedded / `no_std` / constrained environments**: heavy formatting and allocation assumptions
1. **Performance-critical paths**: repeated error construction/formatting is wasted work
1. **Strict binary size constraints**: “fancy” output pulls in terminal styling and related dependencies

---

## Ecosystem & Alternatives

miette is compelling because it combines:

* a diagnostic trait/protocol
* a dynamic report type
* high-quality rendering (terminal + JSON)

Depending on what you need, alternatives may fit better:

### Ariadne

* **Best for:** extremely beautiful and flexible diagnostics rendering
* **Tradeoff:** manual/imperative API; it’s a renderer more than an error-handling system  
  Repo: https://github.com/zesterer/ariadne  
  Docs: https://docs.rs/ariadne/latest/ariadne/

### codespan-reporting

* **Best for:** rustc-like rendering with a stable, long-standing approach
* **Tradeoff:** more boilerplate; maintenance slower; manual file database management  
  Repo: https://github.com/brendanzab/codespan  
  Docs: https://docs.rs/codespan-reporting/latest/codespan_reporting/

### eyre / color-eyre

* **Best for:** app-level error handling + backtraces + “suggestions”
* **Tradeoff:** doesn’t natively do source-span snippets like miette/ariadne  
  Repo: https://github.com/eyre-rs/eyre  
  Docs: https://docs.rs/eyre/latest/eyre/  
  Site: https://eyre.rs/

### annotate-snippets

* **Best for:** exact rustc snippet style (maintained by Rust project)
* **Tradeoff:** clunkier, data-heavy API; not “derive macro magical”  
  Repo: https://github.com/rust-lang/annotate-snippets-rs  
  Docs: https://docs.rs/annotate-snippets/latest/annotate_snippets/

### thiserror

* **Best for:** lightweight custom error types (no rendering)
* **Tradeoff:** no pretty diagnostics; you’d pair it with a renderer if needed  
  Repo: https://github.com/dtolnay/thiserror  
  Docs: https://docs.rs/thiserror/latest/thiserror/

#### Summary Table (from research)

|Feature|Miette|Ariadne|Codespan-reporting|Eyre/Color-eyre|
|-------|------|-------|------------------|---------------|
|Best For|All-in-one CLIs|Beautiful layout|Compilers|Apps/Services|
|Derive Macros|Yes|No|No|No|
|Source Snippets|Yes|Yes (Excellent)|Yes|No|
|Philosophy|“Magic” & Fast|Manual & Precise|Stable & Standard|Context & Backtrace|

---

## Licensing

miette is **dual-licensed** under:

* **Apache License 2.0**
* **MIT License**

This is a standard permissive Rust ecosystem licensing model suitable for commercial and open-source usage.

---

### Version Notes

* The research references `miette` **version 7** in the recommended `Cargo.toml` snippet:
  ````toml
  miette = { version = "7", features = ["fancy"] }
  ````

* The behavior that “fancy output is off by default” is a deliberate design choice and is central to first-run expectations.

---

If you want, I can add an “Architecture & Data Flow” diagram-style section (how `Diagnostic` metadata + `SourceCode` feed handlers), or provide a “recommended crate layout” for a real CLI (library defines typed diagnostics; binary selects handler and formats output).