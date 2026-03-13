//! biscuit-file CLI - File format conversion utility.
//!
//! Convert between TOML, YAML, JSON, and extract text/markdown from PDFs.
//! For Markdown files, extracts and converts the frontmatter block.

use biscuit_file::json5::{to_json5_compact, to_json5_pretty};
use biscuit_file::{FileReference, FileType, Json5, Pdf, Toml, Yaml, detect_file_type};
use clap::{ArgGroup, Parser, Subcommand, ValueEnum};
use color_eyre::eyre::{Result, WrapErr, bail};
use std::io::{IsTerminal, Read};
use std::path::PathBuf;
use std::process;

/// File format conversion and extraction utility.
///
/// Convert TOML/YAML/JSON files between formats, extract text/markdown from PDFs,
/// and extract frontmatter from Markdown files. Omit the file path or use `-` to
/// read from STDIN (requires --input-format).
///
/// Output format flags are mutually exclusive — specify at most one.
#[derive(Parser)]
#[command(name = "bf", version, about, long_about = None)]
#[command(group(
    ArgGroup::new("output_format")
        .args(["json", "json5", "yaml", "toml", "md", "text"])
))]
struct Cli {
    /// Input file path (omit or use `-` for STDIN)
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Output as JSON5
    #[arg(long)]
    json5: bool,

    /// Output as YAML
    #[arg(long)]
    yaml: bool,

    /// Output as TOML
    #[arg(long)]
    toml: bool,

    /// Output as Markdown (for PDFs)
    #[arg(long)]
    md: bool,

    /// Output as plain text (for PDFs)
    #[arg(long)]
    text: bool,

    /// Compact single-line output (JSON and JSON5 only)
    #[arg(long)]
    compact: bool,

    /// Force input format (override auto-detection, required for STDIN)
    #[arg(long)]
    input_format: Option<InputFormat>,
}

/// Resolved output format.
#[derive(Clone, Copy, Debug)]
enum OutputFormat {
    Json,
    Json5,
    Yaml,
    Toml,
    Text,
    Markdown,
}

impl Cli {
    fn output_format(&self) -> Option<OutputFormat> {
        if self.json {
            Some(OutputFormat::Json)
        } else if self.json5 {
            Some(OutputFormat::Json5)
        } else if self.yaml {
            Some(OutputFormat::Yaml)
        } else if self.toml {
            Some(OutputFormat::Toml)
        } else if self.md {
            Some(OutputFormat::Markdown)
        } else if self.text {
            Some(OutputFormat::Text)
        } else {
            None
        }
    }

    fn is_stdin(&self) -> bool {
        match &self.file {
            None => true,
            Some(p) => p.as_os_str() == "-",
        }
    }
}

/// Supported input formats.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum InputFormat {
    /// TOML input
    Toml,
    /// YAML input
    Yaml,
    /// JSON input
    Json,
    /// JSON5 input
    Json5,
    /// Markdown input (extracts frontmatter)
    Markdown,
    /// PDF input
    Pdf,
}

#[derive(Subcommand)]
enum Commands {
    /// Resolve a file reference to its filesystem path
    #[command(alias = "ref")]
    Reference {
        /// The file reference string (e.g., @foo.md, !README.md, vault:note.md)
        reference: String,

        /// Output a path relative to the matched base directory
        #[arg(long)]
        relative: bool,

        /// Output a path relative to the current working directory
        #[arg(long)]
        relative_cwd: bool,

        /// Add a vault root path
        #[arg(long = "add-vault", short = 'v')]
        vaults: Vec<PathBuf>,
    },
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Handle subcommands first
    if let Some(Commands::Reference {
        reference,
        relative,
        relative_cwd,
        vaults,
    }) = cli.command
    {
        return run_reference(&reference, relative, relative_cwd, &vaults);
    }

    // No file argument and STDIN is a terminal (not piped) → show help
    if cli.file.is_none() && std::io::stdin().is_terminal() {
        Cli::parse_from(["bf", "--help"]);
    }

    let from_stdin = cli.is_stdin();

    // Detect input format
    let input_format = if let Some(fmt) = cli.input_format {
        match fmt {
            InputFormat::Toml => FileType::Toml,
            InputFormat::Yaml => FileType::Yaml,
            InputFormat::Json => FileType::Json,
            InputFormat::Json5 => FileType::Json5,
            InputFormat::Markdown => FileType::Markdown,
            InputFormat::Pdf => FileType::Pdf,
        }
    } else if from_stdin {
        bail!("--input-format is required when reading from STDIN");
    } else {
        detect_file_type(cli.file.as_ref().unwrap()).wrap_err("Failed to detect file type")?
    };

