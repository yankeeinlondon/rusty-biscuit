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
use super::probe::{Da1Reply, GraphicsProbe, KittyReply, ProbePlan, ProbeReply, ProbeResult, ProbeSkip};
use super::tmux::{Passthrough, TmuxClient, TmuxFacts};

/// Environment variable that forces or disables a graphics format.
pub const GRAPHICS_ENV: &str = "BISCUIT_GRAPHICS";
/// Environment variable that declares the transport between this program and
/// the terminal.
pub const GRAPHICS_TRANSPORT_ENV: &str = "BISCUIT_GRAPHICS_TRANSPORT";
/// Environment variable that turns construction-time terminal queries off.
pub const PROBE_ENV: &str = "BISCUIT_TERMINAL_PROBE";

const GRAPHICS_VALUES: &[&str] = &["auto", "kitty", "iterm2", "sixel", "none"];
const TRANSPORT_VALUES: &[&str] = &["auto", "direct", "tmux"];
const PROBE_VALUES: &[&str] = &["auto", "off"];

/// The first Zellij release that renders Kitty graphics.
const ZELLIJ_KITTY_MIN: ToolVersion = ToolVersion::new(0, 45, 0);
/// The first Windows Terminal release that renders Sixel.
const WINDOWS_TERMINAL_SIXEL_MIN: ToolVersion = ToolVersion::new(1, 22, 0);
/// The first tmux release with `allow-passthrough`.
const TMUX_PASSTHROUGH_MIN: ToolVersion = ToolVersion::new(3, 3, 0);

/// The first tmux release that draws Sixel images itself.
const TMUX_NATIVE_SIXEL_MIN: ToolVersion = ToolVersion::new(3, 4, 0);

/// Every environment variable resolution reads.
const ROUTE_ENV_VARS: &[&str] = &[
    GRAPHICS_ENV,
    GRAPHICS_TRANSPORT_ENV,
    PROBE_ENV,
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
    "TMUX_PANE",
    "STY",
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
    /// The image must be sized from a measured cell size, never the
    /// estimated fallback. Set for Zellij, which measures its host itself: a
    /// query success there says nothing about whether it knows the cell size.
    pub needs_measured_cells: bool,
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
            needs_measured_cells: false,
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
            needs_measured_cells: false,
        }
    }

    fn through_tmux(mut self) -> Self {
        self.transport = GraphicsTransport::Tmux;
        self
    }

    fn needing_measured_cells(mut self) -> Self {
        self.needs_measured_cells = true;
        self
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
    /// The file version of the `WindowsTerminal.exe` hosting this process,
    /// found by walking the parent process chain on native Windows; `None`
    /// when unreadable.
    pub windows_terminal_version: Option<ToolVersion>,
    /// Whether this process runs in CI, where nothing is probed.
    pub is_ci: bool,
    /// Whether the caller allows construction-time probing.
    pub probe_mode: ProbeMode,
    /// The graphics probe's results; empty until a probe runs.
    pub probe: GraphicsProbe,
    /// What the visible tmux reports; read only when `TMUX` is set.
    pub tmux: TmuxFacts,
    vars: BTreeMap<String, OsString>,
}

