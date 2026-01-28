//! Terminal information utility CLI.
//!
//! Displays terminal metadata and capabilities including:
//! - Terminal application detection
//! - Color depth and mode
//! - Feature support (italics, images, underlines, OSC links)
//! - Multiplexing status
//! - OS and distribution information

use biscuit_terminal::{
    discovery::{
        clipboard, detection::multiplex_support, detection::MultiplexSupport, fonts, mode_2027,
        osc_queries,
    },
    terminal::Terminal,
};
use clap::Parser;
use serde::Serialize;

/// Terminal information utility
#[derive(Parser, Debug)]
#[command(name = "bt")]
#[command(author, version, about = "Display terminal metadata and capabilities")]
struct Args {
    /// Output in JSON format
    #[arg(long)]
    json: bool,

    /// Verbose output (show more details)
    #[arg(short, long)]
    verbose: bool,
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

    /// Whether italics are supported
    supports_italic: bool,
    /// Image rendering support
    image_support: String,
    /// Underline style support
    underline_support: UnderlineInfo,
    /// OSC8 hyperlink support
    osc_link_support: bool,
    /// OSC52 clipboard support
    osc52_clipboard: bool,
    /// Mode 2027 grapheme cluster width support
    mode_2027_graphemes: bool,

    /// Multiplexer type
    multiplex: String,

    /// Path to terminal config file
    #[serde(skip_serializing_if = "Option::is_none")]
    config_file: Option<String>,
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

    // Setup logging if RUST_LOG is set
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    let args = Args::parse();
    let metadata = collect_metadata();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&metadata)?);
    } else {
        print_pretty(&metadata, args.verbose);
    }

    Ok(())
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
        font: terminal.font.clone(),
        font_size: terminal.font_size,
        is_nerd_font: terminal.is_nerd_font,
        font_ligatures: terminal.font_ligatures.as_ref().map(|ligatures| {
            ligatures.iter().map(|l| format!("{:?}", l)).collect()
        }),
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
        osc52_clipboard: clipboard::osc52_support(),
        mode_2027_graphemes: mode_2027::supports_mode_2027(),
        multiplex: format_multiplex(multiplex_support()),
        config_file: terminal.config_file.as_ref().map(|p| p.display().to_string()),
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

    // Features section
    println!("\n{}{}Features{}", bold, blue, reset);
    let yes = format!("{}yes{}", green, reset);
    let no_mark = format!("{}no{}", dim, reset);
    let check = |b: bool| if b { &yes } else { &no_mark };

    println!("  Italics:      {}", check(metadata.supports_italic));
    println!("  Images:       {}", metadata.image_support);
    println!("  OSC8 Links:   {}", check(metadata.osc_link_support));
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

    // Config
    if let Some(config) = &metadata.config_file {
        println!("\n{}{}Config{}", bold, blue, reset);
        println!("  File:       {}", config);
    }

    println!();
}
