use clap::{ArgGroup, Parser};
use color_eyre::eyre::{eyre, Context, Result};
use shared::markdown::highlighting::{
    detect_code_theme, detect_color_mode, detect_prose_theme, ColorMode, ThemePair,
};
use shared::markdown::output::{HtmlOptions, MermaidMode, TerminalOptions, write_terminal};
use shared::markdown::Markdown;
use std::io::{self, Read};
use std::path::PathBuf;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "md", about = "Markdown Awesome Tool", version)]
#[command(group = ArgGroup::new("output-mode")
    .args(["html", "show_html", "ast", "clean", "clean_save"])
    .multiple(false))]
struct Cli {
    /// Input file path (reads from stdin if not provided, use "-" for explicit stdin)
    input: Option<PathBuf>,

    /// Theme for prose content (kebab-case name)
    #[arg(long, value_parser = parse_theme_name)]
    theme: Option<ThemePair>,

    /// Theme for code blocks (overrides derived theme)
    #[arg(long, value_parser = parse_theme_name)]
    code_theme: Option<ThemePair>,

    /// List available themes
    #[arg(long)]
    list_themes: bool,

    /// Clean up markdown formatting (output to stdout)
    #[arg(long, group = "output-mode")]
    clean: bool,

    /// Clean up and save back to file
    #[arg(long, group = "output-mode")]
    clean_save: bool,

    /// Output as HTML
    #[arg(long, group = "output-mode")]
    html: bool,

    /// Generate HTML and open in browser
    #[arg(long, group = "output-mode")]
    show_html: bool,

    /// Output MDAST JSON
    #[arg(long, group = "output-mode")]
    ast: bool,

    /// Merge JSON into frontmatter (JSON wins on conflicts)
    #[arg(long, value_name = "JSON")]
    fm_merge_with: Option<String>,

    /// Set default frontmatter values (document wins on conflicts)
    #[arg(long, value_name = "JSON")]
    fm_defaults: Option<String>,

    /// Include line numbers in code blocks
    #[arg(long)]
    line_numbers: bool,

    /// Disable image rendering (show placeholders instead)
    #[arg(long)]
    no_images: bool,

    /// Render mermaid diagrams to terminal as images.
    /// Falls back to code blocks if terminal doesn't support images.
    #[arg(long)]
    mermaid: bool,

    /// Display mermaid diagrams as text (code blocks) instead of images.
    /// Useful for terminals that don't support inline images.
    #[arg(long)]
    mermaid_alt: bool,

    /// Increase verbosity (-v INFO, -vv DEBUG, -vvv TRACE, -vvvv TRACE with file/line)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,
}

/// Parses a theme name string into ThemePair.
fn parse_theme_name(s: &str) -> Result<ThemePair, String> {
    ThemePair::try_from(s).map_err(|e| e.to_string())
}

