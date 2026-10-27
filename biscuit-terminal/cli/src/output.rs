use crate::*;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::discovery::detection::{
    ColorDepth, ColorMode, Connection, GraphicsRoute, ImageSupport, MultiplexSupport,
    multiplex_support,
};
use biscuit_terminal::discovery::fonts::FontLigature;
use biscuit_terminal::discovery::locale::CharEncoding;
use biscuit_terminal::terminal::Terminal;
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
    /// Image rendering support, projected from `graphics_route`
    pub image_support: ImageSupport,
    /// How an image reaches the screen, or why it cannot
    pub graphics_route: GraphicsRoute,
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
    /// `measured` when the terminal answered the OSC query, else `estimated`
    pub source: osc_queries::ColorSource,
}

impl From<(osc_queries::RgbValue, osc_queries::ColorSource)> for ColorInfo {
    fn from((c, source): (osc_queries::RgbValue, osc_queries::ColorSource)) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
            source,
        }
    }
}

/// A color report line: `label`, the color, and whether it was measured.
fn color_line(label: &str, c: &ColorInfo) -> String {
    let source = match c.source {
        osc_queries::ColorSource::Measured => "measured",
        osc_queries::ColorSource::Estimated => "estimated",
    };
    format!(
        "{label}: <b>{}</b> ({}, {}, {}) <dim>{source}</dim>",
        c.hex.as_deref().unwrap_or("?"),
        c.r,
        c.g,
        c.b
    )
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
    let bg_color = osc_queries::bg_color_with_source().map(ColorInfo::from);
    let text_color = osc_queries::text_color_with_source().map(ColorInfo::from);
    let cursor_color = osc_queries::cursor_color_with_source().map(ColorInfo::from);

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
        color_depth: terminal.color_depth,
        color_mode: terminal.color_mode(),
        bg_color,
        text_color,
        cursor_color,
        font: terminal.font.clone(),
        font_size: terminal.font_size,
        is_nerd_font: terminal.is_nerd_font,
        font_ligatures: terminal.font_ligatures.clone(),
        ligatures_likely: fonts::ligature_support_likely(),

        supports_italic: terminal.supports_italic,
        image_support: terminal.image_support(),
        graphics_route: terminal.graphics_route.clone(),
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

pub fn print_content_analysis(analysis: &ContentAnalysis, term: &Terminal) {
    println!("{}", render_content_analysis(analysis, term));
}

/// Renders the content analysis report without printing it.
pub fn render_content_analysis(analysis: &ContentAnalysis, term: &Terminal) -> String {
    let line_lengths = analysis
        .line_lengths
        .iter()
        .map(|len| len.to_string())
        .collect::<Vec<String>>()
        .join(", ");

    let yes = Prose::new("<green>yes</green>");
    let no = Prose::new("no");

    let mut list = UnorderedList::empty();
    list.add(Prose::new(format!("Lines: <b>{}</b>", analysis.line_count)))
        .add(Prose::new(format!("Line lengths: <b>{}</b>", line_lengths)))
        .add(Prose::new(format!(
            "Total length: <b>{}</b>",
            analysis.total_length
        )))
        .add(Prose::new(format!(
            "Color codes: {}",
            if analysis.contains_color_escape_codes {
                yes.render(term)
            } else {
                no.render(term)
            }
        )))
        .add(Prose::new(format!(
            "OSC8 links: {}",
            if analysis.contains_osc8_links {
                yes.render(term)
            } else {
                no.render(term)
            }
        )));

    let mut section = Section::new(HeadingLevel::h2, "Content Analysis");
    section.push(list);
    section.render(term)
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

/// The pretty report's `Images:` line: the route's format, how it travels,
/// and the reason (with its fix when one is known).
fn images_line(route: &GraphicsRoute) -> String {
    let reason = Prose::escape_text(&route.reason.message);
    let fix = route
        .reason
        .fix
        .as_deref()
        .map(|fix| format!("; fix: {}", Prose::escape_text(fix)))
        .unwrap_or_default();
    let details = if route.has_pixels() {
        format!(
            "{:?}, {:?}, {:?}",
            route.transport, route.placement, route.confidence
        )
    } else {
        format!("{:?}", route.confidence)
    };
    format!(
        "Images: <b>{:?}</b> ({details}) <dim>{:?}: {reason}{fix}</dim>",
        route.format, route.reason.kind
    )
}

pub fn print_pretty(metadata: &TerminalMetadata, verbose: u8, term: &Terminal) {
    println!("{}", render_pretty(metadata, verbose, term));
}

/// Renders the default metadata report without printing it.
pub fn render_pretty(metadata: &TerminalMetadata, verbose: u8, term: &Terminal) -> String {
    let yes_no = |v: bool| -> &'static str {
        if v {
            "<green>yes</green>"
        } else {
            "no"
        }
    };

    let opt = |v: Option<&str>| -> String {
        match v {
            Some(s) => format!("<b>{}</b>", Prose::escape_text(s)),
            None => "<dim>n/a</dim>".to_string(),
        }
    };

    let nerd = |v: Option<bool>| -> &'static str {
        match v {
            Some(true) => "<green>yes</green>",
            Some(false) => "no",
            None => "<dim>unknown</dim>",
        }
    };

    let env_var = |name: &str, value: Result<String, std::env::VarError>| -> Prose {
        match value {
            Ok(v) => Prose::new(format!("{}: <b>{}</b>", name, Prose::escape_text(&v))),
            Err(_) => Prose::new(format!("{}: <dim>unset</dim>", name)),
        }
    };

    let mut root = Section::new(HeadingLevel::h1, "Terminal Metadata");

    // Basic Info
    let mut basic = Section::new(HeadingLevel::h2, "Basic Info");
    let mut basic_list = UnorderedList::empty();
    basic_list
        .add(Prose::new(format!("App: <b>{}</b>", Prose::escape_text(&metadata.app))))
        .add(Prose::new(format!("OS: <b>{}</b>", Prose::escape_text(&metadata.os))));
    if let Some(distro) = &metadata.distro {
        basic_list.add(Prose::new(format!(
            "Distro: <b>{}</b> ({})",
            Prose::escape_text(&distro.name),
            Prose::escape_text(&distro.family)
        )));
    }
    basic_list
        .add(Prose::new(format!(
            "Size: <b>{} x {}</b>",
            metadata.width, metadata.height
        )))
        .add(Prose::new(format!("Is TTY: {}", yes_no(metadata.is_tty))))
        .add(Prose::new(format!("In CI: {}", yes_no(metadata.is_ci))));
    basic.push(basic_list);
    root.push(basic);

    // Repository
    let mut repo = Section::new(HeadingLevel::h2, "Repository");
    let mut repo_list = UnorderedList::empty();
    repo_list
        .add(Prose::new(format!("In Repo: {}", yes_no(metadata.in_repo))))
        .add(Prose::new(format!(
            "Monorepo: {}",
            yes_no(metadata.in_monorepo)
        )));
    if let Some(repo_root) = &metadata.repo_root {
        repo_list.add(Prose::new(format!("Root: {}", opt(Some(repo_root)))));
    }
    if let Some(package_root) = &metadata.package_root {
        repo_list.add(Prose::new(format!("Package: {}", opt(Some(package_root)))));
    }
    repo.push(repo_list);
    root.push(repo);

    // Fonts
    let mut fonts = Section::new(HeadingLevel::h2, "Fonts");
    let mut font_list = UnorderedList::empty();
    font_list
        .add(Prose::new(format!("Name: {}", opt(metadata.font.as_deref()))))
        .add(Prose::new(format!(
            "Size: {}",
            metadata
                .font_size
                .map(|s| format!("<b>{}pt</b>", s))
                .unwrap_or_else(|| "<dim>n/a</dim>".to_string())
        )))
        .add(Prose::new(format!(
            "Nerd Font: {}",
            nerd(metadata.is_nerd_font)
        )))
        .add(Prose::new(format!(
            "Ligatures: {}",
            if metadata.ligatures_likely {
                "<green>likely</green>"
            } else {
                "<dim>unlikely</dim>"
            }
        )));
    fonts.push(font_list);
    root.push(fonts);

    // Colors
    let mut colors = Section::new(HeadingLevel::h2, "Colors");
    let mut color_list = UnorderedList::empty();
    color_list
        .add(Prose::new(format!("Depth: <b>{:?}</b>", metadata.color_depth)))
        .add(Prose::new(format!("Mode: <b>{:?}</b>", metadata.color_mode)));
    if let Some(bg) = &metadata.bg_color {
        color_list.add(Prose::new(color_line("Background", bg)));
    }
    if let Some(fg) = &metadata.text_color {
        color_list.add(Prose::new(color_line("Foreground", fg)));
    }
    if let Some(cursor) = &metadata.cursor_color {
        color_list.add(Prose::new(color_line("Cursor", cursor)));
    }
    colors.push(color_list);
    root.push(colors);

    // Features
    let mut features = Section::new(HeadingLevel::h2, "Features");
    let mut feature_list = UnorderedList::empty();
    feature_list
        .add(Prose::new(format!(
            "Italics: {}",
            yes_no(metadata.supports_italic)
        )))
        .add(Prose::new(images_line(&metadata.graphics_route)))
        .add(Prose::new(format!(
            "OSC8 Links: {}",
            yes_no(metadata.osc_link_support)
        )))
        .add(Prose::new(format!(
            "OSC10 FG: {}",
            yes_no(metadata.osc10_fg_color)
        )))
        .add(Prose::new(format!(
            "OSC11 BG: {}",
            yes_no(metadata.osc11_bg_color)
        )))
        .add(Prose::new(format!(
            "OSC12 Cursor: {}",
            yes_no(metadata.osc12_cursor_color)
        )))
        .add(Prose::new(format!(
            "OSC52 Clip: {}",
            yes_no(metadata.osc52_clipboard)
        )))
        .add(Prose::new(format!(
            "Mode 2027: {}",
            yes_no(metadata.mode_2027_graphemes)
        )));
    features.push(feature_list);
    root.push(features);

    // Underline Support
    let mut underline = Section::new(HeadingLevel::h2, "Underline Support");
    let mut underline_list = UnorderedList::empty();
    underline_list
        .add(Prose::new(format!(
            "Straight: {}",
            yes_no(metadata.underline_support.straight)
        )))
        .add(Prose::new(format!(
            "Double: {}",
            yes_no(metadata.underline_support.double)
        )))
        .add(Prose::new(format!(
            "Curly: {}",
            yes_no(metadata.underline_support.curly)
        )))
        .add(Prose::new(format!(
            "Dotted: {}",
            yes_no(metadata.underline_support.dotted)
        )))
        .add(Prose::new(format!(
            "Dashed: {}",
            yes_no(metadata.underline_support.dashed)
        )))
        .add(Prose::new(format!(
            "Colored: {}",
            yes_no(metadata.underline_support.colored)
        )));
    underline.push(underline_list);
    root.push(underline);

    // Multiplexing
    let mut multiplex = Section::new(HeadingLevel::h2, "Multiplexing");
    let mut multiplex_list = UnorderedList::empty();
    multiplex_list.add(Prose::new(format!(
        "Type: <b>{:?}</b>",
        metadata.multiplex
    )));
    multiplex.push(multiplex_list);
    root.push(multiplex);

    // Connection
    let mut connection = Section::new(HeadingLevel::h2, "Connection");
    let mut conn_list = UnorderedList::empty();
    match &metadata.connection {
        ConnectionInfo::Local => {
            conn_list.add(Prose::new("Type: <green>Local</green>"));
        }
        ConnectionInfo::Ssh {
            host,
            source_port,
            server_port,
            tty_path,
        } => {
            conn_list
                .add(Prose::new("Type: <yellow>SSH</yellow>"))
                .add(Prose::new(format!(
                    "Host: <b>{}</b>",
                    Prose::escape_text(host)
                )))
                .add(Prose::new(format!(
                    "Ports: {} -> {}",
                    source_port, server_port
                )));
            if let Some(tty) = tty_path {
                conn_list.add(Prose::new(format!(
                    "TTY: <b>{}</b>",
                    Prose::escape_text(tty)
                )));
            }
        }
        ConnectionInfo::Mosh { connection } => {
            conn_list
                .add(Prose::new("Type: <yellow>Mosh</yellow>"))
                .add(Prose::new(format!(
                    "Connection: <b>{}</b>",
                    Prose::escape_text(connection)
                )));
        }
    }
    connection.push(conn_list);
    root.push(connection);

    // Locale & Encoding
    let mut locale_section = Section::new(HeadingLevel::h2, "Locale");
    let mut locale_list = UnorderedList::empty();
    locale_list
        .add(Prose::new(format!(
            "Raw: {}",
            opt(metadata.locale_raw.as_deref())
        )))
        .add(Prose::new(format!(
            "Tag: {}",
            opt(metadata.locale_tag.as_deref())
        )))
        .add(Prose::new(format!(
            "Encoding: <b>{:?}</b>",
            metadata.char_encoding
        )));
    locale_section.push(locale_list);
    root.push(locale_section);

    // Config
    if let Some(config) = &metadata.config_file {
        let mut config_section = Section::new(HeadingLevel::h2, "Config");
        let mut config_list = UnorderedList::empty();
        config_list.add(Prose::new(format!("File: {}", opt(Some(config)))));
        config_section.push(config_list);
        root.push(config_section);
    }

    // Verbose-only: environment details
    if verbose >= 1 {
        let mut env = Section::new(HeadingLevel::h2, "Environment");
        let mut env_list = UnorderedList::empty();
        env_list
            .add(env_var("TERM", std::env::var("TERM")))
            .add(env_var("TERM_PROGRAM", std::env::var("TERM_PROGRAM")))
            .add(env_var("COLORTERM", std::env::var("COLORTERM")));
        let no_color_item = if std::env::var("NO_COLOR").is_ok() {
            Prose::new("NO_COLOR: <yellow>set</yellow>")
        } else {
            Prose::new("NO_COLOR: <dim>unset</dim>")
        };
        env_list.add(no_color_item);
        env.push(env_list);
        root.push(env);
    }

    // Very verbose: raw detection values
    if verbose >= 2 {
        let mut raw = Section::new(HeadingLevel::h2, "Raw Detection");
        let mut raw_list = UnorderedList::empty();
        raw_list
            .add(env_var(
                "TERM_PROGRAM_VERSION",
                std::env::var("TERM_PROGRAM_VERSION"),
            ))
            .add(env_var("LANG", std::env::var("LANG")))
            .add(env_var("LC_ALL", std::env::var("LC_ALL")))
            .add(env_var("SSH_CLIENT", std::env::var("SSH_CLIENT")))
            .add(env_var("TMUX", std::env::var("TMUX")));
        raw.push(raw_list);
        root.push(raw);
    }

    root.render(term)
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::discovery::detection::{
        ColorDepth, ColorMode, GraphicsFormat, GraphicsPlacement, GraphicsTransport,
        ImageSupport, MultiplexSupport, RouteConfidence, RouteReason, RouteReasonKind,
        TerminalApp,
    };
    use biscuit_terminal::discovery::locale::CharEncoding;
    use biscuit_terminal::terminal::Terminal;

    fn kitty_route() -> GraphicsRoute {
        GraphicsRoute {
            format: GraphicsFormat::Kitty,
            transport: GraphicsTransport::Direct,
            placement: GraphicsPlacement::Ordinary,
            confidence: RouteConfidence::Known,
            reason: RouteReason {
                kind: RouteReasonKind::Supported,
                message: "Kitty identified by TERM".to_string(),
                fix: None,
            },
            needs_measured_cells: false,
            needs_cursor_resync: false,
        }
    }

    /// Under tmux the background is the per-app default with no OSC 11
    /// reply; the report must not present it as the terminal's answer.
    #[test]
    fn color_lines_say_whether_the_terminal_reported_the_color() {
        use biscuit_terminal::discovery::osc_queries::{ColorSource, RgbValue};
        let guess = ColorInfo::from((RgbValue::new(30, 30, 30), ColorSource::Estimated));
        assert_eq!(
            color_line("Background", &guess),
            "Background: <b>#1e1e1e</b> (30, 30, 30) <dim>estimated</dim>"
        );
        let answer = ColorInfo::from((RgbValue::new(40, 44, 52), ColorSource::Measured));
        assert!(color_line("Background", &answer).ends_with("<dim>measured</dim>"));

        let json = serde_json::to_string(&guess).unwrap();
        assert!(json.contains("\"source\":\"estimated\""), "{json}");
    }

    #[test]
    fn images_line_shows_route_details_for_a_pixel_route() {
        let line = images_line(&kitty_route());

        assert!(line.contains("<b>Kitty</b>"), "{line}");
        assert!(line.contains("Direct, Ordinary, Known"), "{line}");
        assert!(line.contains("Kitty identified by TERM"), "{line}");
    }

    #[test]
    fn images_line_shows_reason_and_fix_when_no_pixels() {
        let mut route = GraphicsRoute::unavailable(
            RouteReasonKind::BlockedTransport,
            RouteConfidence::Known,
            "tmux drops graphics",
        );
        route.reason.fix = Some("set allow-passthrough on".to_string());

        let line = images_line(&route);

        assert!(line.contains("<b>None</b>"), "{line}");
        assert!(line.contains("BlockedTransport: tmux drops graphics"), "{line}");
        assert!(line.contains("fix: set allow-passthrough on"), "{line}");
    }

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
            graphics_route: kitty_route(),
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
        assert!(json.contains("\"graphics_route\":{\"format\":\"Kitty\""), "{json}");
        assert!(json.contains("\"multiplex\":\"None\""));
        assert!(json.contains("\"char_encoding\":\"Utf8\""));
    }

    #[test]
    fn render_pretty_plain_mode_emits_no_ansi_escapes() {
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
            ligatures_likely: true,
            supports_italic: true,
            image_support: ImageSupport::Kitty,
            graphics_route: kitty_route(),
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
            locale_raw: Some("en_US.UTF-8".to_string()),
            locale_tag: Some("en-US".to_string()),
            char_encoding: CharEncoding::Utf8,
            config_file: Some("/tmp/config.toml".to_string()),
        };

        let plain_term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .width(80)
            .build();
        let output = render_pretty(&meta, 0, &plain_term);
        assert!(
            !output.contains('\x1b'),
            "plain output must contain no ANSI escapes, got: {output:?}"
        );
        assert!(output.contains("Terminal Metadata"));
        assert!(output.contains("test-app"));
        assert!(output.contains("yes"));
    }

    #[test]
    fn render_pretty_non_plain_mode_emits_ansi_escapes() {
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
            ligatures_likely: true,
            supports_italic: true,
            image_support: ImageSupport::Kitty,
            graphics_route: kitty_route(),
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
            locale_raw: Some("en_US.UTF-8".to_string()),
            locale_tag: Some("en-US".to_string()),
            char_encoding: CharEncoding::Utf8,
            config_file: Some("/tmp/config.toml".to_string()),
        };

        let color_term = Terminal::builder()
            .color_depth(ColorDepth::TrueColor)
            .width(80)
            .build();
        let output = render_pretty(&meta, 0, &color_term);
        assert!(
            output.contains('\x1b'),
            "non-plain output should contain ANSI escapes for styled yes/no values, got: {output:?}"
        );
    }

    #[test]
    fn render_content_analysis_plain_mode_emits_no_ansi_escapes() {
        let analysis = ContentAnalysis {
            line_count: 2,
            line_lengths: vec![5, 5],
            contains_color_escape_codes: true,
            contains_osc8_links: false,
            total_length: 10,
        };

        let plain_term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .width(80)
            .build();
        let output = render_content_analysis(&analysis, &plain_term);
        assert!(
            !output.contains('\x1b'),
            "plain content analysis must contain no ANSI escapes, got: {output:?}"
        );
        assert!(output.contains("yes"));
        assert!(output.contains("no"));
    }
}
