//! # Darkmatter CLI
//!
//! A themed markdown renderer for the terminal and browser.
//!
//! Darkmatter renders markdown documents with syntax highlighting, image support,
//! and theme-aware styling. It can render ANSI terminal output, markdown text,
//! HTML, or AST JSON.
//!
//! ## Installation
//!
//! ### From crates.io
//!
//! ```bash
//! cargo install darkmatter-cli
//! ```
//!
//! ### From source
//!
//! ```bash
//! git clone https://github.com/yankeeinlondon/dockhand
//! cd dockhand
//! just -f darkmatter-cli/justfile install
//! ```
//!
//! This installs the `md` binary to your Cargo bin directory.
//!
//! ## Usage
//!
//! ### Basic rendering
//!
//! ```bash
//! # Render a markdown file (auto mode)
//! md README.md
//!
//! # Pipe content from stdin
//! cat README.md | md
//! echo "# Hello\n\nWorld" | md
//! ```
//!
//! ### Output formats
//!
//! ```bash
//! # Explicit output format
//! md README.md --output html > output.html
//! md README.md --output markdown
//! md README.md --output json
//! md README.md --output ast      # Alias of json
//! md README.md --output text     # Alias of markdown
//! ```
//!
//! ### Show output artifact
//!
//! ```bash
//! # Open output in default app
//! md README.md --output html --show
//! md README.md --output markdown --show
//! md README.md --output json --show
//! ```
//!
//! ### Table of contents
//!
//! ```bash
//! # Show table of contents tree
//! md toc README.md
//!
//! # JSON format
//! md toc README.md --json
//! ```
//!
//! ### Comparing documents
//!
//! ```bash
//! # Show differences between two markdown files
//! md delta original.md updated.md
//! md delta original.md updated.md --json
//! md delta original.md updated.md -v
//! ```
//!
//! ### Markdown cleanup
//!
//! ```bash
//! # Clean up markdown formatting (stdout)
//! md README.md --clean
//!
//! # Clean up and save back to file
//! md README.md --clean-save
//! ```
//!
//! ### Theming
//!
//! ```bash
//! # List available themes
//! md --list-themes
//!
//! # Use a specific theme
//! md README.md --theme dracula
//!
//! # Use different themes for prose and code
//! md README.md --theme nord --code-theme monokai
//! ```
//!
//! ### Advanced options
//!
//! ```bash
//! # Show line numbers in code blocks
//! md README.md --line-numbers
//!
//! # Render mermaid diagrams as images
//! md README.md --mermaid
//!
//! # Verbose output for debugging
//! md README.md -v      # INFO level
//! md README.md -vv     # DEBUG level
//! md README.md -vvv    # TRACE level
//! ```
//!
//! ## Features
//!
//! - **Terminal rendering**: ANSI escape codes with automatic color depth detection
//! - **Markdown output**: Clean markdown text output for piping and file workflows
//! - **HTML output**: Standalone HTML with embedded styles and syntax highlighting
//! - **AST JSON output**: JSON AST export for programmatic workflows
//! - **Image rendering**: Inline images in supported terminals (iTerm2, Kitty, etc.)
//! - **Mermaid diagrams**: Render mermaid diagrams to terminal or HTML
//! - **Theme support**: Multiple prose and code themes with light/dark mode detection
//! - **Markdown cleanup**: Normalize markdown formatting
//! - **Document comparison**: Structural diff between markdown documents
//! - **Table of contents**: Extract document structure as tree or JSON
//!
//! ## Library Usage
//!
//! The rendering functionality is provided by the [`darkmatter`] crate.
//! See [`darkmatter::markdown`] for the core markdown processing API.
//!
//! ```rust,ignore
//! use darkmatter::markdown::{Markdown, TerminalOptions, write_terminal};
//!
//! let md: Markdown = "# Hello\n\nWorld".into();
//! let options = TerminalOptions::default();
//!
//! let mut stdout = std::io::stdout();
//! write_terminal(&mut stdout, &md, options)?;
//! ```

