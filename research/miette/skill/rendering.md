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