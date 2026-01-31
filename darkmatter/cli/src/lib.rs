//! # Darkmatter CLI
//!
//! A themed markdown renderer for the terminal and browser.
//!
//! Darkmatter renders markdown documents with syntax highlighting, image support,
//! and theme-aware styling. It can output to ANSI terminals (with escape codes)
//! or generate standalone HTML files.
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
//! # Render a markdown file to terminal
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
//! # Output as HTML
//! md README.md --html > output.html
//!
//! # Generate HTML and open in browser
//! md README.md --show-html
//!
//! # Output MDAST JSON (abstract syntax tree)
//! md README.md --ast
//!
//! # Show table of contents
//! md README.md --toc
//! md README.md --toc-filename  # Include filename in header
//! md README.md --toc --json    # JSON format
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
//! ### Comparing documents
//!
//! ```bash
//! # Show differences between two markdown files
//! md original.md --delta updated.md
//! md original.md --delta updated.md --json  # JSON format
//! md original.md --delta updated.md -v      # Verbose with visual diff
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
//! # Disable image rendering
//! md README.md --no-images
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
//! - **HTML output**: Standalone HTML with embedded styles and syntax highlighting
//! - **Syntax highlighting**: Language-aware code block highlighting via syntect
//! - **Image rendering**: Inline images in supported terminals (iTerm2, Kitty, etc.)
//! - **Mermaid diagrams**: Render mermaid diagrams to terminal or HTML
//! - **Theme support**: Multiple prose and code themes with light/dark mode detection
//! - **Markdown cleanup**: Normalize markdown formatting
//! - **Document comparison**: Structural diff between markdown documents
//! - **Table of contents**: Extract document structure as tree or JSON
//!
//! ## Library Usage
//!
//! The rendering functionality is provided by the [`darkmatter_lib`] crate.
//! See [`darkmatter_lib::markdown`] for the core markdown processing API.
//!
//! ```rust,ignore
//! use darkmatter_lib::markdown::{Markdown, TerminalOptions, write_terminal};
//!
//! let md: Markdown = "# Hello\n\nWorld".into();
//! let options = TerminalOptions::default();
//!
//! let mut stdout = std::io::stdout();
//! write_terminal(&mut stdout, &md, options)?;
//! ```

// Re-export the CLI struct for programmatic access
pub use cli::Cli;

mod cli {
    use clap::{ArgGroup, Parser};
    use clap_complete::Shell;
    use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
    use darkmatter_lib::markdown::highlighting::ThemePair;
    use std::path::{Path, PathBuf};

    /// Command-line interface for the darkmatter markdown renderer.
    ///
    /// Use `md --help` to see all available options.
    #[derive(Parser)]
    #[command(name = "md", about = "Markdown Awesome Tool", version)]
    #[command(group = ArgGroup::new("output-mode")
        .args(["html", "show_html", "ast", "clean", "clean_save", "toc", "toc_filename", "delta"])
        .multiple(false))]
    #[command(after_help = "\
SHELL COMPLETIONS:
  Enable tab completions with --completions <SHELL>

  Bash (add to ~/.bashrc):
    source <(COMPLETE=bash md)

  Zsh (add to ~/.zshrc):
    source <(COMPLETE=zsh md)

  Fish (add to ~/.config/fish/config.fish):
    COMPLETE=fish md | source

  PowerShell (add to $PROFILE):
    $env:COMPLETE = \"powershell\"; md | Out-String | Invoke-Expression; Remove-Item Env:\\COMPLETE

  Run 'md --completions <SHELL>' to see the setup command for your shell.
  Completions filter to .md and .dm files, including one directory level deep.
")]
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
        #[arg(long, group = "output-mode")]
        pub clean: bool,

        /// Clean up and save back to file
        #[arg(long, group = "output-mode")]
        pub clean_save: bool,

        /// Output as HTML
        #[arg(long, group = "output-mode")]
        pub html: bool,

        /// Generate HTML and open in browser
        #[arg(long, group = "output-mode")]
        pub show_html: bool,

        /// Output MDAST JSON
        #[arg(long, group = "output-mode")]
        pub ast: bool,

        /// Show table of contents as a tree structure
        #[arg(long, group = "output-mode")]
        pub toc: bool,

        /// Show table of contents with filename in header
        #[arg(long, group = "output-mode")]
        pub toc_filename: bool,

        /// Compare with another markdown file and show differences
        #[arg(long, group = "output-mode", value_name = "FILE")]
        pub delta: Option<PathBuf>,

        /// Output as JSON (for --toc and --delta modes)
        #[arg(long)]
        pub json: bool,

        /// Merge JSON into frontmatter (JSON wins on conflicts)
        #[arg(long, value_name = "JSON")]
        pub fm_merge_with: Option<String>,

        /// Set default frontmatter values (document wins on conflicts)
        #[arg(long, value_name = "JSON")]
        pub fm_defaults: Option<String>,

        /// Include line numbers in code blocks
        #[arg(long)]
        pub line_numbers: bool,

        /// Disable image rendering (show placeholders instead)
        #[arg(long)]
        pub no_images: bool,

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
                                    candidates.push(CompletionCandidate::new(relative.into_owned()));
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
fn parse_theme_name(s: &str) -> Result<darkmatter_lib::markdown::highlighting::ThemePair, String> {
    darkmatter_lib::markdown::highlighting::ThemePair::try_from(s).map_err(|e| e.to_string())
}