/// Whether construction-time terminal queries may run.
///
/// An application with its own input loop (a TUI) turns probing off, or
/// supplies a resolved route, so no query races its reader. Only the first
/// [`Terminal`](crate::terminal::Terminal) construction in a process probes;
/// make it before the input loop starts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProbeMode {
    /// Probe when the environment cannot decide.
    #[default]
    Auto,
    /// Never query the terminal while constructing; colors and graphics use
    /// only the environment.
    Off,
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
    /// variables resolution consults, the Zellij and Windows Terminal
    /// versions, and the tmux facts where they matter.
    #[must_use]
    pub fn from_process() -> Self {
        let mut inputs = Self::new(super::dimensions::is_tty());
        inputs.is_windows = cfg!(windows);
        inputs.is_ci = crate::discovery::os_detection::is_ci();
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
        if inputs.var_is_set("TMUX") {
            let facts = super::tmux::read_facts(inputs.var("TMUX_PANE"));
            inputs.tmux = facts;
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
/// `BISCUIT_GRAPHICS_TRANSPORT` / `BISCUIT_TERMINAL_PROBE`, then the Windows
/// SSH server rule, then the multiplexer (Zellij, tmux, a likely tmux hidden
/// behind SSH), then the terminal's identity, then the graphics probe's
/// results in [`RouteInputs::probe`]. What none of them decides resolves to
/// [`RouteConfidence::Unknown`].
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
    decide(inputs).0
}

/// Which graphics probe exchanges would let [`resolve_graphics_route`] decide
/// what the environment cannot.
///
/// Empty for nonterminal output, CI, invalid or disabling overrides, a
/// forced format, probing turned off, the Windows SSH server, a legacy
/// Windows console, and every terminal whose identity already decides the
/// route. A visible tmux is asked only what it can answer: its own DA1
/// (`direct_da1_only`) when it could draw Sixel itself, and the wrapped
/// query when its `allow-passthrough` could not be read.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{RouteInputs, probe_plan};
///
/// let known = RouteInputs::new(true).with_var("TERM_PROGRAM", "WezTerm");
/// assert!(!probe_plan(&known).any());
///
/// let unknown_ssh = RouteInputs::new(true).with_var("SSH_CONNECTION", "a 1 b 22");
/// assert!(probe_plan(&unknown_ssh).direct);
/// ```
#[must_use]
pub fn probe_plan(inputs: &RouteInputs) -> ProbePlan {
    decide(inputs).1
}

/// The route plus the probe exchanges that would refine it, gated by
/// whether probing is allowed at all.
fn decide(inputs: &RouteInputs) -> (GraphicsRoute, ProbePlan) {
    if !inputs.output_is_tty {
        let route = GraphicsRoute::unavailable(
            RouteReasonKind::NotATerminal,
            RouteConfidence::Known,
            "output is not a terminal (piped or redirected)",
        );
        return (route, ProbePlan::NONE);
    }

    let choices = (
        parse_choice(GRAPHICS_ENV, inputs.var_os(GRAPHICS_ENV), GRAPHICS_VALUES),
        parse_choice(
            GRAPHICS_TRANSPORT_ENV,
            inputs.var_os(GRAPHICS_TRANSPORT_ENV),
            TRANSPORT_VALUES,
        ),
        parse_choice(PROBE_ENV, inputs.var_os(PROBE_ENV), PROBE_VALUES),
    );
    let (format_choice, transport_choice, probe_choice) = match choices {
        (Ok(format), Ok(transport), Ok(probe)) => (format, transport, probe),
        (Err(route), _, _) | (_, Err(route), _) | (_, _, Err(route)) => {
            return (route, ProbePlan::NONE);
        }
    };

    let probing = if inputs.is_ci {
        Probing::Blocked("not probed in CI")
    } else if probe_choice == Choice::Value("off") {
        Probing::Blocked("not probed: BISCUIT_TERMINAL_PROBE=off")
    } else if inputs.probe_mode == ProbeMode::Off {
        Probing::Blocked("not probed: the caller turned probing off")
    } else {
        Probing::Allowed
    };

    let (route, plan) = match format_choice {
        Choice::Value("none") => (
            GraphicsRoute::unavailable(
                RouteReasonKind::UserDisabled,
                RouteConfidence::Forced,
                "graphics disabled by BISCUIT_GRAPHICS=none",
            ),
            ProbePlan::NONE,
        ),
        Choice::Value(value) => {
            let format = format_from_choice(value)
                .expect("the accepted values other than auto and none name formats");
            (resolve_forced(inputs, format, transport_choice), ProbePlan::NONE)
        }
        Choice::Auto => resolve_detected(inputs, transport_choice, probing),
    };
    match probing {
        Probing::Allowed => (route, plan),
        Probing::Blocked(_) => (route, ProbePlan::NONE),
    }
}

/// Whether a probe may be sent, and if not, the phrase saying why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Probing {
    Allowed,
    Blocked(&'static str),
}

/// Whether the path to the terminal goes through a tmux this process can
/// see (`TMUX` is set) or was told about.
fn tmux_in_path(inputs: &RouteInputs, transport: Choice) -> bool {
    match transport {
        Choice::Value("tmux") => true,
        Choice::Value(_) => false,
        Choice::Auto => inputs.var_is_set("TMUX"),
    }
}

/// `TERM=tmux-*` or `screen-*` with neither `TMUX` nor `STY`: a tmux on the
/// far side of SSH that this process cannot see.
fn hidden_tmux_hint(inputs: &RouteInputs) -> Option<String> {
    if inputs.var_is_set("TMUX") || inputs.var_is_set("STY") {
        return None;
    }
    let term = inputs.var("TERM")?;
    let hinted = ["tmux", "screen"]
        .iter()
        .any(|name| term == *name || term.starts_with(&format!("{name}-")));
    hinted.then(|| format!("TERM={term}"))
}

fn windows_ssh_server(inputs: &RouteInputs) -> bool {
    inputs.is_windows && inputs.var_is_set("SSH_CONNECTION")
}

/// A native Windows console with no modern terminal attached: no
/// `WT_SESSION`, no `TERM_PROGRAM`, no MSYS shell (mintty), and not an SSH
/// session. It answers no queries and renders no pixel format.
fn legacy_windows_console(inputs: &RouteInputs) -> bool {
    inputs.is_windows
        && !inputs.in_ssh_session()
        && !inputs.var_is_set("WT_SESSION")
        && !inputs.var_is_set("TERM_PROGRAM")
        && !inputs.var_is_set("MSYSTEM")
}

/// A `BISCUIT_GRAPHICS` format: no probing and no identity preferences, but
/// transports known to remove the format still rule it out.
fn resolve_forced(inputs: &RouteInputs, format: GraphicsFormat, transport: Choice) -> GraphicsRoute {
    let requested = format!("BISCUIT_GRAPHICS={}", format_name(format).to_ascii_lowercase());
    if inputs.var_is_set("TMUX") && tmux_in_path(inputs, transport) {
        return resolve_visible_tmux(inputs, Some((format, &requested)), Probing::Blocked("not probed: a forced format")).0;
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
    let route = GraphicsRoute::pixels(format, RouteConfidence::Forced, format!("forced by {requested}"));
    if tmux_in_path(inputs, transport) {
        // `BISCUIT_GRAPHICS_TRANSPORT=tmux` with no `TMUX`: a tmux beyond SSH.
        return route.through_tmux();
    }
    route
}

/// Automatic detection from the environment and, where it cannot decide,
/// the probe.
fn resolve_detected(inputs: &RouteInputs, transport: Choice, probing: Probing) -> (GraphicsRoute, ProbePlan) {
    if windows_ssh_server(inputs) {
        return (resolve_windows_ssh(inputs), ProbePlan::NONE);
    }
    if inputs.var_is_set("ZELLIJ") {
        return (resolve_zellij(inputs, probing), ProbePlan::DIRECT);
    }
    if tmux_in_path(inputs, transport) {
        if inputs.var_is_set("TMUX") {
            return resolve_visible_tmux(inputs, None, probing);
        }
        return resolve_declared_tmux(inputs, probing);
    }
    if transport == Choice::Auto
        && let Some(hint) = hidden_tmux_hint(inputs)
    {
        let plan = ProbePlan {
            direct: true,
            tunneled: true,
            direct_da1_only: false,
        };
        return (resolve_hidden_tmux(inputs, &hint, probing), plan);
    }

    if let Some((terminal, source)) = identify_terminal(inputs) {
        return (known_terminal_route(terminal, &source), ProbePlan::NONE);
    }
    if !inputs.in_ssh_session() {
        if inputs.var_is_set("WT_SESSION") {
            return resolve_windows_terminal(inputs, probing);
        }
        if inputs.var_is_set("MSYSTEM") && !inputs.var_is_set("TERM_PROGRAM") {
            let route = known_terminal_route(KnownTerminal::Iterm2Family("mintty"), "MSYSTEM");
            return (route, ProbePlan::NONE);
        }
        if let Some((terminal, source)) = identify_local_hints(inputs) {
            return (known_terminal_route(terminal, &source), ProbePlan::NONE);
        }
    }
    if legacy_windows_console(inputs) {
        let route = GraphicsRoute::unavailable(
            RouteReasonKind::NoSupport,
            RouteConfidence::Known,
            "Windows console without a modern terminal (no WT_SESSION or TERM_PROGRAM) has no pixel format",
        );
        return (route, ProbePlan::NONE);
    }

    let subject = if inputs.in_ssh_session() {
        "SSH session without forwarded terminal hints"
    } else {
        "terminal not identified from the environment"
    };
    (resolve_probed(subject, inputs.probe.direct.as_ref(), probing), ProbePlan::DIRECT)
}

/// What the direct probe says about an endpoint the environment could not
/// identify. A correlated Kitty `OK` beats Sixel.
fn resolve_probed(subject: &str, result: Option<&ProbeResult>, probing: Probing) -> GraphicsRoute {
    let reply = match probe_reply(subject, result, probing) {
        Ok(reply) => reply,
        Err(route) => return route,
    };
    if reply.kitty == KittyReply::Accepted {
        return GraphicsRoute::pixels(
            GraphicsFormat::Kitty,
            RouteConfidence::Verified,
            format!("{subject}; the terminal answered the Kitty graphics query"),
        );
    }
    if reply.da1.as_ref().is_some_and(Da1Reply::has_sixel) {
        return GraphicsRoute::pixels(
            GraphicsFormat::Sixel,
            RouteConfidence::Verified,
            format!("{subject}; the terminal reports Sixel (DA1 attribute 4)"),
        );
    }
    no_pixels_from_reply(subject, reply)
}

/// The reply to read, or the route for a probe that produced none.
fn probe_reply<'a>(
    subject: &str,
    result: Option<&'a ProbeResult>,
    probing: Probing,
) -> Result<&'a ProbeReply, GraphicsRoute> {
    let unknown = |detail: String| {
        Err(GraphicsRoute::unavailable(
            RouteReasonKind::Unknown,
            RouteConfidence::Unknown,
            format!("{subject}; {detail}"),
        ))
    };
    match (result, probing) {
        (_, Probing::Blocked(why)) => unknown(why.to_string()),
        (None, Probing::Allowed) => unknown("the graphics probe was not run".to_string()),
        (Some(ProbeResult::Skipped(ProbeSkip::BusyInput)), _) => {
            unknown("the graphics probe was skipped because input was waiting".to_string())
        }
        (Some(ProbeResult::Skipped(ProbeSkip::Unavailable(error))), _) => {
            unknown(format!("the graphics probe could not run ({error})"))
        }
        (Some(ProbeResult::Replied(reply)), _) => Ok(reply),
    }
}