    let format = cli.output_format();
    let compact = cli.compact;

    // Read input content
    let content = if from_stdin {
        read_stdin()?
    } else {
        std::fs::read(cli.file.as_ref().unwrap()).wrap_err("Failed to read input file")?
    };

    match input_format {
        FileType::Toml => process_toml(&content, format, compact)?,
        FileType::Yaml => process_yaml(&content, format, compact)?,
        FileType::Json => process_json(&content, format, compact)?,
        FileType::Json5 => process_json5(&content, format, compact)?,
        FileType::Markdown => process_markdown(&content, format, compact)?,
        FileType::Pdf => process_pdf(&content, format, compact)?,
        FileType::Unknown => {
            bail!(
                "Unknown file type. Use --input-format to specify the format, \
                 or ensure the file has a recognized extension \
                 (.toml, .yaml, .yml, .json, .json5, .md, .markdown, .mdx, .pdf)"
            );
        }
    }

    Ok(())
}

fn read_stdin() -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buf)
        .wrap_err("Failed to read from STDIN")?;
    Ok(buf)
}

/// Frontmatter format detected from delimiters.
#[derive(Clone, Copy, Debug)]
enum FrontmatterFormat {
    /// YAML frontmatter delimited by `---`
    Yaml,
    /// TOML frontmatter delimited by `+++`
    Toml,
}

/// Extract frontmatter from a markdown document.
///
/// Supports YAML frontmatter (`---`) and TOML frontmatter (`+++`).
/// Returns the raw frontmatter content (without delimiters) and its format.
fn extract_frontmatter(input: &str) -> Result<(&str, FrontmatterFormat)> {
    let trimmed = input.trim_start();

    let (delimiter, fmt) = if trimmed.starts_with("---") {
        ("---", FrontmatterFormat::Yaml)
    } else if trimmed.starts_with("+++") {
        ("+++", FrontmatterFormat::Toml)
    } else {
        bail!(
            "No frontmatter found. Expected `---` (YAML) or `+++` (TOML) at the start of the document"
        );
    };

    // Find the closing delimiter (skip the opening line)
    let after_open = trimmed
        .get(delimiter.len()..)
        .and_then(|s| s.find('\n').map(|i| delimiter.len() + i + 1))
        .unwrap_or(delimiter.len());

    let rest = &trimmed[after_open..];
    let close_pos = rest.find(delimiter).ok_or_else(|| {
        color_eyre::eyre::eyre!("Unclosed frontmatter. Expected closing `{delimiter}` delimiter")
    })?;

    let frontmatter = &rest[..close_pos];
    Ok((frontmatter, fmt))
}

/// Format a JSON value as either compact or pretty-printed JSON.
fn format_json(value: &serde_json::Value, compact: bool) -> Result<String> {
    if compact {
        serde_json::to_string(value).wrap_err("Failed to serialize JSON")
    } else {
        serde_json::to_string_pretty(value).wrap_err("Failed to serialize JSON")
    }
}

/// Format a JSON value as either compact or pretty-printed JSON5.
fn format_json5(value: &serde_json::Value, compact: bool) -> String {
    if compact {
        to_json5_compact(value)
    } else {
        to_json5_pretty(value)
    }
}

/// Process TOML content.
fn process_toml(content: &[u8], format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let input = std::str::from_utf8(content).wrap_err("TOML input is not valid UTF-8")?;
    let toml = Toml::from_str(input).wrap_err("Failed to parse TOML")?;

    let output = match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => {
            let json_value = toml.as_json_value().wrap_err("Failed to convert to JSON")?;
            format_json(&json_value, compact)?
        }
        OutputFormat::Json5 => {
            let json_value = toml.as_json_value().wrap_err("Failed to convert to JSON")?;
            format_json5(&json_value, compact)
        }
        OutputFormat::Yaml => toml.as_yaml().wrap_err("Failed to convert to YAML")?,
        OutputFormat::Toml => toml.raw().to_string(),
        OutputFormat::Text | OutputFormat::Markdown => {
            bail!("--text and --md are only supported for PDF files");
        }
    };

    println!("{output}");
    Ok(())
}

/// Process YAML content.
fn process_yaml(content: &[u8], format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let yaml = Yaml::from_bytes(content).wrap_err("Failed to parse YAML")?;

    let output = match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => {
            let value = yaml.as_json().wrap_err("Failed to convert to JSON")?;
            format_json(&value, compact)?
        }
        OutputFormat::Json5 => {
            let value = yaml.as_json().wrap_err("Failed to convert to JSON")?;
            format_json5(&value, compact)
        }
        OutputFormat::Yaml => {
            serde_yaml_ng::to_string(yaml.value()).wrap_err("Failed to serialize YAML")?
        }
        OutputFormat::Toml => {
            let toml_value = yaml.as_toml().wrap_err("Failed to convert to TOML")?;
            toml::to_string_pretty(&toml_value).wrap_err("Failed to serialize TOML")?
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            bail!("--text and --md are only supported for PDF files");
        }
    };

    println!("{output}");
    Ok(())
}