/// Initialize tracing subscriber based on verbosity level.
///
/// Verbosity levels:
/// - 0 (default): WARN only (errors and warnings)
/// - 1 (-v): INFO (tool calls, phase transitions)
/// - 2 (-vv): DEBUG (tool arguments, API requests)
/// - 3 (-vvv): TRACE (request/response bodies)
/// - 4+ (-vvvv): TRACE with file/line numbers
fn init_tracing(verbose: u8) {
    // Only initialize if verbose mode is enabled
    if verbose == 0 {
        return;
    }

    let base_filter = match std::env::var("RUST_LOG") {
        Ok(filter) => filter,
        Err(_) => match verbose {
            // -v: Show INFO for progress and tool calls
            1 => "info,md=info,shared=info".to_string(),
            // -vv: Show DEBUG for tool arguments and requests
            2 => "info,md=debug,shared=debug".to_string(),
            // -vvv+: Show TRACE for detailed debugging
            _ => "debug,md=trace,shared=trace".to_string(),
        },
    };

    let filter = EnvFilter::try_new(&base_filter).unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(verbose >= 4)
                .with_line_number(verbose >= 4)
                .with_writer(std::io::stderr)
                .compact(),
        )
        .init();
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    // Handle --list-themes first (no input needed)
    if cli.list_themes {
        list_themes();
        return Ok(());
    }

    // Load markdown from input or stdin
    let mut md = load_markdown(cli.input.as_ref())?;

    // Handle frontmatter operations
    if let Some(ref json) = cli.fm_merge_with {
        let data: serde_json::Value =
            serde_json::from_str(json).wrap_err("Invalid JSON in --fm-merge-with argument")?;
        // TODO: Implement fm_merge_with when Markdown API is available
        eprintln!("Frontmatter merge: {:?}", data);
        return Ok(());
    }

    if let Some(ref json) = cli.fm_defaults {
        let data: serde_json::Value =
            serde_json::from_str(json).wrap_err("Invalid JSON in --fm-defaults argument")?;
        // TODO: Implement fm_defaults when Markdown API is available
        eprintln!("Frontmatter defaults: {:?}", data);
        return Ok(());
    }

    // Handle clean operations
    if cli.clean {
        md.cleanup();
        println!("{}", md.as_string());
        return Ok(());
    }

    if cli.clean_save {
        let path = cli
            .input
            .ok_or_else(|| eyre!("--clean-save requires a file path, not stdin"))?;
        md.cleanup();
        std::fs::write(&path, md.as_string())
            .wrap_err_with(|| format!("Failed to write to {:?}", path))?;
        eprintln!("Saved cleaned content to {:?}", path);
        return Ok(());
    }

    // Resolve themes
    let prose_theme = cli.theme.unwrap_or_else(detect_prose_theme);
    let code_theme = cli
        .code_theme
        .unwrap_or_else(|| detect_code_theme(prose_theme));
    let color_mode = detect_color_mode();

    // Handle output modes
    if cli.ast {
        let ast = md.as_ast().context("Failed to generate AST")?;
        println!("{}", serde_json::to_string_pretty(&ast)?);
        return Ok(());
    }

    if cli.html {
        let mut options = HtmlOptions::default();
        options.prose_theme = prose_theme;
        options.code_theme = code_theme;
        options.color_mode = color_mode;
        // For HTML output, default to interactive mermaid diagrams
        // (browsers can render them natively via mermaid.js)
        options.mermaid_mode = if cli.mermaid_alt {
            MermaidMode::Text
        } else {
            MermaidMode::Image
        };

        let html = md.as_html(options).context("Failed to convert to HTML")?;
        println!("{}", html);
        return Ok(());
    }

    if cli.show_html {
        let mut options = HtmlOptions::default();
        options.prose_theme = prose_theme;
        options.code_theme = code_theme;
        options.color_mode = color_mode;
        // For HTML output, default to interactive mermaid diagrams
        options.mermaid_mode = if cli.mermaid_alt {
            MermaidMode::Text
        } else {
            MermaidMode::Image
        };

        let html = md.as_html(options).context("Failed to convert to HTML")?;
        let temp_path = std::env::temp_dir().join("md-preview.html");
        std::fs::write(&temp_path, &html)
            .wrap_err("Failed to write temp HTML file")?;

        // Non-blocking open, graceful error handling
        if let Err(e) = open::that(&temp_path) {
            eprintln!("Failed to open browser: {}", e);
            eprintln!("Preview available at: {}", temp_path.display());
        }
        return Ok(());
    }

    // Default: render to terminal
    let mut options = TerminalOptions::default();
    options.prose_theme = prose_theme;
    options.code_theme = code_theme;
    options.color_mode = color_mode;
    options.include_line_numbers = cli.line_numbers;
    options.color_depth = None; // Auto-detect
    options.render_images = !cli.no_images;
    options.mermaid_mode = if cli.mermaid {
        MermaidMode::Image
    } else if cli.mermaid_alt {
        MermaidMode::Text
    } else {
        MermaidMode::Off
    };

    // Derive base_path from input file for relative image resolution
    if let Some(ref path) = cli.input
        && path.to_str() != Some("-")
    {
        options.base_path = path.parent().map(|p| p.to_path_buf());
    }

    // Use write_terminal with stdout for proper image rendering
    // (viuer requires direct stdout access for graphics protocols)
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    write_terminal(&mut handle, &md, options).context("Failed to render markdown for terminal")?;

    Ok(())
}

/// Loads markdown from a file path or stdin.
fn load_markdown(path: Option<&PathBuf>) -> Result<Markdown> {
    if let Some(p) = path {
        if p.to_str() == Some("-") {
            // Explicit stdin marker
            read_from_stdin()
        } else {
            Markdown::try_from(p.as_path())
                .wrap_err_with(|| format!("Failed to read file: {:?}", p))
        }
    } else {
        // No path provided - check if stdin has data
        if atty::is(atty::Stream::Stdin) {
            // Interactive terminal - no input available
            Err(eyre!(
                "No input file provided. Use `md --help` for usage."
            ))
        } else {
            // Piped input available
            read_from_stdin()
        }
    }
}

/// Reads markdown content from stdin.
fn read_from_stdin() -> Result<Markdown> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .wrap_err("Failed to read from stdin")?;
    Ok(buffer.into())
}

/// Lists all available themes with descriptions.
fn list_themes() {
    println!("Available themes:\n");
    for theme_pair in ThemePair::all() {
        println!(
            "  {:20} {}",
            theme_pair.kebab_name(),
            theme_pair.description(ColorMode::Dark)
        );
    }
    println!("\nUse --theme <name> to set prose theme");
    println!("Use --code-theme <name> to override code theme");
}