/// The route for a reply that offered neither a Kitty `OK` nor Sixel.
fn no_pixels_from_reply(subject: &str, reply: &ProbeReply) -> GraphicsRoute {
    match (&reply.kitty, &reply.da1) {
        (KittyReply::Rejected(error), _) => GraphicsRoute::unavailable(
            RouteReasonKind::NoSupport,
            RouteConfidence::Verified,
            format!("{subject}; the terminal rejected the Kitty graphics query ({error:?}) and reports no Sixel"),
        ),
        (KittyReply::NoReply, Some(_)) => GraphicsRoute::unavailable(
            RouteReasonKind::NoSupport,
            RouteConfidence::Verified,
            format!("{subject}; the terminal answered without Kitty graphics or Sixel"),
        ),
        (KittyReply::Conflicting, _) => GraphicsRoute::unavailable(
            RouteReasonKind::Unknown,
            RouteConfidence::Unknown,
            format!("{subject}; the Kitty graphics query got conflicting replies"),
        ),
        (KittyReply::Accepted, _) => GraphicsRoute::unavailable(
            RouteReasonKind::Unknown,
            RouteConfidence::Unknown,
            format!("{subject}; the terminal answered the Kitty graphics query, which this route cannot use"),
        ),
        (KittyReply::NoReply, None) => GraphicsRoute::unavailable(
            RouteReasonKind::Unknown,
            RouteConfidence::Unknown,
            format!("{subject}; the graphics probe got no reply before its deadline"),
        ),
    }
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

/// Zellij renders Kitty graphics from 0.45, when its host supports them;
/// only a query to the pane shows whether it is enabled. A pane that does
/// not answer Kitty may still answer Sixel.
fn resolve_zellij(inputs: &RouteInputs, probing: Probing) -> GraphicsRoute {
    let version = inputs.zellij_version;
    let subject = match version {
        Some(version) => format!("Zellij {version}"),
        None => "Zellij (version unreadable)".to_string(),
    };
    let too_old = version.is_some_and(|v| v < ZELLIJ_KITTY_MIN);
    let reply = probe_reply(&subject, inputs.probe.direct.as_ref(), probing);

    if let Ok(reply) = reply {
        if reply.kitty == KittyReply::Accepted && !too_old {
            return GraphicsRoute::pixels(
                GraphicsFormat::Kitty,
                RouteConfidence::Verified,
                format!("{subject} answered a Kitty graphics query from the pane"),
            )
            .needing_measured_cells();
        }
        if reply.da1.as_ref().is_some_and(Da1Reply::has_sixel) {
            return GraphicsRoute::pixels(
                GraphicsFormat::Sixel,
                RouteConfidence::Verified,
                format!("{subject} reports Sixel (DA1 attribute 4)"),
            )
            .needing_measured_cells();
        }
    }
    if too_old {
        return GraphicsRoute::unavailable(
            RouteReasonKind::NoSupport,
            RouteConfidence::Known,
            format!("{subject} predates Kitty graphics (0.45) and reported no Sixel"),
        )
        .with_fix(format!("upgrade Zellij to {ZELLIJ_KITTY_MIN} or later"));
    }
    match reply {
        Err(not_answered) => not_answered,
        Ok(reply) if reply.da1.is_some() && reply.kitty != KittyReply::Conflicting => {
            GraphicsRoute::unavailable(
                RouteReasonKind::NoSupport,
                RouteConfidence::Verified,
                format!("{subject} has Kitty graphics disabled for this pane and reports no Sixel"),
            )
        }
        Ok(reply) => no_pixels_from_reply(&subject, reply),
    }
}

/// Windows Terminal renders Sixel from 1.22. Either positive signal
/// suffices: the `WindowsTerminal.exe` hosting this process is 1.22 or later,
/// or the probe's DA1 reply reports Sixel.
fn resolve_windows_terminal(inputs: &RouteInputs, probing: Probing) -> (GraphicsRoute, ProbePlan) {
    if let Some(version) = inputs.windows_terminal_version
        && version >= WINDOWS_TERMINAL_SIXEL_MIN
    {
        let route = GraphicsRoute::pixels(
            GraphicsFormat::Sixel,
            RouteConfidence::Known,
            format!("Windows Terminal {version} renders Sixel"),
        );
        return (route, ProbePlan::NONE);
    }
    if let Some(ProbeResult::Replied(reply)) = &inputs.probe.direct
        && probing == Probing::Allowed
        && reply.da1.as_ref().is_some_and(Da1Reply::has_sixel)
    {
        let route = GraphicsRoute::pixels(
            GraphicsFormat::Sixel,
            RouteConfidence::Verified,
            "Windows Terminal reports Sixel (DA1 attribute 4)",
        );
        return (route, ProbePlan::DIRECT);
    }
    let route = match inputs.windows_terminal_version {
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
            "Windows Terminal (WT_SESSION) whose version could not be read and whose DA1 reply reports no Sixel",
        )
        .with_fix("set BISCUIT_GRAPHICS=sixel if this Windows Terminal is 1.22 or later"),
    };
    (route, ProbePlan::DIRECT)
}

