//! Graphics route: how an image reaches the screen, or why it cannot.
//!
//! A [`GraphicsRoute`] replaces the single "which protocol" answer with the
//! path an image takes: the pixel format, the transport that carries it, the
//! placement style, how sure we are, and a structured reason. The
//! [`ImageSupport`] answer is a projection of the route.
//!
//! [`resolve_graphics_route`] is a pure function of [`RouteInputs`], so every
//! environment the resolution order distinguishes can be tested without
//! touching the process environment; [`RouteInputs::from_process`] gathers
//! the real inputs. The developer guide is
//! `biscuit-terminal/docs/discovery/graphics-routes.md`.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt;

use serde::{Deserialize, Serialize};

use super::image::ImageSupport;

/// Environment variable that forces or disables a graphics format.
pub const GRAPHICS_ENV: &str = "BISCUIT_GRAPHICS";
/// Environment variable that declares the transport between this program and
/// the terminal.
pub const GRAPHICS_TRANSPORT_ENV: &str = "BISCUIT_GRAPHICS_TRANSPORT";

const GRAPHICS_VALUES: &[&str] = &["auto", "kitty", "iterm2", "sixel", "none"];
const TRANSPORT_VALUES: &[&str] = &["auto", "direct", "tmux"];

/// The first Zellij release that renders Kitty graphics.
const ZELLIJ_KITTY_MIN: ToolVersion = ToolVersion::new(0, 45, 0);
/// The first Windows Terminal release that renders Sixel.
const WINDOWS_TERMINAL_SIXEL_MIN: ToolVersion = ToolVersion::new(1, 22, 0);

/// Every environment variable resolution reads.
const ROUTE_ENV_VARS: &[&str] = &[
    GRAPHICS_ENV,
    GRAPHICS_TRANSPORT_ENV,
    "TERM_PROGRAM",
    "TERM",
    "LC_TERMINAL",
    "ITERM_SESSION_ID",
    "ITERM_PROFILE",
    "WT_SESSION",
    "MSYSTEM",
    "SSH_CONNECTION",
    "SSH_CLIENT",
    "SSH_TTY",
    "TMUX",
    "ZELLIJ",
];

/// The pixel format an image is encoded in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphicsFormat {
    /// The Kitty graphics protocol.
    Kitty,
    /// iTerm2's `OSC 1337;File=` inline image format.
    Iterm2,
    /// DEC Sixel.
    Sixel,
    /// No pixel format; images use a text fallback.
    None,
}

/// How escape sequences travel from this program to the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphicsTransport {
    /// Written as-is.
    Direct,
    /// Wrapped for tmux passthrough.
    Tmux,
}

/// How a Kitty image is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphicsPlacement {
    /// Placed at the cursor.
    Ordinary,
    /// A virtual placement drawn with Unicode placeholder cells.
    Placeholders,
}

/// How the route was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RouteConfidence {
    /// Chosen by `BISCUIT_GRAPHICS` or a caller's force override.
    Forced,
    /// Confirmed by a terminal reply.
    Verified,
    /// Inferred from a terminal identity, or supplied by the caller.
    Known,
    /// Nothing established the route.
    Unknown,
}

/// Why a route was chosen, or why no pixel route exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RouteReasonKind {
    /// A pixel route was chosen.
    Supported,
    /// The terminal is known to lack a usable pixel format.
    NoSupport,
    /// Nothing established whether graphics work.
    Unknown,
    /// Graphics were disabled by `BISCUIT_GRAPHICS=none`.
    UserDisabled,
    /// Something between this program and the terminal removes the format.
    BlockedTransport,
    /// An override variable holds a value outside its accepted set.
    InvalidInput,
    /// Stdout is not a terminal, so no graphics controls are written.
    NotATerminal,
}

/// The structured reason carried by every [`GraphicsRoute`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteReason {
    pub kind: RouteReasonKind,
    /// A human-readable explanation.
    pub message: String,
    /// The change that would give graphics. Present only when the blocker was
    /// observed, never as a guess.
    pub fix: Option<String>,
}

