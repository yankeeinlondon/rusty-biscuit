use crate::*;
use biscuit_terminal::discovery::detection::{Connection, MultiplexSupport, multiplex_support};
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
    pub font_ligatures: Option<Vec<String>>,
    /// Whether the terminal likely supports font ligatures (heuristic)
    pub ligatures_likely: bool,

    /// Supported color depth
    pub color_depth: String,
    /// Light/dark mode
    pub color_mode: String,
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
    pub image_support: String,
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
    pub multiplex: String,

    /// Connection type (Local, SSH, Mosh)
    pub connection: ConnectionInfo,
    /// Raw locale string from environment (e.g., "en_US.UTF-8", "C")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale_raw: Option<String>,
    /// Normalized locale tag (BCP47 format, e.g., "en-US", "und" for C/POSIX)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale_tag: Option<String>,
    /// Character encoding
    pub char_encoding: String,

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
        width: terminal.width(),
        height: terminal.height(),
        is_tty: terminal.is_tty,
        is_ci: terminal.is_ci,
        in_repo: terminal.in_repo,
        in_monorepo: terminal.in_monorepo,
        repo_root: terminal.repo_root.as_ref().map(|p| p.display().to_string()),
        package_root: terminal.package_root.clone(),
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

pub fn format_multiplex(m: MultiplexSupport) -> String {
    match m {
        MultiplexSupport::None => "None".to_string(),
        MultiplexSupport::Native { .. } => "Native".to_string(),
        MultiplexSupport::Tmux { .. } => "tmux".to_string(),
        MultiplexSupport::Zellij { .. } => "Zellij".to_string(),
    }
}

pub fn print_pretty(metadata: &TerminalMetadata, verbose: bool) {
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

    println!("\n{}{}Repository{}", bold, blue, reset);
    println!(
        "  In Repo:    {}",
        if metadata.in_repo {
            format!("{}yes{}", green, reset)
        } else {
            "no".to_string()
        }
    );
    println!(
        "  Monorepo:   {}",
        if metadata.in_monorepo {
            format!("{}yes{}", green, reset)
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
    println!("\n{}{}Fonts{}", bold, blue, reset);
    if let Some(font) = &metadata.font {
        println!("  Name:       {}", font);
    } else {
        println!("  Name:       {}n/a{}", dim, reset);
    }
    if let Some(size) = metadata.font_size {
        println!("  Size:       {}pt", size);
    } else {
        println!("  Size:       {}n/a{}", dim, reset);
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

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::discovery::detection::{Connection, MultiplexSupport};

    #[test]
    fn test_analyze_content() {
        let text = "Hello \x1b[31mWorld\x1b[0m";
        let analysis = analyze_content(text);
        assert_eq!(analysis.total_length, 11);
        assert_eq!(analysis.line_count, 1);
        assert_eq!(analysis.contains_color_escape_codes, true);

        let clean_text = "Just plain text\nwith two lines";
        let analysis_clean = analyze_content(clean_text);
        assert_eq!(analysis_clean.total_length, 29);
        assert_eq!(analysis_clean.line_count, 2);
        assert_eq!(analysis_clean.contains_color_escape_codes, false);
    }

    #[test]
    fn test_format_multiplex() {
        assert_eq!(format_multiplex(MultiplexSupport::None), "None");
        assert_eq!(
            format_multiplex(MultiplexSupport::Tmux {
                split_window: true,
                resize_pane: true,
                focus_pane: true,
                multiple_windows: true,
                session_persistence: true,
                detach_session: true,
            }),
            "tmux"
        );
        assert_eq!(
            format_multiplex(MultiplexSupport::Zellij {
                split_window: true,
                resize_pane: true,
                focus_pane: true,
                multiple_tabs: true,
                session_resurrection: true,
                floating_panes: true,
                detach_session: true,
            }),
            "Zellij"
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
            color_depth: "TrueColor".to_string(),
            color_mode: "Dark".to_string(),
            bg_color: None,
            text_color: None,
            cursor_color: None,
            font: Some("Fira Code".to_string()),
            font_size: Some(12),
            is_nerd_font: Some(true),
            font_ligatures: None,
            ligatures_likely: false,
            supports_italic: true,
            image_support: "Kitty".to_string(),
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
            multiplex: "None".to_string(),
            connection: ConnectionInfo::Local,
            locale_raw: None,
            locale_tag: None,
            char_encoding: "UTF-8".to_string(),
            config_file: None,
        };

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"app\":\"test-app\""));
        assert!(json.contains("\"os\":\"linux\""));
        assert!(json.contains("\"width\":80"));
        assert!(json.contains("\"height\":24"));
        assert!(json.contains("\"curly\":true"));
    }
}