/// A tmux this process can see (`TMUX` is set). Sixel that tmux draws
/// itself comes first, because the image then stays with the pane; it is
/// established only when tmux is 3.4 or later, every client of the pane's
/// session has the `sixel` terminal feature, and tmux's own (unwrapped) DA1
/// reply reports attribute `4`. Otherwise the route goes through passthrough
/// (see [`resolve_tmux_passthrough`]). A forced Sixel goes to tmux itself
/// when the version and clients allow it, without asking for its DA1.
fn resolve_visible_tmux(
    inputs: &RouteInputs,
    forced: Option<(GraphicsFormat, &str)>,
    probing: Probing,
) -> (GraphicsRoute, ProbePlan) {
    let native_possible = inputs.tmux.version.filter(|v| *v >= TMUX_NATIVE_SIXEL_MIN).is_some()
        && inputs
            .tmux
            .clients
            .as_deref()
            .is_some_and(|clients| !clients.is_empty() && clients.iter().all(|c| c.has_feature("sixel")));
    if !native_possible {
        return resolve_tmux_passthrough(inputs, forced, probing);
    }
    let subject = format!("tmux {}", inputs.tmux.version.expect("checked above"));
    if let Some((format, requested)) = forced {
        if format == GraphicsFormat::Sixel {
            let route = GraphicsRoute::pixels(
                GraphicsFormat::Sixel,
                RouteConfidence::Forced,
                format!("forced by {requested}, drawn by {subject} itself (every client has the sixel terminal feature)"),
            );
            return (route, ProbePlan::NONE);
        }
        return resolve_tmux_passthrough(inputs, forced, probing);
    }
    if probing == Probing::Allowed
        && let Some(ProbeResult::Replied(reply)) = &inputs.probe.direct
        && reply.da1.as_ref().is_some_and(Da1Reply::has_sixel)
    {
        let route = GraphicsRoute::pixels(
            GraphicsFormat::Sixel,
            RouteConfidence::Verified,
            format!(
                "{subject} draws Sixel itself: its DA1 reply reports attribute 4 and every client has the sixel terminal feature"
            ),
        );
        return (route, NATIVE_SIXEL_PLAN);
    }
    let (route, plan) = resolve_tmux_passthrough(inputs, None, probing);
    let plan = ProbePlan {
        tunneled: plan.tunneled,
        ..NATIVE_SIXEL_PLAN
    };
    (route, plan)
}

