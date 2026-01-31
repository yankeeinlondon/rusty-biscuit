//! Terminal information utility CLI.
//!
//! Displays terminal metadata and capabilities including:
//! - Terminal application detection
//! - Color depth and mode
//! - Feature support (italics, images, underlines, OSC links)
//! - Multiplexing status
//! - OS and distribution information

use std::path::Path;

use biscuit_terminal::{
    components::{
        mermaid::MermaidRenderer,
        terminal_image::{parse_filepath_and_width, parse_width_spec, TerminalImage},
    },
    discovery::{
        clipboard,
        detection::{multiplex_support, Connection, MultiplexSupport},
        eval, fonts, mode_2027, osc_queries,
    },
    terminal::Terminal,
    utils::escape_codes,
};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::engine::{ArgValueCompleter, PathCompleter};
use clap_complete::Shell;
use serde::Serialize;

/// Terminal information utility
#[derive(Parser, Debug)]
#[command(name = "bt")]
#[command(author, version, about = "Display terminal metadata and capabilities")]
#[command(after_help = "\
SHELL COMPLETIONS:
  Two methods are available:

  DYNAMIC (recommended, includes image file filtering):
    # Bash
    echo 'source <(COMPLETE=bash bt)' >> ~/.bashrc

    # Zsh
    echo 'source <(COMPLETE=zsh bt)' >> ~/.zshrc

    # Fish
    echo 'COMPLETE=fish bt | source' >> ~/.config/fish/config.fish

  STATIC (generates a completion script):
    # Bash
    bt --completions bash >> ~/.bashrc

    # Zsh (ensure fpath includes the directory)
    bt --completions zsh > ~/.zfunc/_bt

    # Fish
    bt --completions fish > ~/.config/fish/completions/bt.fish

    # PowerShell
    bt --completions powershell >> $PROFILE
")]
struct Args {
    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    /// Verbose output (show more details)
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Generate shell completions and exit.
    ///
    /// Outputs completion scripts for the specified shell to stdout.
    /// Redirect the output to the appropriate file for your shell.
    /// Use --completions help for setup instructions.
    #[arg(long, value_name = "SHELL", global = true)]
    completions: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,

    /// Content to analyze (positional; multiple values are joined with spaces)
    #[arg(value_name = "CONTENT")]
    content: Vec<String>,
}

/// CLI subcommands
#[derive(Subcommand, Debug)]
enum Command {
    /// Display an image in the terminal
    ///
    /// Supports width specification: "file.jpg|50%" or "file.jpg|80".
    /// Supports PNG, JPG, JPEG, and GIF formats.
    Image {
        /// Image file path with optional width spec (e.g., "photo.jpg|75%")
        #[arg(value_name = "FILEPATH", add = ArgValueCompleter::new(image_completer()))]
        filepath: String,
    },

