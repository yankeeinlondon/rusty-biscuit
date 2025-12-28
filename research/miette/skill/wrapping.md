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