/// The direct exchange that asks a visible tmux about its own Sixel: DA1
/// alone, which tmux answers without the outer terminal.
const NATIVE_SIXEL_PLAN: ProbePlan = ProbePlan {
    direct: true,
    tunneled: false,
    direct_da1_only: true,
};

/// The route through a visible tmux's passthrough. Passthrough needs tmux
/// 3.3 or later with `allow-passthrough` on for the pane; the format comes
/// from the terminals of the clients attached to the pane's session. Only
/// when `allow-passthrough` could not be read does the wrapped probe run:
/// its reply proves passthrough works. A `forced` format keeps the version
/// and passthrough checks and skips identifying the format; the clients
/// still decide whether Kitty is placed with Unicode placeholders.
fn resolve_tmux_passthrough(
    inputs: &RouteInputs,
    forced: Option<(GraphicsFormat, &str)>,
    probing: Probing,
) -> (GraphicsRoute, ProbePlan) {
    let facts = &inputs.tmux;
    let lead = forced.map_or(String::new(), |(_, requested)| format!("{requested}: "));
    let version = match facts.version {
        None => {
            let route = GraphicsRoute::unavailable(
                RouteReasonKind::Unknown,
                RouteConfidence::Unknown,
                format!(
                    "{lead}the tmux version could not be read (tmux -V); graphics through passthrough need tmux {TMUX_PASSTHROUGH_MIN} or later"
                ),
            );
            return (route, ProbePlan::NONE);
        }
        Some(version) if version < TMUX_PASSTHROUGH_MIN => {
            let route = GraphicsRoute::unavailable(
                RouteReasonKind::BlockedTransport,
                RouteConfidence::Known,
                format!("{lead}tmux {version} predates allow-passthrough ({TMUX_PASSTHROUGH_MIN})"),
            )
            .with_fix(format!("upgrade tmux to {TMUX_PASSTHROUGH_MIN} or later"));
            return (route, ProbePlan::NONE);
        }
        Some(version) => version,
    };
    let subject = format!("tmux {version}");
    if facts.passthrough == Passthrough::Off {
        let route = GraphicsRoute::unavailable(
            RouteReasonKind::BlockedTransport,
            RouteConfidence::Known,
            format!("{lead}{subject} has allow-passthrough off for this pane"),
        )
        .with_fix("set -g allow-passthrough on");
        return (route, ProbePlan::NONE);
    }
    let outer = outer_terminal(facts.clients.as_deref());
    if let Some((format, requested)) = forced {
        let route = GraphicsRoute::pixels(
            format,
            RouteConfidence::Forced,
            format!("forced by {requested}, through {subject} passthrough"),
        )
        .through_tmux();
        return (with_placement(route, &outer), ProbePlan::NONE);
    }
    if facts.passthrough == Passthrough::On {
        return (passthrough_route(&subject, &outer, None), ProbePlan::NONE);
    }

    let plan = ProbePlan {
        direct: false,
        tunneled: true,
        direct_da1_only: false,
    };
    let unread = format!("{subject}: allow-passthrough could not be read");
    let route = match probe_reply(&unread, inputs.probe.tunneled.as_ref(), probing) {
        Ok(reply) if reply.kitty != KittyReply::Conflicting
            && (reply.kitty != KittyReply::NoReply || reply.da1.is_some()) =>
        {
            passthrough_route(&subject, &outer, Some(reply))
        }
        Ok(reply) => no_pixels_from_reply(&format!("{unread}, and the wrapped probe"), reply),
        Err(not_answered) => not_answered,
    };
    (route, plan)
}