    /// Render a flowchart from node definitions
    ///
    /// Creates a Mermaid flowchart and renders it to the terminal.
    /// Default direction is left-to-right (LR).
    ///
    /// Examples:
    ///   bt flowchart "A --> B --> C"
    ///   bt flowchart --vertical "Start --> Middle --> End"
    ///   bt flowchart "A[Input] --> B{Decision}" "B -->|Yes| C[Output]"
    ///   bt flowchart --inverse "A --> B"  # Solid background with inverted colors
    Flowchart {
        /// Render top-down instead of left-right
        #[arg(long)]
        vertical: bool,

        /// Use inverted colors with solid background
        ///
        /// Instead of transparent background matching the terminal, renders with
        /// a solid background (white in dark mode, black in light mode) and
        /// contrasting shapes.
        #[arg(long)]
        inverse: bool,

        /// Flowchart node and edge definitions (e.g., "A --> B --> C")
        #[arg(value_name = "CONTENT", required = true)]
        content: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
struct TerminalMetadata {
    /// Terminal application name
    app: String,
    /// Operating system type
    os: String,
    /// Linux distribution info (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    distro: Option<DistroInfo>,

    /// Terminal width in columns
    width: u32,
    /// Terminal height in rows
    height: u32,

    /// Whether stdout is connected to a TTY
    is_tty: bool,
    /// Whether running in a CI environment
    is_ci: bool,

    /// Font name (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    font: Option<String>,
    /// Font size in pixels (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    font_size: Option<u32>,
    /// Whether using a Nerd Font (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    is_nerd_font: Option<bool>,
    /// Font ligatures (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    font_ligatures: Option<Vec<String>>,
    /// Whether the terminal likely supports font ligatures (heuristic)
    ligatures_likely: bool,

    /// Supported color depth
    color_depth: String,
    /// Light/dark mode
    color_mode: String,
    /// Background color (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    bg_color: Option<ColorInfo>,
    /// Text/foreground color (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    text_color: Option<ColorInfo>,
    /// Cursor color (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor_color: Option<ColorInfo>,

    /// Whether italics are supported
    supports_italic: bool,
    /// Image rendering support
    image_support: String,
    /// Underline style support
    underline_support: UnderlineInfo,
    /// OSC8 hyperlink support
    osc_link_support: bool,
    /// OSC10 foreground color query support
    osc10_fg_color: bool,
    /// OSC11 background color query support
    osc11_bg_color: bool,
    /// OSC12 cursor color query support
    osc12_cursor_color: bool,
    /// OSC52 clipboard support
    osc52_clipboard: bool,
    /// Mode 2027 grapheme cluster width support
    mode_2027_graphemes: bool,

    /// Multiplexer type
    multiplex: String,

    /// Connection type (Local, SSH, Mosh)
    connection: ConnectionInfo,
    /// Raw locale string from environment (e.g., "en_US.UTF-8", "C")
    #[serde(skip_serializing_if = "Option::is_none")]
    locale_raw: Option<String>,
    /// Normalized locale tag (BCP47 format, e.g., "en-US", "und" for C/POSIX)
    #[serde(skip_serializing_if = "Option::is_none")]
    locale_tag: Option<String>,
    /// Character encoding
    char_encoding: String,

    /// Path to terminal config file
    #[serde(skip_serializing_if = "Option::is_none")]
    config_file: Option<String>,
}

#[derive(Debug, Serialize)]
struct ContentAnalysis {
    /// Number of lines in the content
    line_count: u32,
    /// Length of each line in characters (escape codes stripped)
    line_lengths: Vec<u32>,
    /// Whether the content contains SGR color escape codes
    contains_color_escape_codes: bool,
    /// Whether the content contains OSC8 links
    contains_osc8_links: bool,
    /// Total character length (escape codes stripped)
    total_length: u32,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ConnectionInfo {
    Local,
    #[serde(rename = "SSH")]
    Ssh {
        host: String,
        source_port: u32,
        server_port: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        tty_path: Option<String>,
    },
    Mosh {
        connection: String,
    },
}

#[derive(Debug, Serialize)]
struct DistroInfo {
    /// Distribution ID (e.g., "ubuntu", "fedora")
    id: String,
    /// Pretty name
    name: String,
    /// Version number
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    /// Version codename
    #[serde(skip_serializing_if = "Option::is_none")]
    codename: Option<String>,
    /// Distribution family
    family: String,
}

#[derive(Debug, Serialize)]
struct ColorInfo {
    /// Red component (0-255)
    r: u8,
    /// Green component (0-255)
    g: u8,
    /// Blue component (0-255)
    b: u8,
    /// Hex color code
    #[serde(skip_serializing_if = "Option::is_none")]
    hex: Option<String>,
}

#[derive(Debug, Serialize)]
struct UnderlineInfo {
    /// Straight/single underline
    straight: bool,
    /// Double underline
    double: bool,
    /// Curly/squiggly underline
    curly: bool,
    /// Dotted underline
    dotted: bool,
    /// Dashed underline
    dashed: bool,
    /// Colored underlines
    colored: bool,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Handle dynamic completions (COMPLETE env var)
    // This must run before any other initialization
    clap_complete::CompleteEnv::with_factory(Args::command).complete();

    // Setup logging if RUST_LOG is set
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    let args = Args::parse();

    // Handle --completions flag (generates static completion scripts)
    if let Some(ref shell_arg) = args.completions {
        return handle_completions(shell_arg);
    }

    // Handle subcommands
    match args.command {
        Some(Command::Image { ref filepath }) => {
            return render_image(filepath);
        }
        Some(Command::Flowchart {
            vertical,
            inverse,
            ref content,
        }) => {
            return render_flowchart(vertical, inverse, content, args.json);
        }
        None => {
            // Default behavior: content analysis or terminal metadata
        }
    }

    let content = if args.content.is_empty() {
        None
    } else {
        Some(args.content.join(" "))
    };

    if let Some(content) = content.as_deref() {
        let analysis = analyze_content(content);
        if args.json {
            println!("{}", serde_json::to_string_pretty(&analysis)?);
        } else {
            print_content_analysis(&analysis);
        }
        return Ok(());
    }

    let metadata = collect_metadata();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&metadata)?);
    } else {
        print_pretty(&metadata, args.verbose);
    }

