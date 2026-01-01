use clap::{Parser, Subcommand};
use color_eyre::eyre::{Context, Result};
use shared::markdown::highlighting::{ColorMode, ThemePair};
use shared::markdown::output::{for_terminal, HtmlOptions, TerminalOptions};
use shared::markdown::Markdown;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mat", about = "Markdown Awesome Tool", version)]
struct Cli {
    /// Input file path or "-" for stdin
    input: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Cleanup and normalize markdown
    Clean {
        /// Output file path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Render to HTML
    Html {
        /// Theme for code blocks
        #[arg(long, default_value = "github")]
        theme: String,

        /// Color mode (light or dark)
        #[arg(long, default_value = "dark")]
        mode: String,

        /// Output file path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Render to terminal with ANSI codes
    View {
        /// Theme for code blocks
        #[arg(long, default_value = "github")]
        theme: String,

        /// Color mode (light or dark)
        #[arg(long, default_value = "dark")]
        mode: String,

        /// Include line numbers in code blocks
        #[arg(long)]
        line_numbers: bool,
    },
    /// Export AST as JSON
    Ast {
        /// Output file path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Load markdown from file or stdin
    let mut md = load_markdown(&cli.input)
        .with_context(|| format!("Failed to load markdown from {:?}", cli.input))?;

    // Execute the requested command
    match cli.command {
        Commands::Clean { output } => {
            md.cleanup();
            let cleaned = md.as_string();
            write_output(&cleaned, output)?;
        }
        Commands::Html { theme, mode, output } => {
            let theme_pair = parse_theme(&theme)?;
            let color_mode = parse_color_mode(&mode)?;

            let mut options = HtmlOptions::default();
            options.code_theme = theme_pair;
            options.prose_theme = theme_pair;
            options.color_mode = color_mode;

            let html = md
                .as_html(options)
                .context("Failed to convert markdown to HTML")?;

            write_output(&html, output)?;
        }
        Commands::View {
            theme,
            mode,
            line_numbers,
        } => {
            let theme_pair = parse_theme(&theme)?;
            let color_mode = parse_color_mode(&mode)?;

            let mut options = TerminalOptions::default();
            options.code_theme = theme_pair;
            options.prose_theme = theme_pair;
            options.color_mode = color_mode;
            options.include_line_numbers = line_numbers;
            options.color_depth = None; // Auto-detect

            let terminal_output = for_terminal(&md, options)
                .context("Failed to render markdown for terminal")?;

            println!("{}", terminal_output);
        }
        Commands::Ast { output } => {
            let ast = md.as_ast().context("Failed to generate AST")?;

            let json = serde_json::to_string_pretty(&ast)
                .context("Failed to serialize AST to JSON")?;

            write_output(&json, output)?;
        }
    }

    Ok(())
}

/// Loads markdown from a file path or stdin if path is "-".
fn load_markdown(path: &PathBuf) -> Result<Markdown> {
    if path.to_str() == Some("-") {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        Ok(buffer.into())
    } else {
        // Read from file
        Markdown::try_from(path.as_path())
            .with_context(|| format!("Failed to read file: {:?}", path))
    }
}

/// Writes output to a file or stdout if no path is provided.
fn write_output(content: &str, path: Option<PathBuf>) -> Result<()> {
    if let Some(output_path) = path {
        std::fs::write(&output_path, content)
            .with_context(|| format!("Failed to write to {:?}", output_path))?;
        eprintln!("Output written to {:?}", output_path);
    } else {
        println!("{}", content);
    }
    Ok(())
}

/// Parses a theme string into a ThemePair.
fn parse_theme(theme: &str) -> Result<ThemePair> {
    match theme.to_lowercase().as_str() {
        "github" => Ok(ThemePair::Github),
        "solarized" => Ok(ThemePair::Solarized),
        "dracula" => Ok(ThemePair::Dracula),
        "monokai" => Ok(ThemePair::Monokai),
        "nord" => Ok(ThemePair::Nord),
        _ => Err(color_eyre::eyre::eyre!(
            "Unknown theme: {}. Valid themes: github, solarized, dracula, monokai, nord",
            theme
        )),
    }
}

/// Parses a color mode string into a ColorMode.
fn parse_color_mode(mode: &str) -> Result<ColorMode> {
    match mode.to_lowercase().as_str() {
        "light" => Ok(ColorMode::Light),
        "dark" => Ok(ColorMode::Dark),
        _ => Err(color_eyre::eyre::eyre!(
            "Unknown color mode: {}. Valid modes: light, dark",
            mode
        )),
    }
}