impl fmt::Display for RouteReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.fix {
            Some(fix) => write!(f, "{} ({fix})", self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// What an image passes through to reach the screen.
///
/// The route is resolved once per [`Terminal`](crate::terminal::Terminal) and
/// is authoritative for every image rendered with it; components never
/// resolve again from the environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphicsRoute {
    pub format: GraphicsFormat,
    pub transport: GraphicsTransport,
    pub placement: GraphicsPlacement,
    pub confidence: RouteConfidence,
    pub reason: RouteReason,
}

impl GraphicsRoute {
    fn pixels(
        format: GraphicsFormat,
        confidence: RouteConfidence,
        message: impl Into<String>,
    ) -> Self {
        Self {
            format,
            transport: GraphicsTransport::Direct,
            placement: GraphicsPlacement::Ordinary,
            confidence,
            reason: RouteReason {
                kind: RouteReasonKind::Supported,
                message: message.into(),
                fix: None,
            },
        }
    }

    /// A route with no pixel format.
    #[must_use]
    pub fn unavailable(
        kind: RouteReasonKind,
        confidence: RouteConfidence,
        message: impl Into<String>,
    ) -> Self {
        Self {
            format: GraphicsFormat::None,
            transport: GraphicsTransport::Direct,
            placement: GraphicsPlacement::Ordinary,
            confidence,
            reason: RouteReason {
                kind,
                message: message.into(),
                fix: None,
            },
        }
    }

    fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.reason.fix = Some(fix.into());
        self
    }

    /// Whether the route carries a pixel format.
    #[must_use]
    pub fn has_pixels(&self) -> bool {
        self.format != GraphicsFormat::None
    }

    /// The pixel format support this route gives, as an [`ImageSupport`].
    #[must_use]
    pub fn image_support(&self) -> ImageSupport {
        match self.format {
            GraphicsFormat::Kitty => ImageSupport::Kitty,
            GraphicsFormat::Iterm2 => ImageSupport::ITerm,
            GraphicsFormat::Sixel => ImageSupport::Sixel,
            GraphicsFormat::None => ImageSupport::None,
        }
    }

    /// The route a caller's force override (the render tree's
    /// `force_graphics`, darkmatter's `TerminalImageMode::Force`) produces.
    ///
    /// A pixel route is kept as it is. A route without pixels is upgraded to
    /// Kitty only when nothing more specific than "no support" or "unknown"
    /// stopped it: the override never lifts `BISCUIT_GRAPHICS=none`, an
    /// invalid override, a blocked transport, or nonterminal stdout.
    #[must_use]
    pub fn forced_by_caller(&self) -> Self {
        match self.reason.kind {
            RouteReasonKind::NoSupport | RouteReasonKind::Unknown if !self.has_pixels() => {
                Self::pixels(
                    GraphicsFormat::Kitty,
                    RouteConfidence::Forced,
                    "Kitty forced by the caller's graphics override",
                )
            }
            _ => self.clone(),
        }
    }
}

impl From<ImageSupport> for GraphicsRoute {
    /// A caller-supplied capability: a direct, ordinary route with
    /// [`RouteConfidence::Known`].
    fn from(support: ImageSupport) -> Self {
        let format = match support {
            ImageSupport::Kitty => GraphicsFormat::Kitty,
            ImageSupport::ITerm => GraphicsFormat::Iterm2,
            ImageSupport::Sixel => GraphicsFormat::Sixel,
            ImageSupport::None => {
                return Self::unavailable(
                    RouteReasonKind::NoSupport,
                    RouteConfidence::Known,
                    "the caller supplied a terminal without image support",
                );
            }
        };
        Self::pixels(
            format,
            RouteConfidence::Known,
            "image support supplied by the caller",
        )
    }
}

/// A `major.minor.patch` version of a terminal or multiplexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ToolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ToolVersion {
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parses the first `major.minor[.patch]` run in `text`, so both
    /// `zellij 0.45.1` and `1.22.10942.0` parse. Missing parts are zero.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let start = text.find(|c: char| c.is_ascii_digit())?;
        let mut parts = text[start..]
            .split(|c: char| !c.is_ascii_digit())
            .take_while(|part| !part.is_empty());
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        Some(Self::new(major, minor, patch))
    }
}