/// The route through a tmux whose passthrough is on, or was shown to work
/// by a `reply` to the wrapped probe.
fn passthrough_route(subject: &str, outer: &OuterTerminal, reply: Option<&ProbeReply>) -> GraphicsRoute {
    let (how, confidence) = match reply {
        Some(_) => ("delivered the wrapped probe", RouteConfidence::Verified),
        None => ("has allow-passthrough on", RouteConfidence::Known),
    };
    match (outer, reply) {
        (OuterTerminal::Identified { format, names, .. }, _) => with_placement(
            GraphicsRoute::pixels(*format, confidence, format!("{subject} {how}, to {names}"))
                .through_tmux(),
            outer,
        ),
        (OuterTerminal::Disagree(names), _) => GraphicsRoute::unavailable(
            RouteReasonKind::NoSupport,
            RouteConfidence::Known,
            format!("{subject}: the attached clients' terminals ({names}) share no pixel format"),
        ),
        (OuterTerminal::Unidentified, None) => GraphicsRoute::pixels(
            GraphicsFormat::Kitty,
            RouteConfidence::Unknown,
            format!("{subject} {how}; the outer terminal is not identified, so Kitty is sent unverified"),
        )
        .through_tmux(),
        (OuterTerminal::Unidentified, Some(reply)) if reply.kitty == KittyReply::Accepted => {
            GraphicsRoute::pixels(
                GraphicsFormat::Kitty,
                RouteConfidence::Verified,
                format!("{subject} {how}, and the outer terminal answered the Kitty graphics query"),
            )
            .through_tmux()
        }
        (OuterTerminal::Unidentified, Some(reply)) => {
            no_pixels_from_reply(&format!("{subject} {how}"), reply)
        }
    }
}

/// `route` with Unicode placeholders when it sends Kitty to an `outer`
/// terminal whose every client can draw them. A Kitty or Ghostty client that
/// tmux does not send UTF-8 and 24-bit color keeps ordinary placement, and
/// the reason says why: the placeholder cells carry the image ID in their
/// foreground color.
fn with_placement(mut route: GraphicsRoute, outer: &OuterTerminal) -> GraphicsRoute {
    let OuterTerminal::Identified { placeholders, .. } = outer else {
        return route;
    };
    if route.format != GraphicsFormat::Kitty {
        return route;
    }
    match placeholders {
        PlaceholderCells::Usable => {
            route.placement = GraphicsPlacement::Placeholders;
            route.reason.message.push_str(", placed with Unicode placeholders");
        }
        PlaceholderCells::NeedColorAndUtf8 => route.reason.message.push_str(
            "; placed ordinarily, because Unicode placeholders need tmux to send UTF-8 and 24-bit color (the RGB terminal feature) to every client",
        ),
        PlaceholderCells::Unsupported => {}
    }
    route
}

/// The outer terminal behind a visible tmux, from its attached clients.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OuterTerminal {
    /// No client is attached, the list could not be read, or no client's
    /// terminal is recognized.
    Unidentified,
    /// Every client renders `format`; `names` lists the terminals.
    Identified {
        format: GraphicsFormat,
        names: String,
        placeholders: PlaceholderCells,
    },
    /// The clients' terminals have no format in common.
    Disagree(String),
}

/// Whether Kitty images can be placed with Unicode placeholder cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaceholderCells {
    /// Every client is Kitty or Ghostty, and tmux sends each UTF-8 and
    /// 24-bit color.
    Usable,
    /// Every client is Kitty or Ghostty, but tmux does not send at least one
    /// of them UTF-8 or 24-bit color.
    NeedColorAndUtf8,
    /// A client's terminal does not draw placeholders (WezTerm ignores
    /// `U=1`) or is not identified.
    Unsupported,
}