// Re-export CLI types for programmatic access
pub use cli::{Cli, Command as CliCommand, OutputFormat};

mod cli {
    use clap::{Parser, Subcommand, ValueEnum};
    use clap_complete::Shell;
    use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
    use darkmatter::markdown::highlighting::ThemePair;
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

    /// Subcommands for non-render operations.
    #[derive(Clone, Debug, Subcommand)]
    pub enum Command {
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
        #[arg(long, value_parser = super::parse_theme_name)]
        pub theme: Option<ThemePair>,

        /// Theme for code blocks (overrides derived theme)
        #[arg(long, value_parser = super::parse_theme_name)]
        pub code_theme: Option<ThemePair>,

        /// List available themes
        #[arg(long)]
        pub list_themes: bool,

        /// Clean up markdown formatting (output to stdout)
        #[arg(long, conflicts_with = "clean_save")]
        pub clean: bool,

        /// Clean up and save back to file
        #[arg(long, conflicts_with = "clean")]
        pub clean_save: bool,

        /// Output format for top-level render mode
        #[arg(long, value_enum, default_value_t = OutputFormat::Auto, conflicts_with_all = ["clean", "clean_save"])]
        pub output: OutputFormat,

        /// Open selected output in the default app using a temp file
        #[arg(long)]
        pub show: bool,

        /// Merge JSON into frontmatter (JSON wins on conflicts)
        #[arg(long, value_name = "JSON")]
        pub fm_merge_with: Option<String>,

        /// Set default frontmatter values (document wins on conflicts)
        #[arg(long, value_name = "JSON")]
        pub fm_defaults: Option<String>,

        /// Include line numbers in code blocks
        #[arg(long)]
        pub line_numbers: bool,

        /// Render mermaid diagrams to terminal as images.
        /// Falls back to code blocks if terminal doesn't support images.
        #[arg(long)]
        pub mermaid: bool,

        /// Increase verbosity (-v INFO, -vv DEBUG, -vvv TRACE, -vvvv TRACE with file/line)
        #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        pub verbose: u8,

        /// Generate shell completions for the specified shell
        #[arg(long, value_name = "SHELL")]
        pub completions: Option<Shell>,

        /// TOC and delta subcommands
        #[command(subcommand)]
        pub command: Option<Command>,
    }

    /// Completes markdown files (.md, .dm) in current directory and one level deep.
    fn complete_markdown_files(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
        let current_str = current.to_string_lossy();
        let mut candidates = Vec::new();

        let is_markdown = |p: &Path| {
            p.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("dm"))
                .unwrap_or(false)
        };

        // Current directory files
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && is_markdown(&path)
                    && let Some(name) = path.file_name()
                {
                    let name = name.to_string_lossy();
                    if name.starts_with(current_str.as_ref()) {
                        candidates.push(CompletionCandidate::new(name.into_owned()));
                    }
                }
            }
        }

        // One level deep in subdirectories
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let dir_path = entry.path();
                if dir_path.is_dir() {
                    // Skip hidden directories
                    if dir_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with('.'))
                        .unwrap_or(false)
                    {
                        continue;
                    }

                    if let Ok(subentries) = std::fs::read_dir(&dir_path) {
                        for subentry in subentries.flatten() {
                            let file_path = subentry.path();
                            if file_path.is_file() && is_markdown(&file_path) {
                                // Strip leading "./" for cleaner display
                                let relative = file_path
                                    .strip_prefix("./")
                                    .unwrap_or(&file_path)
                                    .to_string_lossy();
                                if relative.starts_with(current_str.as_ref()) {
                                    candidates
                                        .push(CompletionCandidate::new(relative.into_owned()));
                                }
                            }
                        }
                    }
                }
            }
        }

        candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
        candidates
    }
}

/// Parses a theme name string into ThemePair.
fn parse_theme_name(s: &str) -> Result<darkmatter::markdown::highlighting::ThemePair, String> {
    darkmatter::markdown::highlighting::ThemePair::try_from(s).map_err(|e| e.to_string())
}
