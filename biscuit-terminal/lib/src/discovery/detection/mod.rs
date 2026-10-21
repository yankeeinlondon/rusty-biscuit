//! Terminal capability detection submodules.
//!
//! This module groups static-detection helpers by capability:
//!
//! - [`app`] — Terminal application identification
//! - [`color`] — Color depth and light/dark mode detection
//! - [`dimensions`] — Terminal width/height and TTY checks
//! - [`image`] — Image support, a projection of the graphics route
//! - [`route`] — The graphics route (format, transport, placement, reason)
//! - [`osc8`] — OSC8 hyperlink support
//! - [`styling_caps`] — Italics, dim, and underline support
//! - [`tabs`] — Horizontal-tab interval detection
//! - [`multiplex`] — Multiplexer detection (tmux, Zellij, native)
//! - [`tmux`] — tmux version, passthrough, and attached-client facts
//! - [`connection`] — Local vs SSH vs Mosh connection detection

pub mod app;
pub mod color;
pub mod connection;
pub mod dimensions;
pub mod image;
pub mod multiplex;
pub mod osc8;
pub mod probe;
pub mod route;
pub mod styling_caps;
pub mod tabs;
pub mod tmux;
mod versions;

pub use app::{TerminalApp, get_terminal_app};
pub use color::{ColorDepth, ColorMode, color_depth, color_mode};
pub use connection::{Connection, MoshClient, SshClient, detect_connection};
pub use dimensions::{dimensions, is_tty, set_tty_override, terminal_height, terminal_width};
pub use image::{
    DetectionMethod, ImageSupport, ImageSupportResult, image_support, image_support_with_reason,
};
pub use multiplex::{MultiplexSupport, multiplex_support};
pub use osc8::osc8_link_support;
pub use probe::{
    Da1Reply, GraphicsProbe, KittyReply, ProbePlan, ProbeReply, ProbeResult, ProbeSkip,
};
pub use route::{
    GRAPHICS_ENV, GRAPHICS_TRANSPORT_ENV, GraphicsFormat, GraphicsPlacement, GraphicsRoute,
    GraphicsTransport, PROBE_ENV, ProbeMode, RouteConfidence, RouteInputs, RouteReason,
    RouteReasonKind, ToolVersion, graphics_route, probe_plan, resolve_graphics_route,
};
pub use styling_caps::{UnderlineSupport, dim_support, italics_support, underline_support};
pub use tabs::{DEFAULT_TAB_WIDTH, tab_width};
pub use tmux::{Passthrough, TmuxClient, TmuxFacts};

// Re-export OSC support functions from osc_queries module for API compatibility
pub use crate::discovery::osc_queries::{osc10_support, osc11_support, osc12_support};
