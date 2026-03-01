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
//! md clean README.md
//! md clean README.md --indent 4
//! echo "# Hello" | md clean -
//!
//! # Save cleaned content in place and print a delta-style change report
//! md clean README.md --save
//! md README.md --save
//! ```
//!
//! ### Transform pipeline
//!
//! ```bash
//! # Compose a document through the transform pipeline
//! md compose doc.md
//! md compose doc.md --state '{"name":"Alice"}'
//! md compose doc.md --output html
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
//! - **Markdown cleanup**: Normalize markdown formatting, optionally save in place with change reports
//! - **Transform pipeline**: Compose documents with interpolation, replacement, and transclusion
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

pub mod args;
pub mod commands;
pub mod output;

// Re-export CLI types for programmatic access
pub use args::{Cli, Command as CliCommand, OutputFormat};
