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