impl fmt::Display for ToolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Everything [`resolve_graphics_route`] decides from.
///
/// Build one by hand in tests, or with [`RouteInputs::from_process`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteInputs {
    /// Whether the output graphics would be written to is a terminal.
    pub output_is_tty: bool,
    /// Whether this process runs on native Windows.
    pub is_windows: bool,
    /// `zellij --version`, read only inside Zellij; `None` when unreadable.
    pub zellij_version: Option<ToolVersion>,
    /// Whether Zellij's pane answered a Kitty query. Filled by the graphics
    /// probe (**planned**); `None` means not asked.
    pub zellij_kitty_enabled: Option<bool>,
    /// The file version of the `WindowsTerminal.exe` hosting this process,
    /// found by walking the parent process chain on native Windows; `None`
    /// when unreadable.
    pub windows_terminal_version: Option<ToolVersion>,
    vars: BTreeMap<String, OsString>,
}

impl RouteInputs {
    /// Inputs for a terminal with no environment variables set.
    #[must_use]
    pub fn new(output_is_tty: bool) -> Self {
        Self {
            output_is_tty,
            ..Self::default()
        }
    }

    /// Reads the process: stdout's TTY state, the OS, the environment
    /// variables resolution consults, and the Zellij and Windows Terminal
    /// versions where they matter.
    #[must_use]
    pub fn from_process() -> Self {
        let mut inputs = Self::new(super::dimensions::is_tty());
        inputs.is_windows = cfg!(windows);
        for &name in ROUTE_ENV_VARS {
            if let Some(value) = std::env::var_os(name) {
                inputs.vars.insert(name.to_string(), value);
            }
        }
        if inputs.var_is_set("ZELLIJ") {
            inputs.zellij_version = super::versions::zellij_version();
        }
        if inputs.is_windows && inputs.var_is_set("WT_SESSION") {
            inputs.windows_terminal_version = super::versions::windows_terminal_version();
        }
        inputs
    }

    /// Replaces the TTY state, for programs whose graphics go somewhere other
    /// than stdout (for example stderr).
    #[must_use]
    pub fn with_output_is_tty(mut self, output_is_tty: bool) -> Self {
        self.output_is_tty = output_is_tty;
        self
    }

    /// Sets an environment variable as resolution will see it.
    #[must_use]
    pub fn with_var(mut self, name: &str, value: impl Into<OsString>) -> Self {
        self.vars.insert(name.to_string(), value.into());
        self
    }

    fn var_os(&self, name: &str) -> Option<&OsStr> {
        self.vars.get(name).map(OsString::as_os_str)
    }

    fn var_is_set(&self, name: &str) -> bool {
        self.vars.contains_key(name)
    }

    /// A variable's value when it is valid UTF-8; identity hints that are not
    /// UTF-8 identify nothing.
    fn var(&self, name: &str) -> Option<&str> {
        self.var_os(name).and_then(OsStr::to_str)
    }

    fn in_ssh_session(&self) -> bool {
        ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"]
            .iter()
            .any(|name| self.var_is_set(name))
    }
}

/// A validated override variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Choice {
    /// Unset or `auto`.
    Auto,
    /// One of the variable's other accepted values.
    Value(&'static str),
}

/// The one validator both override variables share. Accepts exactly the
/// lowercase values listed; an empty, padded, differently cased, or
/// non-UTF-8 value is invalid, never read as `auto`.
fn parse_choice(
    name: &str,
    value: Option<&OsStr>,
    accepted: &'static [&'static str],
) -> Result<Choice, GraphicsRoute> {
    let Some(raw) = value else {
        return Ok(Choice::Auto);
    };
    let matched = raw
        .to_str()
        .and_then(|text| accepted.iter().copied().find(|&a| a == text));
    match matched {
        Some("auto") => Ok(Choice::Auto),
        Some(value) => Ok(Choice::Value(value)),
        None => {
            let accepted_list = accepted.join(", ");
            Err(GraphicsRoute::unavailable(
                RouteReasonKind::InvalidInput,
                RouteConfidence::Forced,
                format!(
                    "{name}={} is not a valid value; accepted values: {accepted_list}",
                    describe_value(raw)
                ),
            )
            .with_fix(format!("set {name} to one of: {accepted_list}")))
        }
    }
}