    Ok(())
}

/// Handles the --completions flag.
///
/// If "help" is provided, shows setup instructions.
/// Otherwise, generates shell completion scripts.
fn handle_completions(shell_arg: &str) -> color_eyre::Result<()> {
    let shell_lower = shell_arg.to_lowercase();

    if shell_lower == "help" {
        print_completions_help();
        return Ok(());
    }

    let shell = match shell_lower.as_str() {
        "bash" => Shell::Bash,
        "elvish" => Shell::Elvish,
        "fish" => Shell::Fish,
        "powershell" | "pwsh" => Shell::PowerShell,
        "zsh" => Shell::Zsh,
        _ => {
            eprintln!(
                "error: invalid shell '{}'\n\nValid shells: bash, elvish, fish, powershell, zsh\n\nUse 'bt --completions help' for setup instructions.",
                shell_arg
            );
            std::process::exit(1);
        }
    };

    print_completions(shell);
    Ok(())
}

/// Prints shell completions to stdout.
fn print_completions(shell: Shell) {
    let mut cmd = Args::command();
    clap_complete::generate(shell, &mut cmd, "bt", &mut std::io::stdout());
}

/// Prints help about setting up shell completions.
fn print_completions_help() {
    println!(
        r#"bt Shell Completions Setup

Two methods are available for enabling tab completion:

DYNAMIC COMPLETIONS (recommended)
=================================
Dynamic completions call bt at completion time, providing:
- Image file filtering (only *.png, *.jpg, *.jpeg, *.gif)
- Always up-to-date with current bt version

Setup:
  Bash:  echo 'source <(COMPLETE=bash bt)' >> ~/.bashrc
  Zsh:   echo 'source <(COMPLETE=zsh bt)' >> ~/.zshrc
  Fish:  echo 'COMPLETE=fish bt | source' >> ~/.config/fish/config.fish

STATIC COMPLETIONS
==================
Static completions generate a script once. Faster but less features.

Setup:
  Bash:       bt --completions bash >> ~/.bashrc
  Zsh:        bt --completions zsh > ~/.zfunc/_bt
  Fish:       bt --completions fish > ~/.config/fish/completions/bt.fish
  PowerShell: bt --completions powershell >> $PROFILE

After setup, restart your shell or source the file to activate completions.
"#
    );
}

/// Creates a path completer that filters for image files.
///
/// Completes files with extensions: png, jpg, jpeg, gif (case-insensitive).
/// Also completes directories to allow navigation.
fn image_completer() -> PathCompleter {
    PathCompleter::any().filter(|path| {
        // Always allow directories for navigation
        if path.is_dir() {
            return true;
        }

        // Check for image extensions
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                let ext_lower = ext.to_lowercase();
                matches!(ext_lower.as_str(), "png" | "jpg" | "jpeg" | "gif")
            })
    })
}

/// Render an image to the terminal.
///
/// Supports width specification syntax: "file.jpg|50%" or "file.jpg|80"
fn render_image(image_spec: &str) -> color_eyre::Result<()> {
    // Parse the filepath and optional width
    let (filepath, width_spec) = parse_filepath_and_width(image_spec)
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    // Resolve path relative to CWD
    let path = Path::new(&filepath);

    // Create the terminal image
    let mut term_image = TerminalImage::new(path)
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    // Apply width if specified
    if let Some(ref ws) = width_spec {
        term_image.width = parse_width_spec(ws)
            .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
        term_image.width_raw = Some(format!("|{}", ws));
    }

    // Get terminal capabilities
    let terminal = Terminal::new();

    // Render the image
    let output = term_image.render_to_terminal(&terminal)
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    // Output the result
    print!("{}", output);

    Ok(())
}

