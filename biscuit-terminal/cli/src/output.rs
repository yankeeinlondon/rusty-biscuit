use crate::*;
use biscuit_terminal::discovery::detection::{
    ColorDepth, ColorMode, Connection, ImageSupport, MultiplexSupport, multiplex_support,
};
use biscuit_terminal::discovery::fonts::FontLigature;
use biscuit_terminal::discovery::locale::CharEncoding;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TerminalMetadata {
    /// Terminal application name
    pub app: String,
    /// Operating system type
    pub os: String,
    /// Linux distribution info (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distro: Option<DistroInfo>,

    /// Terminal width in columns
    pub width: u32,
    /// Terminal height in rows
    pub height: u32,

    /// Whether stdout is connected to a TTY
    pub is_tty: bool,
    /// Whether running in a CI environment
    pub is_ci: bool,

    /// Whether the current directory is inside a git repository
    pub in_repo: bool,
    /// Whether the current repository is a monorepo
    pub in_monorepo: bool,
    /// Root path of the git repository (if detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_root: Option<String>,
    /// Root path of the package containing the current working directory (monorepos only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_root: Option<String>,

    /// Font name (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// Font size in pixels (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u32>,
    /// Whether using a Nerd Font (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_nerd_font: Option<bool>,
    /// Font ligatures (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_ligatures: Option<Vec<FontLigature>>,
    /// Whether the terminal likely supports font ligatures (heuristic)
    pub ligatures_likely: bool,

    /// Supported color depth
    pub color_depth: ColorDepth,
    /// Light/dark mode
    pub color_mode: ColorMode,
    /// Background color (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_color: Option<ColorInfo>,
    /// Text/foreground color (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<ColorInfo>,
    /// Cursor color (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_color: Option<ColorInfo>,

    /// Whether italics are supported
    pub supports_italic: bool,
    /// Image rendering support
    pub image_support: ImageSupport,
    /// Underline style support
    pub underline_support: UnderlineInfo,
    /// OSC8 hyperlink support
    pub osc_link_support: bool,
    /// OSC10 foreground color query support
    pub osc10_fg_color: bool,
    /// OSC11 background color query support
    pub osc11_bg_color: bool,
    /// OSC12 cursor color query support
    pub osc12_cursor_color: bool,
    /// OSC52 clipboard support
    pub osc52_clipboard: bool,
    /// Mode 2027 grapheme cluster width support
    pub mode_2027_graphemes: bool,

    /// Multiplexer type
    pub multiplex: MultiplexSupport,

    /// Connection type (Local, SSH, Mosh)
    pub connection: ConnectionInfo,
    /// Raw locale string from environment (e.g., "en_US.UTF-8", "C")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale_raw: Option<String>,
    /// Normalized locale tag (BCP47 format, e.g., "en-US", "und" for C/POSIX)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale_tag: Option<String>,
    /// Character encoding
    pub char_encoding: CharEncoding,

    /// Path to terminal config file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContentAnalysis {
    /// Number of lines in the content
    pub line_count: u32,
    /// Length of each line in characters (escape codes stripped)
    pub line_lengths: Vec<u32>,
    /// Whether the content contains SGR color escape codes
    pub contains_color_escape_codes: bool,
    /// Whether the content contains OSC8 links
    pub contains_osc8_links: bool,
    /// Total character length (escape codes stripped)
    pub total_length: u32,
}