/// The pixel formats a client's terminal renders through passthrough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClientFormats {
    kitty: bool,
    iterm2: bool,
    /// Draws Kitty images through Unicode placeholder cells (`U=1`).
    placeholders: bool,
}

/// A client's terminal and its formats. `client_termtype` (the terminal's
/// own answer to tmux) is read first, then `client_termname`; a generic
/// `TERM` such as `xterm-256color` identifies nothing.
fn client_terminal(client: &TmuxClient) -> Option<(&'static str, ClientFormats)> {
    const KITTY_PLACEHOLDERS: ClientFormats = ClientFormats {
        kitty: true,
        iterm2: false,
        placeholders: true,
    };
    const BOTH: ClientFormats = ClientFormats {
        kitty: true,
        iterm2: true,
        placeholders: false,
    };
    const ITERM2: ClientFormats = ClientFormats {
        kitty: false,
        iterm2: true,
        placeholders: false,
    };
    let termtype = client.termtype.to_ascii_lowercase();
    let by_type = [
        ("kitty", "kitty", KITTY_PLACEHOLDERS),
        ("ghostty", "Ghostty", KITTY_PLACEHOLDERS),
        ("wezterm", "WezTerm", BOTH),
        ("konsole", "Konsole", BOTH),
        ("iterm2", "iTerm2", ITERM2),
    ];
    if let Some(&(_, name, formats)) = by_type.iter().find(|(prefix, ..)| termtype.starts_with(prefix)) {
        return Some((name, formats));
    }
    let termname = client.termname.to_ascii_lowercase();
    let by_name = [
        ("kitty", "kitty", KITTY_PLACEHOLDERS),
        ("ghostty", "Ghostty", KITTY_PLACEHOLDERS),
        ("wezterm", "WezTerm", BOTH),
    ];
    by_name
        .iter()
        .find(|(part, ..)| termname.contains(part))
        .map(|&(_, name, formats)| (name, formats))
}

/// The format every attached client can render, preferring Kitty. A client
/// whose terminal is not recognized counts as Kitty-only, the same default
/// a lone unidentified client gets.
fn outer_terminal(clients: Option<&[TmuxClient]>) -> OuterTerminal {
    let clients = clients.unwrap_or_default();
    let identified: Vec<_> = clients.iter().map(client_terminal).collect();
    if identified.iter().all(Option::is_none) {
        return OuterTerminal::Unidentified;
    }
    let mut names: Vec<&str> = Vec::new();
    let mut common = ClientFormats {
        kitty: true,
        iterm2: true,
        placeholders: true,
    };
    for terminal in &identified {
        let (name, formats) = terminal.unwrap_or((
            "an unidentified terminal",
            ClientFormats {
                kitty: true,
                iterm2: false,
                placeholders: false,
            },
        ));
        if !names.contains(&name) {
            names.push(name);
        }
        common.kitty &= formats.kitty;
        common.iterm2 &= formats.iterm2;
        common.placeholders &= formats.placeholders;
    }
    let names = names.join(", ");
    let placeholders = if !common.placeholders {
        PlaceholderCells::Unsupported
    } else if clients.iter().all(|c| c.utf8 && c.has_feature("RGB")) {
        PlaceholderCells::Usable
    } else {
        PlaceholderCells::NeedColorAndUtf8
    };
    if common.kitty {
        OuterTerminal::Identified {
            format: GraphicsFormat::Kitty,
            names,
            placeholders,
        }
    } else if common.iterm2 {
        OuterTerminal::Identified {
            format: GraphicsFormat::Iterm2,
            names,
            placeholders,
        }
    } else {
        OuterTerminal::Disagree(names)
    }
}

/// A likely tmux beyond SSH (see [`hidden_tmux_hint`]). Only a tmux with
/// passthrough on delivers the wrapped query to the outer terminal, so a
/// correlated `OK` to it confirms the tmux transport; an `OK` to the
/// unwrapped query means there was no tmux after all.
fn resolve_hidden_tmux(inputs: &RouteInputs, hint: &str, probing: Probing) -> GraphicsRoute {
    let accepted = |result: &Option<ProbeResult>| {
        matches!(result, Some(ProbeResult::Replied(reply)) if reply.kitty == KittyReply::Accepted)
    };
    if probing == Probing::Allowed {
        if accepted(&inputs.probe.tunneled) {
            return tunneled_route(inputs, RouteConfidence::Verified, &format!(
                "{hint} without TMUX: a tmux with passthrough on delivered the wrapped Kitty graphics query"
            ));
        }
        if accepted(&inputs.probe.direct) {
            let mut route = tunneled_route(
                inputs,
                RouteConfidence::Verified,
                &format!("{hint}, but the terminal itself answered the Kitty graphics query"),
            );
            route.transport = GraphicsTransport::Direct;
            return route;
        }
    }
    let detail = match probing {
        Probing::Blocked(why) => why.to_string(),
        Probing::Allowed => "neither the wrapped nor the plain Kitty graphics query was answered".to_string(),
    };
    GraphicsRoute::unavailable(
        RouteReasonKind::Unknown,
        RouteConfidence::Unknown,
        format!("{hint} without TMUX suggests a tmux beyond SSH; {detail}"),
    )
    .with_fix(format!(
        "set {GRAPHICS_TRANSPORT_ENV}=tmux if a tmux with allow-passthrough on runs between this program and the terminal"
    ))
}