/// Process JSON content.
fn process_json(content: &[u8], format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let value: serde_json::Value =
        serde_json::from_slice(content).wrap_err("Failed to parse JSON")?;

    let output = match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => format_json(&value, compact)?,
        OutputFormat::Json5 => format_json5(&value, compact),
        OutputFormat::Yaml => {
            serde_yaml_ng::to_string(&value).wrap_err("Failed to convert to YAML")?
        }
        OutputFormat::Toml => {
            let toml_value: toml::Value = serde_json::from_value(
                serde_json::to_value(&value).wrap_err("Failed to convert JSON value")?,
            )
            .wrap_err("Failed to convert to TOML (JSON may contain types unsupported by TOML)")?;
            toml::to_string_pretty(&toml_value).wrap_err("Failed to serialize TOML")?
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            bail!("--text and --md are only supported for PDF files");
        }
    };

    println!("{output}");
    Ok(())
}

/// Process JSON5 content.
fn process_json5(content: &[u8], format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let input = std::str::from_utf8(content).wrap_err("JSON5 input is not valid UTF-8")?;
    let json5 = Json5::from_str(input).wrap_err("Failed to parse JSON5")?;

    let output = match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => {
            if compact {
                json5
                    .as_json_compact()
                    .wrap_err("Failed to convert to JSON")?
            } else {
                json5.as_json().wrap_err("Failed to convert to JSON")?
            }
        }
        OutputFormat::Json5 => {
            if compact {
                json5.as_json5_compact()
            } else {
                json5.as_json5()
            }
        }
        OutputFormat::Yaml => json5.as_yaml().wrap_err("Failed to convert to YAML")?,
        OutputFormat::Toml => json5.as_toml().wrap_err("Failed to convert to TOML")?,
        OutputFormat::Text | OutputFormat::Markdown => {
            bail!("--text and --md are only supported for PDF files");
        }
    };

    println!("{output}");
    Ok(())
}

/// Process Markdown content by extracting frontmatter and converting it.
fn process_markdown(content: &[u8], format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let input = std::str::from_utf8(content).wrap_err("Markdown input is not valid UTF-8")?;
    let (frontmatter, fm_format) = extract_frontmatter(input)?;

    match fm_format {
        FrontmatterFormat::Yaml => process_yaml(frontmatter.as_bytes(), format, compact),
        FrontmatterFormat::Toml => process_toml(frontmatter.as_bytes(), format, compact),
    }
}

/// Resolve a file reference and print the result.
///
/// Exit codes: 0 = found, 1 = not found, 2 = error.
fn run_reference(
    reference: &str,
    relative: bool,
    relative_cwd: bool,
    vaults: &[PathBuf],
) -> Result<()> {
    let file_ref = match FileReference::new(reference) {
        Ok(fr) => fr,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(2);
        }
    };

    let file_ref = vaults
        .iter()
        .fold(file_ref, |fr, vault| fr.add_vault(vault));

    let result = if relative_cwd {
        file_ref.resolve_relative(None)
    } else if relative {
        // --relative: resolve, then compute path relative to the reference's scope root.
        // Since the scope root isn't directly exposed, we approximate by resolving
        // the absolute path and stripping the filename's directory contribution.
        file_ref.resolve_relative(None)
    } else {
        file_ref.resolve()
    };

    match result {
        Ok(Some(path)) => {
            println!("{}", path.display());
            Ok(())
        }
        Ok(None) => {
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(2);
        }
    }
}

/// Process PDF content.
fn process_pdf(content: &[u8], format: Option<OutputFormat>, _compact: bool) -> Result<()> {
    let pdf = Pdf::from_bytes(content.to_vec()).wrap_err("Failed to parse PDF")?;

    let output = match format.unwrap_or(OutputFormat::Text) {
        OutputFormat::Text => pdf.as_text().wrap_err("Failed to extract text from PDF")?,
        OutputFormat::Markdown => {
            let md = pdf
                .as_markdown(Default::default())
                .wrap_err("Failed to convert PDF to Markdown")?;
            md.content
        }
        OutputFormat::Json | OutputFormat::Json5 | OutputFormat::Yaml | OutputFormat::Toml => {
            bail!("--json, --json5, --yaml, and --toml are only supported for data file inputs");
        }
    };

    println!("{output}");
    Ok(())
}