/// Render a flowchart to the terminal.
///
/// Creates a Mermaid flowchart with the given content and renders it
/// using the MermaidRenderer. Default direction is left-right (LR),
/// use `vertical` for top-down (TD).
fn render_flowchart(
    vertical: bool,
    inverse: bool,
    content: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;

    let direction = if vertical { "TD" } else { "LR" };
    let body = content.join(" ");
    let instructions = format!("flowchart {}\n    {}", direction, body);

    if json {
        let output = serde_json::json!({
            "type": "flowchart",
            "direction": direction,
            "inverse": inverse,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer based on inverse flag
    let renderer = if inverse {
        // Inverse: solid background with opposite theme
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        // Default: transparent background with theme matching terminal
        MermaidRenderer::for_terminal(&instructions)
    };

    // Render the diagram to a temp PNG file
    let png_path = match renderer.render_to_temp_png() {
        Ok(path) => path,
        Err(e) => {
            return handle_flowchart_error(e, &instructions);
        }
    };

    // Use TerminalImage to display (same approach as `bt image`)
    // This handles terminal detection and fallback gracefully
    let terminal = Terminal::new();
    let term_image = TerminalImage::new(&png_path)
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    match term_image.render_to_terminal(&terminal) {
        Ok(output) => print!("{}", output),
        Err(e) => {
            // Clean up before returning error
            let _ = std::fs::remove_file(&png_path);
            return Err(color_eyre::eyre::eyre!("Failed to display flowchart: {}", e));
        }
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&png_path);

    Ok(())
}

/// Handle flowchart rendering errors with user-friendly output.
///
/// Parses mmdc errors to extract syntax information and formats
/// them nicely without JavaScript callstacks.
fn handle_flowchart_error(
    error: biscuit_terminal::components::mermaid::MermaidRenderError,
    instructions: &str,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidRenderError;

    // Check for NO_COLOR
    let no_color = std::env::var("NO_COLOR").is_ok();
    let red = if no_color { "" } else { "\x1b[31m" };
    let bold = if no_color { "" } else { "\x1b[1m" };
    let dim = if no_color { "" } else { "\x1b[2m" };
    let reset = if no_color { "" } else { "\x1b[0m" };

    match error {
        MermaidRenderError::MmdcExecutionFailed { stderr, .. } => {
            // Check if this is a parse/syntax error
            if stderr.contains("Parse error") || stderr.contains("Expecting") {
                // Add breathing room before error
                eprintln!();
                eprintln!("{}{}Error:{} Mermaid Syntax Error\n", red, bold, reset);

                // Extract useful lines from stderr (skip JS callstack and useless line numbers)
                for line in stderr.lines() {
                    // Include the context line that shows actual mermaid code (starts with ...)
                    if line.starts_with("...") {
                        eprintln!("{}", line);
                    }
                    // Include the error pointer line (contains ^ and dashes)
                    else if line.contains("^") && line.chars().filter(|c| *c == '-').count() > 3 {
                        eprintln!("{}", line);
                    }
                    // Include the "Expecting" line
                    else if line.starts_with("Expecting") || line.contains("Expecting '") {
                        eprintln!("{}", line);
                    }
                    // Skip "Error: Parse error on line X:" - not useful to CLI users
                    // Skip JS callstack lines (contain file paths or "at ")
                }

                // Show the mermaid block that was defined
                eprintln!(
                    "\n{}Mermaid block was defined as:{}\n",
                    dim, reset
                );
                eprintln!("```mermaid\n{}\n```", instructions);
            } else {
                // Non-syntax error, show the full error (with breathing room)
                eprintln!();
                eprintln!("{}{}Error:{} {}", red, bold, reset, stderr);
            }
        }
        MermaidRenderError::MmdcNotFound => {
            eprintln!(
                "{}{}Error:{} mmdc CLI not found.\n\nInstall with: npm install -g @mermaid-js/mermaid-cli",
                red, bold, reset
            );
        }
        MermaidRenderError::NpmNotFound => {
            eprintln!(
                "{}{}Error:{} npm not found.\n\nInstall Node.js and npm to render Mermaid diagrams.",
                red, bold, reset
            );
        }
        _ => {
            eprintln!("{}{}Error:{} {}", red, bold, reset, error);
        }
    }

    // Return error to get non-zero exit code
    std::process::exit(1);
}

fn collect_metadata() -> TerminalMetadata {
    let terminal = Terminal::new();

    // Get colors
    let bg_color = osc_queries::bg_color().map(|c| ColorInfo {
        r: c.r,
        g: c.g,
        b: c.b,
        hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
    });

    let text_color = osc_queries::text_color().map(|c| ColorInfo {
        r: c.r,
        g: c.g,
        b: c.b,
        hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
    });

    let cursor_color = osc_queries::cursor_color().map(|c| ColorInfo {
        r: c.r,
        g: c.g,
        b: c.b,
        hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
    });

    // Get distro info
    let distro = terminal.distro.as_ref().map(|d| DistroInfo {
        id: d.id.clone(),
        name: d.name.clone(),
        version: d.version.clone(),
        codename: d.codename.clone(),
        family: d.family.to_string(),
    });

    TerminalMetadata {
        app: format!("{:?}", terminal.app),
        os: terminal.os.to_string(),
        distro,
        width: Terminal::width(),
        height: Terminal::height(),
        is_tty: terminal.is_tty,
        is_ci: terminal.is_ci,
        color_depth: format!("{:?}", terminal.color_depth),
        color_mode: format!("{:?}", Terminal::color_mode()),
        bg_color,
        text_color,
        cursor_color,
        font: terminal.font.clone(),
        font_size: terminal.font_size,
        is_nerd_font: terminal.is_nerd_font,
        font_ligatures: terminal
            .font_ligatures
            .as_ref()
            .map(|ligatures| ligatures.iter().map(|l| format!("{:?}", l)).collect()),
        ligatures_likely: fonts::ligature_support_likely(),

        supports_italic: terminal.supports_italic,
        image_support: format!("{:?}", terminal.image_support),
        underline_support: UnderlineInfo {
            straight: terminal.underline_support.straight,
            double: terminal.underline_support.double,
            curly: terminal.underline_support.curly,
            dotted: terminal.underline_support.dotted,
            dashed: terminal.underline_support.dashed,
            colored: terminal.underline_support.colored,
        },
        osc_link_support: terminal.osc_link_support,
        osc10_fg_color: osc_queries::osc10_support(),
        osc11_bg_color: osc_queries::osc11_support(),
        osc12_cursor_color: osc_queries::osc12_support(),
        osc52_clipboard: clipboard::osc52_support(),
        mode_2027_graphemes: mode_2027::supports_mode_2027(),
        multiplex: format_multiplex(multiplex_support()),
        connection: format_connection(&terminal.remote),
        locale_raw: terminal.locale.raw().map(|s| s.to_string()),
        locale_tag: terminal.locale.tag().map(|s| s.to_string()),
        char_encoding: format!("{:?}", terminal.char_encoding),
        config_file: terminal
            .config_file
            .as_ref()
            .map(|p| p.display().to_string()),
    }
}

fn analyze_content(content: &str) -> ContentAnalysis {
    let stripped = escape_codes::strip_escape_codes(content);
    let line_lengths: Vec<u32> = stripped
        .split('\n')
        .map(|line| line.chars().count() as u32)
        .collect();
    let line_count = line_lengths.len() as u32;
    let total_length = line_lengths.iter().copied().sum();

    ContentAnalysis {
        line_count,
        line_lengths,
        contains_color_escape_codes: escape_codes::strip_color_codes(content) != content,
        contains_osc8_links: eval::has_osc8_link(content),
        total_length,
    }
}

fn print_content_analysis(analysis: &ContentAnalysis) {
    let no_color = std::env::var("NO_COLOR").is_ok();
    let bold = if no_color { "" } else { "\x1b[1m" };
    let dim = if no_color { "" } else { "\x1b[2m" };
    let reset = if no_color { "" } else { "\x1b[0m" };
    let green = if no_color { "" } else { "\x1b[32m" };

    let yes = format!("{}yes{}", green, reset);
    let no_mark = format!("{}no{}", dim, reset);
    let check = |b: bool| if b { &yes } else { &no_mark };

    let line_lengths = analysis
        .line_lengths
        .iter()
        .map(|len| len.to_string())
        .collect::<Vec<String>>()
        .join(", ");

    println!();
    println!("{}Content Analysis{}", bold, reset);
    println!("{}══════════════════{}", dim, reset);
    println!("  Lines:        {}", analysis.line_count);
    println!("  Line lengths: {}", line_lengths);
    println!("  Total length: {}", analysis.total_length);
    println!(
        "  Color codes:  {}",
        check(analysis.contains_color_escape_codes)
    );
    println!("  OSC8 links:   {}", check(analysis.contains_osc8_links));
    println!();
}

fn format_connection(conn: &Connection) -> ConnectionInfo {
    match conn {
        Connection::Local => ConnectionInfo::Local,
        Connection::SshClient(ssh) => ConnectionInfo::Ssh {
            host: ssh.host.clone(),
            source_port: ssh.source_port,
            server_port: ssh.server_port,
            tty_path: ssh.tty_path.clone(),
        },
        Connection::MoshClient(mosh) => ConnectionInfo::Mosh {
            connection: mosh.connection.clone(),
        },
    }
}

fn format_multiplex(m: MultiplexSupport) -> String {
    match m {
        MultiplexSupport::None => "None".to_string(),
        MultiplexSupport::Native { .. } => "Native".to_string(),
        MultiplexSupport::Tmux { .. } => "tmux".to_string(),
        MultiplexSupport::Zellij { .. } => "Zellij".to_string(),
    }
}

fn print_pretty(metadata: &TerminalMetadata, verbose: bool) {
    // Respect NO_COLOR environment variable
    let no_color = std::env::var("NO_COLOR").is_ok();

    let bold = if no_color { "" } else { "\x1b[1m" };
    let dim = if no_color { "" } else { "\x1b[2m" };
    let reset = if no_color { "" } else { "\x1b[0m" };
    let green = if no_color { "" } else { "\x1b[32m" };
    let yellow = if no_color { "" } else { "\x1b[33m" };
    let blue = if no_color { "" } else { "\x1b[34m" };

    println!();
    println!("{}Terminal Metadata{}", bold, reset);
    println!("{}═══════════════════════════════════════{}", dim, reset);

    // Basic info section
    println!("\n{}{}Basic Info{}", bold, blue, reset);
    println!("  App:        {}", metadata.app);
    println!("  OS:         {}", metadata.os);
    if let Some(distro) = &metadata.distro {
        println!("  Distro:     {} ({})", distro.name, distro.family);
    }
    println!("  Size:       {} x {}", metadata.width, metadata.height);
    println!(
        "  Is TTY:     {}",
        if metadata.is_tty {
            format!("{}yes{}", green, reset)
        } else {
            "no".to_string()
        }
    );
    println!(
        "  In CI:      {}",
        if metadata.is_ci {
            format!("{}yes{}", yellow, reset)
        } else {
            "no".to_string()
        }
    );

    // Font section (always displayed)
    println!("\n{}{}Fonts{}", bold, blue, reset);
    if let Some(font) = &metadata.font {
        println!("  Name:       {}", font);
    } else {
        println!("  Name:       {}", format!("{}n/a{}", dim, reset));
    }
    if let Some(size) = metadata.font_size {
        println!("  Size:       {}pt", size);
    } else {
        println!("  Size:       {}", format!("{}n/a{}", dim, reset));
    }
    println!(
        "  Nerd Font:  {}",
        match metadata.is_nerd_font {
            Some(true) => format!("{}yes{}", green, reset),
            Some(false) => "no".to_string(),
            None => format!("{}unknown{}", dim, reset),
        }
    );
    println!(
        "  Ligatures:  {}",
        if metadata.ligatures_likely {
            format!("{}likely{}", green, reset)
        } else {
            format!("{}unlikely{}", dim, reset)
        }
    );

    // Color section
    println!("\n{}{}Colors{}", bold, blue, reset);
    println!("  Depth:      {}", metadata.color_depth);
    println!("  Mode:       {}", metadata.color_mode);
    if let Some(bg) = &metadata.bg_color {
        println!(
            "  Background: {} ({}, {}, {})",
            bg.hex.as_ref().unwrap_or(&"?".to_string()),
            bg.r,
            bg.g,
            bg.b
        );
    }
    if let Some(fg) = &metadata.text_color {
        println!(
            "  Foreground: {} ({}, {}, {})",
            fg.hex.as_ref().unwrap_or(&"?".to_string()),
            fg.r,
            fg.g,
            fg.b
        );
    }
    if let Some(cursor) = &metadata.cursor_color {
        println!(
            "  Cursor:     {} ({}, {}, {})",
            cursor.hex.as_ref().unwrap_or(&"?".to_string()),
            cursor.r,
            cursor.g,
            cursor.b
        );
    }

    // Features section
    println!("\n{}{}Features{}", bold, blue, reset);
    let yes = format!("{}yes{}", green, reset);
    let no_mark = format!("{}no{}", dim, reset);
    let check = |b: bool| if b { &yes } else { &no_mark };

    println!("  Italics:      {}", check(metadata.supports_italic));
    println!("  Images:       {}", metadata.image_support);
    println!("  OSC8 Links:   {}", check(metadata.osc_link_support));
    println!("  OSC10 FG:     {}", check(metadata.osc10_fg_color));
    println!("  OSC11 BG:     {}", check(metadata.osc11_bg_color));
    println!("  OSC12 Cursor: {}", check(metadata.osc12_cursor_color));
    println!("  OSC52 Clip:   {}", check(metadata.osc52_clipboard));
    println!("  Mode 2027:    {}", check(metadata.mode_2027_graphemes));

    // Underline section
    println!("\n{}{}Underline Support{}", bold, blue, reset);
    println!(
        "  Straight:   {}",
        check(metadata.underline_support.straight)
    );
    println!("  Double:     {}", check(metadata.underline_support.double));
    println!("  Curly:      {}", check(metadata.underline_support.curly));
    println!("  Dotted:     {}", check(metadata.underline_support.dotted));
    println!("  Dashed:     {}", check(metadata.underline_support.dashed));
    println!(
        "  Colored:    {}",
        check(metadata.underline_support.colored)
    );

    // Reserved for future verbose-only output
    let _ = verbose;

    // Multiplexing
    println!("\n{}{}Multiplexing{}", bold, blue, reset);
    println!("  Type:       {}", metadata.multiplex);

    // Connection
    println!("\n{}{}Connection{}", bold, blue, reset);
    match &metadata.connection {
        ConnectionInfo::Local => {
            println!("  Type:       {}Local{}", green, reset);
        }
        ConnectionInfo::Ssh {
            host,
            source_port,
            server_port,
            tty_path,
        } => {
            println!("  Type:       {}SSH{}", yellow, reset);
            println!("  Host:       {}", host);
            println!("  Ports:      {} -> {}", source_port, server_port);
            if let Some(tty) = tty_path {
                println!("  TTY:        {}", tty);
            }
        }
        ConnectionInfo::Mosh { connection } => {
            println!("  Type:       {}Mosh{}", yellow, reset);
            println!("  Connection: {}", connection);
        }
    }

    // Locale & Encoding
    println!("\n{}{}Locale{}", bold, blue, reset);
    let na = format!("{}n/a{}", dim, reset);
    println!(
        "  Raw:        {}",
        metadata.locale_raw.as_deref().unwrap_or(&na)
    );
    println!(
        "  Tag:        {}",
        metadata.locale_tag.as_deref().unwrap_or(&na)
    );
    println!("  Encoding:   {}", metadata.char_encoding);

    // Config
    if let Some(config) = &metadata.config_file {
        println!("\n{}{}Config{}", bold, blue, reset);
        println!("  File:       {}", config);
    }

    println!();
}