/// `BISCUIT_GRAPHICS_TRANSPORT=tmux` with no `TMUX`: the user says a tmux
/// runs beyond SSH. The format comes from the forwarded outer-terminal
/// identity or the wrapped probe; the transport itself stays unverified.
fn resolve_declared_tmux(inputs: &RouteInputs, probing: Probing) -> (GraphicsRoute, ProbePlan) {
    let declared = format!("{GRAPHICS_TRANSPORT_ENV}=tmux");
    let plan = ProbePlan {
        direct: false,
        tunneled: true,
        direct_da1_only: false,
    };
    if let Some((terminal, source)) = identify_terminal(inputs) {
        let route = match terminal {
            KnownTerminal::AppleTerminal => known_terminal_route(terminal, &source),
            KnownTerminal::KittyFamily(name) | KnownTerminal::Iterm2Family(name) => {
                let format = match terminal {
                    KnownTerminal::KittyFamily(_) => GraphicsFormat::Kitty,
                    _ => GraphicsFormat::Iterm2,
                };
                GraphicsRoute::pixels(
                    format,
                    RouteConfidence::Forced,
                    format!("{declared} (unverified) to {name} ({source})"),
                )
                .through_tmux()
            }
        };
        return (route, ProbePlan::NONE);
    }
    if probing == Probing::Allowed
        && matches!(&inputs.probe.tunneled, Some(ProbeResult::Replied(reply)) if reply.kitty == KittyReply::Accepted)
    {
        let route = tunneled_route(
            inputs,
            RouteConfidence::Verified,
            &format!("{declared}: the terminal beyond tmux answered the wrapped Kitty graphics query"),
        );
        return (route, plan);
    }
    let route = GraphicsRoute::unavailable(
        RouteReasonKind::Unknown,
        RouteConfidence::Unknown,
        format!("{declared}: the terminal beyond tmux is not identified and did not answer the wrapped Kitty graphics query"),
    )
    .with_fix(format!("set {GRAPHICS_ENV} to the outer terminal's format (kitty, iterm2, or sixel)"));
    (route, plan)
}

/// A route confirmed by a Kitty `OK` under a tmux hint, wrapped for tmux
/// passthrough: Kitty with ordinary placement, or the iTerm2 format when the
/// client forwarded `LC_TERMINAL=iTerm2` (iTerm2 answers Kitty queries but is
/// never sent Kitty automatically).
fn tunneled_route(inputs: &RouteInputs, confidence: RouteConfidence, message: &str) -> GraphicsRoute {
    let format = match inputs.var("LC_TERMINAL").and_then(terminal_from_lc_terminal) {
        Some(KnownTerminal::Iterm2Family(_)) => GraphicsFormat::Iterm2,
        _ => GraphicsFormat::Kitty,
    };
    GraphicsRoute::pixels(format, confidence, message).through_tmux()
}

/// Resolves the route for this process: stdout, the process environment,
/// and, when they cannot decide, the graphics probe.
///
/// [`Terminal::new`](crate::terminal::Terminal::new) calls this once and
/// keeps the result; prefer reading
/// [`Terminal::graphics_route`](crate::terminal::Terminal::graphics_route).
#[must_use]
pub fn graphics_route() -> GraphicsRoute {
    graphics_route_with(ProbeMode::Auto)
}

/// [`graphics_route`] with the caller's probing choice.
pub(crate) fn graphics_route_with(probe_mode: ProbeMode) -> GraphicsRoute {
    let mut inputs = RouteInputs::from_process();
    inputs.probe_mode = probe_mode;
    let route = resolve_graphics_route(&inputs.probed());
    tracing::debug!(
        format = ?route.format,
        confidence = ?route.confidence,
        reason = %route.reason,
        "Resolved graphics route"
    );
    route
}

impl RouteInputs {
    /// Runs the probe exchanges [`probe_plan`] asks for and records their
    /// results. Exchanges are shared with the color query and cached per
    /// process, so only the first call writes to the terminal.
    #[must_use]
    pub fn probed(mut self) -> Self {
        let plan = probe_plan(&self);
        if plan.direct {
            self.probe.direct = Some(crate::discovery::startup_exchange::direct_probe(!plan.direct_da1_only));
        }
        if plan.tunneled {
            self.probe.tunneled = Some(crate::discovery::startup_exchange::tunneled_probe());
        }
        self
    }
}

#[cfg(test)]
mod tests;
