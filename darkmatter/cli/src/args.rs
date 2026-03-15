use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use darkmatter::markdown::highlighting::ThemePair;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Output format for top-level render mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    /// Auto: terminal rendering on TTY, markdown text on non-TTY.
    Auto,
    /// Markdown text output.
    #[value(alias = "text")]
    Markdown,
    /// HTML output.
    Html,
    /// AST JSON output.
    #[value(alias = "ast")]
    Json,
}

/// CLI subcommands.
#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Render a markdown document (same as default behavior without a subcommand).
    Read {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: Option<PathBuf>,

        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Auto)]
        output: OutputFormat,

        /// Open output in the default app using a temp file
        #[arg(long)]
        show: bool,
    },

    /// Clean up markdown formatting.
    Clean {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: Option<PathBuf>,

        /// Save cleaned markdown in place and report delta-style changes
        #[arg(long)]
        save: bool,

        /// Normalize nested list indentation width (spaces per level)
        #[arg(
            long,
            value_name = "#",
            value_parser = parse_indent_size,
            add = ArgValueCompleter::new(complete_indent_values)
        )]
        indent: Option<usize>,
    },

    /// Compose a document through the transform pipeline.
    Compose {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: Option<PathBuf>,

        /// Default values as JSON; fills in null/missing frontmatter keys without overriding existing values
        #[arg(long, value_name = "JSON")]
        state: Option<String>,

        /// Output format (default: markdown for compose)
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        output: OutputFormat,

        /// Render composed content to stdout AND open in default app
        #[arg(long)]
        show: bool,

        /// Include frontmatter in the output (default: body only)
        #[arg(long, visible_alias = "fm")]
        frontmatter: bool,
    },

    /// Show markdown table of contents.
    Toc {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT")]
        input: PathBuf,

        /// Output TOC as JSON
        #[arg(long)]
        json: bool,
    },

    /// Compare two markdown documents.
    Delta {
        /// Base/original markdown file
        #[arg(value_name = "BASE")]
        base: PathBuf,

        /// Updated markdown file
        #[arg(value_name = "UPDATED")]
        updated: PathBuf,

        /// Output delta as JSON
        #[arg(long)]
        json: bool,
    },

    /// Get frontmatter properties from a markdown document.
    Get {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Frontmatter property names to retrieve
        #[arg(value_name = "PROP", required = true, num_args = 1..)]
        props: Vec<String>,

        /// Output as JSON5
        #[arg(long)]
        json5: bool,

        /// Output as YAML
        #[arg(long)]
        yaml: bool,

        /// Output as TOML
        #[arg(long)]
        toml: bool,
    },

    /// Set a frontmatter property on a markdown document.
    Set {
        /// Input file path (use "-" for stdin; outputs to stdout)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Frontmatter property name to set
        #[arg(value_name = "PROP")]
        prop: String,

        /// Value to set (parsed as JSON if valid, otherwise treated as a string)
        #[arg(value_name = "VALUE")]
        value: String,

        /// Write the change back to the source file (no output)
        #[arg(long)]
        save: bool,
    },

    /// Open a markdown file in your preferred editor.
    ///
    /// Resolves the file using biscuit-file's FileReference system (supports `@`, `!`,
    /// `vault:`, etc.). Creates the file if it doesn't exist. Blocks until the editor
    /// exits. Returns the fully qualified filename on success.
    ///
    /// Editor priority: $EDITOR > $VISUAL > first installed from default list.
    Edit {
        /// File path or reference to edit
        #[arg(value_name = "FILE", add = ArgValueCompleter::new(complete_markdown_files))]
        file: String,
    },

    /// Hash a markdown document's frontmatter and body.
    Hash {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: Option<PathBuf>,

        /// Only output the body/prose hash
        #[arg(long)]
        body: bool,

        /// Only output the frontmatter hash
        #[arg(long)]
        frontmatter: bool,

        /// Strict mode: no whitespace normalization or key reordering
        #[arg(long)]
        strict: bool,
    },
}

/// Command-line interface for the darkmatter markdown renderer.
///
/// Use `md --help` to see all available options.
#[derive(Parser)]
#[command(name = "md", about = "Markdown Awesome Tool", version)]
#[command(subcommand_precedence_over_arg = true)]
pub struct Cli {
    /// Input file path (reads from stdin if not provided, use "-" for explicit stdin)
    #[arg(add = ArgValueCompleter::new(complete_markdown_files))]
    pub input: Option<PathBuf>,

    /// Theme for prose content (kebab-case name)
    #[arg(long, value_parser = parse_theme_name)]
    pub theme: Option<ThemePair>,

    /// Theme for code blocks (overrides derived theme)
    #[arg(long, value_parser = parse_theme_name)]
    pub code_theme: Option<ThemePair>,

    /// List available themes
    #[arg(long)]
    pub list_themes: bool,

    /// Output format for top-level render mode (when no subcommand given)
    #[arg(long, value_enum, default_value_t = OutputFormat::Auto)]
    pub output: OutputFormat,

    /// Open selected output in the default app using a temp file
    #[arg(long)]
    pub show: bool,

    /// Shorthand for `clean --save` with top-level [INPUT]
    #[arg(long)]
    pub save: bool,

    /// Include line numbers in code blocks
    #[arg(long)]
    pub line_numbers: bool,

    /// Render mermaid diagrams to terminal as images.
    /// Falls back to code blocks if terminal doesn't support images.
    #[arg(long)]
    pub mermaid: bool,

