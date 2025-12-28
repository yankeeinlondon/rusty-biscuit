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