/// Metadata about a rendered image or diagram.
///
/// Output to stderr as JSON when --meta flag is used.
#[derive(Debug, Serialize)]
pub struct RenderMeta {
    /// Absolute path to the rendered/loaded image file
    pub filename: String,
    /// Whether this was a cache hit (true) or generated fresh (false)
    pub cache_hit: bool,
    /// File size in bytes
    pub file_size_bytes: u64,
    /// Time to render/load in milliseconds
    pub render_time_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ConnectionInfo {
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
pub struct DistroInfo {
    /// Distribution ID (e.g., "ubuntu", "fedora")
    pub id: String,
    /// Pretty name
    pub name: String,
    /// Version number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Version codename
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codename: Option<String>,
    /// Distribution family
    pub family: String,
}

#[derive(Debug, Serialize)]
pub struct ColorInfo {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Hex color code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hex: Option<String>,
}

impl From<biscuit_terminal::discovery::osc_queries::RgbValue> for ColorInfo {
    fn from(c: biscuit_terminal::discovery::osc_queries::RgbValue) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UnderlineInfo {
    /// Straight/single underline
    pub straight: bool,
    /// Double underline
    pub double: bool,
    /// Curly/squiggly underline
    pub curly: bool,
    /// Dotted underline
    pub dotted: bool,
    /// Dashed underline
    pub dashed: bool,
    /// Colored underlines
    pub colored: bool,
}

pub fn collect_metadata() -> TerminalMetadata {
    let terminal = Terminal::new();

    // Get colors
    let bg_color = osc_queries::bg_color().map(ColorInfo::from);
    let text_color = osc_queries::text_color().map(ColorInfo::from);
    let cursor_color = osc_queries::cursor_color().map(ColorInfo::from);

    // Get distro info
    let distro = terminal.distro.as_ref().map(|d| DistroInfo {
        id: d.id.clone(),
        name: d.name.clone(),
        version: d.version.clone(),
        codename: d.codename.clone(),
        family: d.family.to_string(),
    });

    TerminalMetadata {
        app: terminal.app.to_string(),
        os: terminal.os.to_string(),
        distro,
        width: terminal.width(),
        height: terminal.height(),
        is_tty: terminal.is_tty,
        is_ci: terminal.is_ci,
        in_repo: terminal.in_repo,
        in_monorepo: terminal.in_monorepo,
        repo_root: terminal.repo_root.as_ref().map(|p| p.display().to_string()),
        package_root: terminal.package_root.clone(),
        color_depth: terminal.color_depth.clone(),
        color_mode: Terminal::color_mode(),
        bg_color,
        text_color,
        cursor_color,
        font: terminal.font.clone(),
        font_size: terminal.font_size,
        is_nerd_font: terminal.is_nerd_font,
        font_ligatures: terminal.font_ligatures.clone(),
        ligatures_likely: fonts::ligature_support_likely(),

        supports_italic: terminal.supports_italic,
        image_support: terminal.image_support.clone(),
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
        multiplex: multiplex_support(),
        connection: format_connection(&terminal.remote),
        locale_raw: terminal.locale.raw().map(|s| s.to_string()),
        locale_tag: terminal.locale.tag().map(|s| s.to_string()),
        char_encoding: terminal.char_encoding.clone(),
        config_file: terminal
            .config_file
            .as_ref()
            .map(|p| p.display().to_string()),
    }
}

pub fn analyze_content(content: &str) -> ContentAnalysis {
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

pub fn print_content_analysis(analysis: &ContentAnalysis) {
    let s = crate::types::CliStyles::detect();

    let yes = format!("{}yes{}", s.green, s.reset);
    let no_mark = format!("{}no{}", s.dim, s.reset);
    let check = |b: bool| if b { &yes } else { &no_mark };

    let line_lengths = analysis
        .line_lengths
        .iter()
        .map(|len| len.to_string())
        .collect::<Vec<String>>()
        .join(", ");

    println!();
    println!("{}Content Analysis{}", s.bold, s.reset);
    println!("{}══════════════════{}", s.dim, s.reset);
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

pub fn format_connection(conn: &Connection) -> ConnectionInfo {
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

pub fn print_pretty(metadata: &TerminalMetadata, verbose: u8) {
    let s = crate::types::CliStyles::detect();

    println!();
    println!("{}Terminal Metadata{}", s.bold, s.reset);
    println!(
        "{}═══════════════════════════════════════{}",
        s.dim, s.reset
    );

    // Basic info section
    println!("\n{}{}Basic Info{}", s.bold, s.blue, s.reset);
    println!("  App:        {}", metadata.app);
    println!("  OS:         {}", metadata.os);
    if let Some(distro) = &metadata.distro {
        println!("  Distro:     {} ({})", distro.name, distro.family);
    }
    println!("  Size:       {} x {}", metadata.width, metadata.height);
    println!(
        "  Is TTY:     {}",
        if metadata.is_tty {
            format!("{}yes{}", s.green, s.reset)
        } else {
            "no".to_string()
        }
    );
    println!(
        "  In CI:      {}",
        if metadata.is_ci {
            format!("{}yes{}", s.yellow, s.reset)
        } else {
            "no".to_string()
        }
    );

    println!("\n{}{}Repository{}", s.bold, s.blue, s.reset);
    println!(
        "  In Repo:    {}",
        if metadata.in_repo {
            format!("{}yes{}", s.green, s.reset)
        } else {
            "no".to_string()
        }
    );
    println!(
        "  Monorepo:   {}",
        if metadata.in_monorepo {
            format!("{}yes{}", s.green, s.reset)
        } else {
            "no".to_string()
        }
    );
    if let Some(repo_root) = &metadata.repo_root {
        println!("  Root:       {}", repo_root);
    }
    if let Some(package_root) = &metadata.package_root {
        println!("  Package:    {}", package_root);
    }

    // Font section (always displayed)
    println!("\n{}{}Fonts{}", s.bold, s.blue, s.reset);
    if let Some(font) = &metadata.font {
        println!("  Name:       {}", font);
    } else {
        println!("  Name:       {}n/a{}", s.dim, s.reset);
    }
    if let Some(size) = metadata.font_size {
        println!("  Size:       {}pt", size);
    } else {
        println!("  Size:       {}n/a{}", s.dim, s.reset);
    }
    println!(
        "  Nerd Font:  {}",
        match metadata.is_nerd_font {
            Some(true) => format!("{}yes{}", s.green, s.reset),
            Some(false) => "no".to_string(),
            None => format!("{}unknown{}", s.dim, s.reset),
        }
    );
    println!(
        "  Ligatures:  {}",
        if metadata.ligatures_likely {
            format!("{}likely{}", s.green, s.reset)
        } else {
            format!("{}unlikely{}", s.dim, s.reset)
        }
    );

    // Color section
    println!("\n{}{}Colors{}", s.bold, s.blue, s.reset);
    println!("  Depth:      {:?}", metadata.color_depth);
    println!("  Mode:       {:?}", metadata.color_mode);
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
    println!("\n{}{}Features{}", s.bold, s.blue, s.reset);
    let yes = format!("{}yes{}", s.green, s.reset);
    let no_mark = format!("{}no{}", s.dim, s.reset);
    let check = |b: bool| if b { &yes } else { &no_mark };

    println!("  Italics:      {}", check(metadata.supports_italic));
    println!("  Images:       {:?}", metadata.image_support);
    println!("  OSC8 Links:   {}", check(metadata.osc_link_support));
    println!("  OSC10 FG:     {}", check(metadata.osc10_fg_color));
    println!("  OSC11 BG:     {}", check(metadata.osc11_bg_color));
    println!("  OSC12 Cursor: {}", check(metadata.osc12_cursor_color));
    println!("  OSC52 Clip:   {}", check(metadata.osc52_clipboard));
    println!("  Mode 2027:    {}", check(metadata.mode_2027_graphemes));

    // Underline section
    println!("\n{}{}Underline Support{}", s.bold, s.blue, s.reset);
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

    // Multiplexing
    println!("\n{}{}Multiplexing{}", s.bold, s.blue, s.reset);
    println!("  Type:       {:?}", metadata.multiplex);

    // Connection
    println!("\n{}{}Connection{}", s.bold, s.blue, s.reset);
    match &metadata.connection {
        ConnectionInfo::Local => {
            println!("  Type:       {}Local{}", s.green, s.reset);
        }
        ConnectionInfo::Ssh {
            host,
            source_port,
            server_port,
            tty_path,
        } => {
            println!("  Type:       {}SSH{}", s.yellow, s.reset);
            println!("  Host:       {}", host);
            println!("  Ports:      {} -> {}", source_port, server_port);
            if let Some(tty) = tty_path {
                println!("  TTY:        {}", tty);
            }
        }
        ConnectionInfo::Mosh { connection } => {
            println!("  Type:       {}Mosh{}", s.yellow, s.reset);
            println!("  Connection: {}", connection);
        }
    }

    // Locale & Encoding
    println!("\n{}{}Locale{}", s.bold, s.blue, s.reset);
    let na = format!("{}n/a{}", s.dim, s.reset);
    println!(
        "  Raw:        {}",
        metadata.locale_raw.as_deref().unwrap_or(&na)
    );
    println!(
        "  Tag:        {}",
        metadata.locale_tag.as_deref().unwrap_or(&na)
    );
    println!("  Encoding:   {:?}", metadata.char_encoding);

    // Config
    if let Some(config) = &metadata.config_file {
        println!("\n{}{}Config{}", s.bold, s.blue, s.reset);
        println!("  File:       {}", config);
    }

    // Verbose-only: environment details
    if verbose >= 1 {
        println!("\n{}{}Environment{}", s.bold, s.blue, s.reset);
        println!(
            "  TERM:       {}",
            std::env::var("TERM").unwrap_or_else(|_| format!("{}unset{}", s.dim, s.reset))
        );
        println!(
            "  TERM_PROGRAM: {}",
            std::env::var("TERM_PROGRAM").unwrap_or_else(|_| format!("{}unset{}", s.dim, s.reset))
        );
        println!(
            "  COLORTERM:  {}",
            std::env::var("COLORTERM").unwrap_or_else(|_| format!("{}unset{}", s.dim, s.reset))
        );
        println!(
            "  NO_COLOR:   {}",
            if std::env::var("NO_COLOR").is_ok() {
                format!("{}set{}", s.yellow, s.reset)
            } else {
                format!("{}unset{}", s.dim, s.reset)
            }
        );
    }

    // Very verbose: raw detection values
    if verbose >= 2 {
        println!("\n{}{}Raw Detection{}", s.bold, s.blue, s.reset);
        println!(
            "  TERM_PROGRAM_VERSION: {}",
            std::env::var("TERM_PROGRAM_VERSION")
                .unwrap_or_else(|_| format!("{}unset{}", s.dim, s.reset))
        );
        println!(
            "  LANG:       {}",
            std::env::var("LANG").unwrap_or_else(|_| format!("{}unset{}", s.dim, s.reset))
        );
        println!(
            "  LC_ALL:     {}",
            std::env::var("LC_ALL").unwrap_or_else(|_| format!("{}unset{}", s.dim, s.reset))
        );
        println!(
            "  SSH_CLIENT: {}",
            std::env::var("SSH_CLIENT").unwrap_or_else(|_| format!("{}unset{}", s.dim, s.reset))
        );
        println!(
            "  TMUX:       {}",
            std::env::var("TMUX").unwrap_or_else(|_| format!("{}unset{}", s.dim, s.reset))
        );
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::discovery::detection::{
        ColorDepth, ColorMode, ImageSupport, MultiplexSupport, TerminalApp,
    };
    use biscuit_terminal::discovery::locale::CharEncoding;

    #[test]
    fn test_analyze_content() {
        let text = "Hello \x1b[31mWorld\x1b[0m";
        let analysis = analyze_content(text);
        assert_eq!(analysis.total_length, 11);
        assert_eq!(analysis.line_count, 1);
        assert!(analysis.contains_color_escape_codes);

        let clean_text = "Just plain text\nwith two lines";
        let analysis_clean = analyze_content(clean_text);
        assert_eq!(analysis_clean.total_length, 29);
        assert_eq!(analysis_clean.line_count, 2);
        assert!(!analysis_clean.contains_color_escape_codes);
    }

    #[test]
    fn test_terminal_app_display() {
        assert_eq!(TerminalApp::Kitty.to_string(), "Kitty");
        assert_eq!(TerminalApp::Ghostty.to_string(), "Ghostty");
        assert_eq!(TerminalApp::ITerm2.to_string(), "ITerm2");
        assert_eq!(TerminalApp::Other("xterm".to_string()).to_string(), "xterm");
        assert_eq!(
            TerminalApp::Other("Windows Terminal".to_string()).to_string(),
            "Windows Terminal"
        );
    }

    #[test]
    fn test_terminal_metadata_serialization() {
        let meta = TerminalMetadata {
            app: "test-app".to_string(),
            os: "linux".to_string(),
            distro: None,
            width: 80,
            height: 24,
            is_tty: true,
            is_ci: false,
            in_repo: false,
            in_monorepo: false,
            repo_root: None,
            package_root: None,
            color_depth: ColorDepth::TrueColor,
            color_mode: ColorMode::Dark,
            bg_color: None,
            text_color: None,
            cursor_color: None,
            font: Some("Fira Code".to_string()),
            font_size: Some(12),
            is_nerd_font: Some(true),
            font_ligatures: None,
            ligatures_likely: false,
            supports_italic: true,
            image_support: ImageSupport::Kitty,
            underline_support: UnderlineInfo {
                curly: true,
                dashed: false,
                dotted: false,
                colored: true,
                double: true,
                straight: true,
            },
            osc_link_support: true,
            osc10_fg_color: true,
            osc11_bg_color: true,
            osc12_cursor_color: true,
            osc52_clipboard: true,
            mode_2027_graphemes: false,
            multiplex: MultiplexSupport::None,
            connection: ConnectionInfo::Local,
            locale_raw: None,
            locale_tag: None,
            char_encoding: CharEncoding::Utf8,
            config_file: None,
        };

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"app\":\"test-app\""));
        assert!(json.contains("\"os\":\"linux\""));
        assert!(json.contains("\"width\":80"));
        assert!(json.contains("\"height\":24"));
        assert!(json.contains("\"curly\":true"));
        // Verify enum fields serialize as expected PascalCase strings
        assert!(json.contains("\"color_depth\":\"TrueColor\""));
        assert!(json.contains("\"color_mode\":\"Dark\""));
        assert!(json.contains("\"image_support\":\"Kitty\""));
        assert!(json.contains("\"multiplex\":\"None\""));
        assert!(json.contains("\"char_encoding\":\"Utf8\""));
    }
}