/// Quotes a value for a reason message, escaping controls so the message
/// cannot carry terminal escapes.
fn describe_value(raw: &OsStr) -> String {
    match raw.to_str() {
        Some(text) => format!("{:?}", text),
        None => "(not valid UTF-8)".to_string(),
    }
}

fn format_from_choice(value: &str) -> Option<GraphicsFormat> {
    match value {
        "kitty" => Some(GraphicsFormat::Kitty),
        "iterm2" => Some(GraphicsFormat::Iterm2),
        "sixel" => Some(GraphicsFormat::Sixel),
        _ => None,
    }
}

fn format_name(format: GraphicsFormat) -> &'static str {
    match format {
        GraphicsFormat::Kitty => "Kitty",
        GraphicsFormat::Iterm2 => "iTerm2",
        GraphicsFormat::Sixel => "Sixel",
        GraphicsFormat::None => "none",
    }
}

/// Terminal applications whose identity selects a format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KnownTerminal {
    KittyFamily(&'static str),
    Iterm2Family(&'static str),
    AppleTerminal,
}

/// `TERM_PROGRAM` values that name a terminal.
fn terminal_from_program(program: &str) -> Option<KnownTerminal> {
    const KITTY: &[&str] = &[
        "ghostty",
        "kitty",
        "WezTerm",
        "Warp",
        "WarpTerminal",
        "konsole",
        "wast",
    ];
    if program == "Apple_Terminal" {
        return Some(KnownTerminal::AppleTerminal);
    }
    if let Some(&name) = KITTY.iter().find(|k| program.eq_ignore_ascii_case(k)) {
        return Some(KnownTerminal::KittyFamily(name));
    }
    if program == "iTerm.app" || program.eq_ignore_ascii_case("iterm2") {
        return Some(KnownTerminal::Iterm2Family("iTerm2"));
    }
    if program.eq_ignore_ascii_case("mintty") {
        return Some(KnownTerminal::Iterm2Family("mintty"));
    }
    if program.eq_ignore_ascii_case("cterm") {
        return Some(KnownTerminal::Iterm2Family("cterm"));
    }
    None
}

/// `LC_TERMINAL` values (sent by the client, so forwarded over SSH).
fn terminal_from_lc_terminal(value: &str) -> Option<KnownTerminal> {
    match value {
        "iTerm2" => Some(KnownTerminal::Iterm2Family("iTerm2")),
        _ => None,
    }
}

/// The outer terminal named by the current client's information.
///
/// Inside an SSH session `LC_TERMINAL` comes from the connecting client and
/// beats `TERM_PROGRAM`, which may be the server's own stale value; locally
/// `TERM_PROGRAM` is set by the terminal running this process and wins.
fn identify_terminal(inputs: &RouteInputs) -> Option<(KnownTerminal, String)> {
    let from_program = inputs
        .var("TERM_PROGRAM")
        .and_then(|p| terminal_from_program(p).map(|t| (t, format!("TERM_PROGRAM={p}"))));
    let from_lc = inputs
        .var("LC_TERMINAL")
        .and_then(|v| terminal_from_lc_terminal(v).map(|t| (t, format!("LC_TERMINAL={v}"))));
    if inputs.in_ssh_session() {
        from_lc.or(from_program)
    } else {
        from_program.or(from_lc)
    }
}

/// Further identity hints set by the terminal itself, consulted after
/// `TERM_PROGRAM` and `LC_TERMINAL`. Not used across SSH or under a
/// multiplexer, where these values may be inherited from elsewhere.
fn identify_local_hints(inputs: &RouteInputs) -> Option<(KnownTerminal, String)> {
    if let Some(term) = inputs.var("TERM")
        && term.contains("kitty")
    {
        return Some((KnownTerminal::KittyFamily("kitty"), format!("TERM={term}")));
    }
    if inputs.var_is_set("ITERM_SESSION_ID") || inputs.var_is_set("ITERM_PROFILE") {
        return Some((
            KnownTerminal::Iterm2Family("iTerm2"),
            "ITERM_SESSION_ID or ITERM_PROFILE".to_string(),
        ));
    }
    None
}

fn known_terminal_route(terminal: KnownTerminal, source: &str) -> GraphicsRoute {
    match terminal {
        KnownTerminal::KittyFamily(name) => GraphicsRoute::pixels(
            GraphicsFormat::Kitty,
            RouteConfidence::Known,
            format!("{name} ({source}) is known to render the Kitty graphics protocol"),
        ),
        KnownTerminal::Iterm2Family(name) => GraphicsRoute::pixels(
            GraphicsFormat::Iterm2,
            RouteConfidence::Known,
            format!("{name} ({source}) is known to render the iTerm2 image format"),
        ),
        KnownTerminal::AppleTerminal => GraphicsRoute::unavailable(
            RouteReasonKind::NoSupport,
            RouteConfidence::Known,
            format!("Terminal.app ({source}) has no supported pixel format"),
        ),
    }
}

/// Resolves the graphics route for `inputs`.
///
/// Resolution order: nonterminal output, then `BISCUIT_GRAPHICS` /
/// `BISCUIT_GRAPHICS_TRANSPORT`, then the Windows SSH server rule, then the
/// multiplexer (Zellij, tmux), then the terminal's identity. What the
/// environment cannot decide resolves to [`RouteConfidence::Unknown`].
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{
///     GraphicsFormat, RouteInputs, resolve_graphics_route,
/// };
///
/// let inputs = RouteInputs::new(true).with_var("TERM_PROGRAM", "WezTerm");
/// assert_eq!(resolve_graphics_route(&inputs).format, GraphicsFormat::Kitty);
///
/// let piped = RouteInputs::new(false).with_var("TERM_PROGRAM", "WezTerm");
/// assert_eq!(resolve_graphics_route(&piped).format, GraphicsFormat::None);
/// ```
#[must_use]
pub fn resolve_graphics_route(inputs: &RouteInputs) -> GraphicsRoute {
    if !inputs.output_is_tty {
        return GraphicsRoute::unavailable(
            RouteReasonKind::NotATerminal,
            RouteConfidence::Known,
            "output is not a terminal (piped or redirected)",
        );
    }

    let format_choice = match parse_choice(GRAPHICS_ENV, inputs.var_os(GRAPHICS_ENV), GRAPHICS_VALUES) {
        Ok(choice) => choice,
        Err(route) => return route,
    };
    let transport_choice = match parse_choice(
        GRAPHICS_TRANSPORT_ENV,
        inputs.var_os(GRAPHICS_TRANSPORT_ENV),
        TRANSPORT_VALUES,
    ) {
        Ok(choice) => choice,
        Err(route) => return route,
    };

    match format_choice {
        Choice::Value("none") => GraphicsRoute::unavailable(
            RouteReasonKind::UserDisabled,
            RouteConfidence::Forced,
            "graphics disabled by BISCUIT_GRAPHICS=none",
        ),
        Choice::Value(value) => {
            let format = format_from_choice(value)
                .expect("the accepted values other than auto and none name formats");
            resolve_forced(inputs, format, transport_choice)
        }
        Choice::Auto => resolve_detected(inputs, transport_choice),
    }
}

/// Whether the path to the terminal goes through tmux, as far as the
/// environment shows.
fn tmux_in_path(inputs: &RouteInputs, transport: Choice) -> bool {
    match transport {
        Choice::Value("tmux") => true,
        Choice::Value(_) => false,
        Choice::Auto => inputs.var_is_set("TMUX"),
    }
}

fn windows_ssh_server(inputs: &RouteInputs) -> bool {
    inputs.is_windows && inputs.var_is_set("SSH_CONNECTION")
}

fn tmux_planned(requested: &str) -> GraphicsRoute {
    GraphicsRoute::unavailable(
        RouteReasonKind::Unknown,
        RouteConfidence::Unknown,
        format!("{requested}: graphics through tmux passthrough are not implemented yet (planned)"),
    )
}

/// A `BISCUIT_GRAPHICS` format: no probing and no identity preferences, but
/// transports known to remove the format still rule it out.
fn resolve_forced(inputs: &RouteInputs, format: GraphicsFormat, transport: Choice) -> GraphicsRoute {
    let requested = format!("BISCUIT_GRAPHICS={}", format_name(format).to_ascii_lowercase());
    if tmux_in_path(inputs, transport) {
        return tmux_planned(&requested);
    }
    if windows_ssh_server(inputs) && format != GraphicsFormat::Iterm2 {
        return GraphicsRoute::unavailable(
            RouteReasonKind::BlockedTransport,
            RouteConfidence::Forced,
            format!(
                "{requested}: the Windows SSH server's console host removes {} sequences",
                format_name(format)
            ),
        );
    }
    if inputs.var_is_set("ZELLIJ") && format == GraphicsFormat::Iterm2 {
        return GraphicsRoute::unavailable(
            RouteReasonKind::BlockedTransport,
            RouteConfidence::Forced,
            format!("{requested}: Zellij drops the iTerm2 image format"),
        );
    }
    GraphicsRoute::pixels(format, RouteConfidence::Forced, format!("forced by {requested}"))
}

/// Automatic detection from the environment.
fn resolve_detected(inputs: &RouteInputs, transport: Choice) -> GraphicsRoute {
    if windows_ssh_server(inputs) {
        return resolve_windows_ssh(inputs);
    }
    if inputs.var_is_set("ZELLIJ") {
        return resolve_zellij(inputs);
    }
    if tmux_in_path(inputs, transport) {
        return tmux_planned("tmux");
    }

    if let Some((terminal, source)) = identify_terminal(inputs) {
        return known_terminal_route(terminal, &source);
    }
    if !inputs.in_ssh_session() {
        if inputs.var_is_set("WT_SESSION") {
            return resolve_windows_terminal(inputs);
        }
        if inputs.var_is_set("MSYSTEM") && !inputs.var_is_set("TERM_PROGRAM") {
            return known_terminal_route(KnownTerminal::Iterm2Family("mintty"), "MSYSTEM");
        }
        if let Some((terminal, source)) = identify_local_hints(inputs) {
            return known_terminal_route(terminal, &source);
        }
    }

    let message = if inputs.in_ssh_session() {
        "SSH session without forwarded terminal hints; the graphics probe is planned"
    } else {
        "terminal not identified from the environment; the graphics probe is planned"
    };
    GraphicsRoute::unavailable(RouteReasonKind::Unknown, RouteConfidence::Unknown, message)
}

/// The measured Windows OpenSSH server path: its system console host removes
/// APC (Kitty) and DCS (Sixel) and passes OSC, so only the iTerm2 format
/// survives, and only an outer terminal on the iTerm2 list renders it.
fn resolve_windows_ssh(inputs: &RouteInputs) -> GraphicsRoute {
    // As in any SSH session, the client's `LC_TERMINAL` beats `TERM_PROGRAM`.
    let from_lc = inputs
        .var("LC_TERMINAL")
        .and_then(|v| windows_ssh_iterm2_outer(v).map(|name| (name, format!("LC_TERMINAL={v}"))));
    let outer = from_lc.or_else(|| {
        inputs
            .var("TERM_PROGRAM")
            .and_then(|p| windows_ssh_iterm2_outer(p).map(|name| (name, format!("TERM_PROGRAM={p}"))))
    });
    match outer {
        Some((name, source)) => GraphicsRoute::pixels(
            GraphicsFormat::Iterm2,
            RouteConfidence::Known,
            format!(
                "Windows SSH server: the console host passes only the iTerm2 format, which {name} ({source}) renders"
            ),
        ),
        None => GraphicsRoute::unavailable(
            RouteReasonKind::BlockedTransport,
            RouteConfidence::Known,
            "Windows SSH server: the console host removes Kitty and Sixel, and the outer terminal is not one that renders the iTerm2 format",
        ),
    }
}

/// The iTerm2 list: terminals that render the iTerm2 format.
fn windows_ssh_iterm2_outer(value: &str) -> Option<&'static str> {
    const ITERM2_LIST: &[(&str, &str)] = &[
        ("iTerm.app", "iTerm2"),
        ("iTerm2", "iTerm2"),
        ("WezTerm", "WezTerm"),
        ("konsole", "Konsole"),
        ("mintty", "mintty"),
        ("cterm", "cterm"),
    ];
    ITERM2_LIST
        .iter()
        .find(|(key, _)| value.eq_ignore_ascii_case(key))
        .map(|&(_, name)| name)
}

/// Zellij renders Kitty graphics from 0.45, when its host supports them.
/// Until the pane probe ships, a Kitty-family identity inherited from the
/// host stands in for the pane's answer.
fn resolve_zellij(inputs: &RouteInputs) -> GraphicsRoute {
    let Some(version) = inputs.zellij_version else {
        return GraphicsRoute::unavailable(
            RouteReasonKind::Unknown,
            RouteConfidence::Unknown,
            "Zellij version unreadable (`zellij --version` failed or timed out)",
        );
    };
    if version < ZELLIJ_KITTY_MIN {
        return GraphicsRoute::unavailable(
            RouteReasonKind::NoSupport,
            RouteConfidence::Known,
            format!("Zellij {version} predates Kitty graphics (0.45); Sixel detection needs the pane probe (planned)"),
        )
        .with_fix(format!("upgrade Zellij to {ZELLIJ_KITTY_MIN} or later"));
    }
    match inputs.zellij_kitty_enabled {
        Some(true) => GraphicsRoute::pixels(
            GraphicsFormat::Kitty,
            RouteConfidence::Verified,
            format!("Zellij {version} answered a Kitty graphics query"),
        ),
        Some(false) => GraphicsRoute::unavailable(
            RouteReasonKind::NoSupport,
            RouteConfidence::Verified,
            format!("Zellij {version} has Kitty graphics disabled"),
        ),
        None => match identify_terminal(inputs) {
            Some((KnownTerminal::KittyFamily(name), source)) => GraphicsRoute::pixels(
                GraphicsFormat::Kitty,
                RouteConfidence::Known,
                format!("Zellij {version} inside {name} ({source}) renders Kitty graphics"),
            ),
            _ => GraphicsRoute::unavailable(
                RouteReasonKind::Unknown,
                RouteConfidence::Unknown,
                format!(
                    "Zellij {version} inside a terminal not known to render Kitty graphics; the pane probe is planned"
                ),
            ),
        },
    }
}

/// Windows Terminal renders Sixel from 1.22. The version comes from the
/// `WindowsTerminal.exe` hosting this process; the DA1 signal arrives with
/// the probe (planned).
fn resolve_windows_terminal(inputs: &RouteInputs) -> GraphicsRoute {
    match inputs.windows_terminal_version {
        Some(version) if version >= WINDOWS_TERMINAL_SIXEL_MIN => GraphicsRoute::pixels(
            GraphicsFormat::Sixel,
            RouteConfidence::Known,
            format!("Windows Terminal {version} renders Sixel"),
        ),
        Some(version) => GraphicsRoute::unavailable(
            RouteReasonKind::NoSupport,
            RouteConfidence::Known,
            format!("Windows Terminal {version} predates Sixel ({WINDOWS_TERMINAL_SIXEL_MIN})"),
        )
        .with_fix(format!(
            "upgrade Windows Terminal to {WINDOWS_TERMINAL_SIXEL_MIN} or later"
        )),
        None => GraphicsRoute::unavailable(
            RouteReasonKind::Unknown,
            RouteConfidence::Unknown,
            "Windows Terminal (WT_SESSION) whose version could not be read",
        )
        .with_fix("set BISCUIT_GRAPHICS=sixel if this Windows Terminal is 1.22 or later"),
    }
}

/// Resolves the route for this process: stdout and the process environment.
///
/// [`Terminal::new`](crate::terminal::Terminal::new) calls this once and
/// keeps the result; prefer reading
/// [`Terminal::graphics_route`](crate::terminal::Terminal::graphics_route).
#[must_use]
pub fn graphics_route() -> GraphicsRoute {
    let route = resolve_graphics_route(&RouteInputs::from_process());
    tracing::debug!(
        format = ?route.format,
        confidence = ?route.confidence,
        reason = %route.reason,
        "Resolved graphics route"
    );
    route
}

#[cfg(test)]
mod tests;