    /// Increase verbosity (-v INFO, -vv DEBUG, -vvv TRACE, -vvvv TRACE with file/line)
    #[arg(
        short = 'v',
        long = "verbose",
        action = clap::ArgAction::Count,
        global = true
    )]
    pub verbose: u8,

    /// Generate shell completions for the specified shell
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<Shell>,

    /// Subcommand (read, clean, compose, toc, delta)
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Completes markdown files (`.md`, `.dm`) and directory paths.
fn complete_markdown_files(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_markdown_files_from(Path::new("."), current)
}

fn complete_markdown_files_from(base_dir: &Path, current: &OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();

    let is_markdown = |p: &Path| {
        p.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("dm"))
            .unwrap_or(false)
    };

    if !current_str.is_empty() && "-".starts_with(current_str.as_ref()) {
        seen.insert("-".to_string());
        candidates.push(CompletionCandidate::new("-"));
    }

    let has_trailing_sep = current_str.ends_with('/') || current_str.ends_with('\\');
    let current_path = Path::new(current_str.as_ref());
    let (dir_part, file_prefix) = if current_str.is_empty() {
        (PathBuf::new(), String::new())
    } else if has_trailing_sep {
        (PathBuf::from(current_str.as_ref()), String::new())
    } else {
        let parent = current_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let prefix = current_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        (parent, prefix)
    };

    let search_dir = if dir_part.as_os_str().is_empty() {
        base_dir.to_path_buf()
    } else if dir_part.is_absolute() {
        dir_part.clone()
    } else {
        base_dir.join(&dir_part)
    };

    if let Ok(entries) = std::fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if !name.starts_with(&file_prefix) {
                continue;
            }

            let is_dir = path.is_dir();
            if !is_dir && !is_markdown(&path) {
                continue;
            }

            let mut display_path = if current_path.is_absolute() {
                path.to_string_lossy().to_string()
            } else {
                path.strip_prefix(base_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string()
            };

            if display_path.starts_with("./") {
                display_path = display_path.trim_start_matches("./").to_string();
            }

            if is_dir && !display_path.ends_with('/') {
                display_path.push('/');
            }

            if seen.insert(display_path.clone()) {
                candidates.push(CompletionCandidate::new(display_path));
            }
        }
    }

    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
}

/// Completes supported list indentation widths.
fn complete_indent_values(current: &OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();
    let mut candidates: Vec<_> = ["2", "4", "8"]
        .into_iter()
        .filter(|value| value.starts_with(current_str.as_ref()))
        .map(CompletionCandidate::new)
        .collect();
    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
}

/// Parses and validates list indentation width.
pub fn parse_indent_size(s: &str) -> Result<usize, String> {
    let value = s
        .parse::<usize>()
        .map_err(|_| format!("'{s}' is not a valid integer"))?;

    match value {
        2 | 4 | 8 => Ok(value),
        _ => Err("indent must be one of: 2, 4, 8".to_string()),
    }
}

/// Parses a theme name string into ThemePair.
pub fn parse_theme_name(s: &str) -> Result<darkmatter::markdown::highlighting::ThemePair, String> {
    darkmatter::markdown::highlighting::ThemePair::try_from(s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completion_values(candidates: Vec<CompletionCandidate>) -> Vec<String> {
        candidates
            .into_iter()
            .map(|candidate| candidate.get_value().to_string_lossy().into_owned())
            .collect()
    }

    fn normalize_path(path: &str) -> String {
        path.replace('\\', "/")
    }

    #[test]
    fn test_complete_indent_values() {
        let values = completion_values(complete_indent_values(OsStr::new("")));
        assert_eq!(values, vec!["2", "4", "8"]);

        let values = completion_values(complete_indent_values(OsStr::new("4")));
        assert_eq!(values, vec!["4"]);
    }

    #[test]
    fn test_parse_indent_size() {
        assert_eq!(parse_indent_size("2"), Ok(2));
        assert_eq!(parse_indent_size("4"), Ok(4));
        assert_eq!(parse_indent_size("8"), Ok(8));
        assert!(parse_indent_size("3").is_err());
        assert!(parse_indent_size("abc").is_err());
    }

    #[test]
    fn test_complete_markdown_files_from_supports_nested_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("README.md"), "# Root").unwrap();
        std::fs::write(temp_dir.path().join("notes.txt"), "ignore").unwrap();

        let docs_dir = temp_dir.path().join("docs");
        let deep_dir = docs_dir.join("deep");
        std::fs::create_dir_all(&deep_dir).unwrap();
        std::fs::write(docs_dir.join("guide.md"), "# Guide").unwrap();
        std::fs::write(deep_dir.join("nested.md"), "# Nested").unwrap();

        let root_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new(""),
        ));
        let root_values: Vec<_> = root_values
            .into_iter()
            .map(|value| normalize_path(&value))
            .collect();
        assert!(root_values.contains(&"README.md".to_string()));
        assert!(root_values.contains(&"docs/".to_string()));
        assert!(!root_values.iter().any(|value| value.ends_with("notes.txt")));

        let docs_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new("docs/"),
        ));
        let docs_values: Vec<_> = docs_values
            .into_iter()
            .map(|value| normalize_path(&value))
            .collect();
        assert!(docs_values.contains(&"docs/guide.md".to_string()));
        assert!(docs_values.contains(&"docs/deep/".to_string()));

        let deep_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new("docs/deep/"),
        ));
        let deep_values: Vec<_> = deep_values
            .into_iter()
            .map(|value| normalize_path(&value))
            .collect();
        assert!(deep_values.contains(&"docs/deep/nested.md".to_string()));
    }
}
