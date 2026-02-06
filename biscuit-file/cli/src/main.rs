//! biscuit-file CLI - File format conversion utility.
//!
//! Convert between TOML, YAML, JSON, and extract text/markdown from PDFs.
//! For Markdown files, extracts and converts the frontmatter block.

use biscuit_file::{detect_file_type, FileType, Pdf, Toml, Yaml};
use clap::{ArgGroup, Parser, ValueEnum};
use color_eyre::eyre::{bail, Result, WrapErr};
use std::io::Read;
use std::path::PathBuf;

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
        .args(["json", "yaml", "toml", "md", "text"])
))]
struct Cli {
    /// Input file path (omit or use `-` for STDIN)
    file: Option<PathBuf>,

    /// Output as JSON
    #[arg(long)]
    json: bool,

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

    /// Force input format (override auto-detection, required for STDIN)
    #[arg(long)]
    input_format: Option<InputFormat>,
}

/// Resolved output format.
#[derive(Clone, Copy, Debug)]
enum OutputFormat {
    Json,
    Yaml,
    Toml,
    Text,
    Markdown,
}

impl Cli {
    fn output_format(&self) -> Option<OutputFormat> {
        if self.json {
            Some(OutputFormat::Json)
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
    /// Markdown input (extracts frontmatter)
    Markdown,
    /// PDF input
    Pdf,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let from_stdin = cli.is_stdin();

    // Detect input format
    let input_format = if let Some(fmt) = cli.input_format {
        match fmt {
            InputFormat::Toml => FileType::Toml,
            InputFormat::Yaml => FileType::Yaml,
            InputFormat::Json => FileType::Json,
            InputFormat::Markdown => FileType::Markdown,
            InputFormat::Pdf => FileType::Pdf,
        }
    } else if from_stdin {
        bail!("--input-format is required when reading from STDIN");
    } else {
        detect_file_type(cli.file.as_ref().unwrap()).wrap_err("Failed to detect file type")?
    };

    let format = cli.output_format();

    // Read input content
    let content = if from_stdin {
        read_stdin()?
    } else {
        std::fs::read(cli.file.as_ref().unwrap()).wrap_err("Failed to read input file")?
    };

    match input_format {
        FileType::Toml => process_toml(&content, format)?,
        FileType::Yaml => process_yaml(&content, format)?,
        FileType::Json => process_json(&content, format)?,
        FileType::Markdown => process_markdown(&content, format)?,
        FileType::Pdf => process_pdf(&content, format)?,
        FileType::Unknown => {
            bail!(
                "Unknown file type. Use --input-format to specify the format, \
                 or ensure the file has a recognized extension \
                 (.toml, .yaml, .yml, .json, .md, .markdown, .mdx, .pdf)"
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
        bail!("No frontmatter found. Expected `---` (YAML) or `+++` (TOML) at the start of the document");
    };

    // Find the closing delimiter (skip the opening line)
    let after_open = trimmed
        .get(delimiter.len()..)
        .and_then(|s| s.find('\n').map(|i| delimiter.len() + i + 1))
        .unwrap_or(delimiter.len());

    let rest = &trimmed[after_open..];
    let close_pos = rest.find(delimiter).ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "Unclosed frontmatter. Expected closing `{delimiter}` delimiter"
        )
    })?;

    let frontmatter = &rest[..close_pos];
    Ok((frontmatter, fmt))
}

/// Process TOML content.
fn process_toml(content: &[u8], format: Option<OutputFormat>) -> Result<()> {
    let input = std::str::from_utf8(content).wrap_err("TOML input is not valid UTF-8")?;
    let toml = Toml::from_str(input).wrap_err("Failed to parse TOML")?;

    let output = match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => toml.as_json().wrap_err("Failed to convert to JSON")?,
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
fn process_yaml(content: &[u8], format: Option<OutputFormat>) -> Result<()> {
    let yaml = Yaml::from_bytes(content).wrap_err("Failed to parse YAML")?;

    let output = match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => {
            let value = yaml.as_json().wrap_err("Failed to convert to JSON")?;
            serde_json::to_string_pretty(&value).wrap_err("Failed to serialize JSON")?
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
fn process_json(content: &[u8], format: Option<OutputFormat>) -> Result<()> {
    let value: serde_json::Value =
        serde_json::from_slice(content).wrap_err("Failed to parse JSON")?;

    let output = match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => {
            serde_json::to_string_pretty(&value).wrap_err("Failed to serialize JSON")?
        }
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

/// Process Markdown content by extracting frontmatter and converting it.
fn process_markdown(content: &[u8], format: Option<OutputFormat>) -> Result<()> {
    let input = std::str::from_utf8(content).wrap_err("Markdown input is not valid UTF-8")?;
    let (frontmatter, fm_format) = extract_frontmatter(input)?;

    match fm_format {
        FrontmatterFormat::Yaml => process_yaml(frontmatter.as_bytes(), format),
        FrontmatterFormat::Toml => process_toml(frontmatter.as_bytes(), format),
    }
}

/// Process PDF content.
fn process_pdf(content: &[u8], format: Option<OutputFormat>) -> Result<()> {
    let pdf = Pdf::from_bytes(content.to_vec()).wrap_err("Failed to parse PDF")?;

    let output = match format.unwrap_or(OutputFormat::Text) {
        OutputFormat::Text => pdf.as_text().wrap_err("Failed to extract text from PDF")?,
        OutputFormat::Markdown => {
            let md = pdf
                .as_markdown(Default::default())
                .wrap_err("Failed to convert PDF to Markdown")?;
            md.content
        }
        OutputFormat::Json | OutputFormat::Yaml | OutputFormat::Toml => {
            bail!("--json, --yaml, and --toml are only supported for data file inputs");
        }
    };

    println!("{output}");
    Ok(())
}
