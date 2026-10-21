//! Environment matrix for `resolve_graphics_route`, through the public API.
//!
//! One test per environment row. Every row is pure: inputs are built with
//! `RouteInputs`, never by mutating the process environment.

use biscuit_terminal::discovery::detection::{
    Da1Reply, GraphicsFormat, GraphicsPlacement, GraphicsRoute, GraphicsTransport, ImageSupport,
    KittyReply, ProbeReply, ProbeResult, RouteConfidence, RouteInputs, RouteReasonKind,
    ToolVersion, resolve_graphics_route,
};

const SSH: &str = "10.0.0.1 50000 10.0.0.2 22";

fn tty() -> RouteInputs {
    RouteInputs::new(true)
}

fn ssh() -> RouteInputs {
    tty().with_var("SSH_CONNECTION", SSH)
}

/// A native Windows host reached over its OpenSSH server.
fn windows_ssh() -> RouteInputs {
    let mut inputs = ssh();
    inputs.is_windows = true;
    inputs
}

fn windows_terminal(version: Option<ToolVersion>) -> RouteInputs {
    let mut inputs = tty().with_var("WT_SESSION", "0f0e0d0c-0000-0000-0000-000000000000");
    inputs.windows_terminal_version = version;
    inputs
}

/// A Zellij pane. `kitty_enabled` is the pane's answer to the graphics
/// probe: `Some(true)` an `OK`, `Some(false)` a DA1 reply without one (and
/// without Sixel), `None` no probe result.
fn zellij(version: Option<ToolVersion>, kitty_enabled: Option<bool>) -> RouteInputs {
    let mut inputs = tty().with_var("ZELLIJ", "0");
    inputs.zellij_version = version;
    inputs.probe.direct = kitty_enabled.map(|enabled| {
        ProbeResult::Replied(ProbeReply {
            kitty: if enabled { KittyReply::Accepted } else { KittyReply::NoReply },
            da1: Some(Da1Reply {
                class: 62,
                attributes: vec![22],
            }),
        })
    });
    inputs
}

fn v(major: u32, minor: u32, patch: u32) -> Option<ToolVersion> {
    Some(ToolVersion::new(major, minor, patch))
}

/// Asserts every field of a route. Every route resolution produces today is
/// direct and ordinary, so those are fixed here.
fn check(
    route: &GraphicsRoute,
    format: GraphicsFormat,
    confidence: RouteConfidence,
    kind: RouteReasonKind,
    fix: Option<&str>,
    phrases: &[&str],
) {
    assert_eq!(route.format, format, "format: {route:?}");
    assert_eq!(route.transport, GraphicsTransport::Direct, "transport: {route:?}");
    assert_eq!(route.placement, GraphicsPlacement::Ordinary, "placement: {route:?}");
    assert_eq!(route.confidence, confidence, "confidence: {route:?}");
    assert_eq!(route.reason.kind, kind, "reason kind: {route:?}");
    match fix {
        None => assert_eq!(route.reason.fix, None, "fix: {route:?}"),
        Some(expected) => {
            let actual = route
                .reason
                .fix
                .as_deref()
                .unwrap_or_else(|| panic!("expected a fix containing {expected:?}: {route:?}"));
            assert!(actual.contains(expected), "fix {actual:?} lacks {expected:?}");
        }
    }
    for phrase in phrases {
        assert!(
            route.reason.message.contains(phrase),
            "message {:?} lacks {phrase:?}",
            route.reason.message
        );
    }
}

macro_rules! route_rows {
    ($(
        $name:ident: $inputs:expr =>
            $format:ident, $confidence:ident, $kind:ident,
            fix: $fix:expr, msg: [$($phrase:expr),* $(,)?];
    )*) => {$(
        #[test]
        fn $name() {
            let route = resolve_graphics_route(&$inputs);
            check(
                &route,
                GraphicsFormat::$format,
                RouteConfidence::$confidence,
                RouteReasonKind::$kind,
                $fix,
                &[$($phrase),*],
            );
        }
    )*};
}

const KITTY_MSG: &str = "known to render the Kitty graphics protocol";
const ITERM2_MSG: &str = "known to render the iTerm2 image format";
const NOT_IDENTIFIED: &str = "terminal not identified";
const GRAPHICS_LIST: &str = "auto, kitty, iterm2, sixel, none";

// Direct identities.
route_rows! {
    direct_term_program_kitty: tty().with_var("TERM_PROGRAM", "kitty")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG, "TERM_PROGRAM=kitty"];
    direct_term_program_ghostty: tty().with_var("TERM_PROGRAM", "ghostty")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    direct_term_program_wezterm: tty().with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    direct_term_program_warp: tty().with_var("TERM_PROGRAM", "Warp")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    direct_term_program_warp_terminal: tty().with_var("TERM_PROGRAM", "WarpTerminal")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    direct_term_program_konsole: tty().with_var("TERM_PROGRAM", "konsole")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    direct_term_program_wast: tty().with_var("TERM_PROGRAM", "wast")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    direct_term_program_iterm_app: tty().with_var("TERM_PROGRAM", "iTerm.app")
        => Iterm2, Known, Supported, fix: None, msg: [ITERM2_MSG];
    direct_term_program_iterm2: tty().with_var("TERM_PROGRAM", "iterm2")
        => Iterm2, Known, Supported, fix: None, msg: [ITERM2_MSG];
    direct_term_program_apple_terminal: tty().with_var("TERM_PROGRAM", "Apple_Terminal")
        => None, Known, NoSupport, fix: None, msg: ["Terminal.app", "no supported pixel format"];
    direct_term_xterm_kitty_alone: tty().with_var("TERM", "xterm-kitty")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG, "TERM=xterm-kitty"];
    direct_iterm_session_id_alone: tty().with_var("ITERM_SESSION_ID", "w0t0p0:ABC")
        => Iterm2, Known, Supported, fix: None, msg: [ITERM2_MSG, "ITERM_SESSION_ID"];
    direct_generic_term_establishes_nothing: tty().with_var("TERM", "xterm-256color")
        => None, Unknown, Unknown, fix: None, msg: [NOT_IDENTIFIED];
    direct_vscode_is_unknown: tty().with_var("TERM_PROGRAM", "vscode")
        => None, Unknown, Unknown, fix: None, msg: [NOT_IDENTIFIED];
}

// New identities.
route_rows! {
    identity_mintty_term_program: tty().with_var("TERM_PROGRAM", "mintty")
        => Iterm2, Known, Supported, fix: None, msg: [ITERM2_MSG, "mintty"];
    identity_msystem_without_term_program_is_mintty: tty().with_var("MSYSTEM", "MINGW64")
        => Iterm2, Known, Supported, fix: None, msg: [ITERM2_MSG, "mintty", "MSYSTEM"];
    identity_msystem_with_vscode_is_not_mintty:
        tty().with_var("MSYSTEM", "MINGW64").with_var("TERM_PROGRAM", "vscode")
        => None, Unknown, Unknown, fix: None, msg: [NOT_IDENTIFIED];
    identity_cterm_term_program: tty().with_var("TERM_PROGRAM", "cterm")
        => Iterm2, Known, Supported, fix: None, msg: [ITERM2_MSG, "cterm"];
    identity_lc_terminal_iterm2_alone: tty().with_var("LC_TERMINAL", "iTerm2")
        => Iterm2, Known, Supported, fix: None, msg: [ITERM2_MSG, "LC_TERMINAL=iTerm2"];
}

// Nonterminal output.
route_rows! {
    non_tty_wezterm: RouteInputs::new(false).with_var("TERM_PROGRAM", "WezTerm")
        => None, Known, NotATerminal, fix: None, msg: ["not a terminal"];
    non_tty_override_does_not_beat_a_pipe:
        RouteInputs::new(false).with_var("BISCUIT_GRAPHICS", "kitty")
        => None, Known, NotATerminal, fix: None, msg: ["not a terminal"];
}

// SSH.
route_rows! {
    ssh_without_hints_is_unknown: ssh()
        => None, Unknown, Unknown, fix: None, msg: ["SSH"];
    ssh_lc_terminal_beats_term_program:
        ssh().with_var("LC_TERMINAL", "iTerm2").with_var("TERM_PROGRAM", "WezTerm")
        => Iterm2, Known, Supported, fix: None, msg: [ITERM2_MSG, "LC_TERMINAL=iTerm2"];
    local_term_program_beats_lc_terminal:
        tty().with_var("LC_TERMINAL", "iTerm2").with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG, "TERM_PROGRAM=WezTerm"];
    ssh_inherited_wt_session_is_not_identity: ssh().with_var("WT_SESSION", "abc")
        => None, Unknown, Unknown, fix: None, msg: ["SSH"];
    ssh_inherited_iterm_session_id_is_not_identity:
        ssh().with_var("ITERM_SESSION_ID", "w0t0p0:ABC")
        => None, Unknown, Unknown, fix: None, msg: ["SSH"];
}

// Windows OpenSSH server.
route_rows! {
    windows_ssh_wezterm_gets_iterm2: windows_ssh().with_var("TERM_PROGRAM", "WezTerm")
        => Iterm2, Known, Supported, fix: None, msg: ["Windows SSH server", "WezTerm"];
    windows_ssh_iterm_app_gets_iterm2: windows_ssh().with_var("TERM_PROGRAM", "iTerm.app")
        => Iterm2, Known, Supported, fix: None, msg: ["Windows SSH server", "iTerm2"];
    windows_ssh_konsole_gets_iterm2: windows_ssh().with_var("TERM_PROGRAM", "konsole")
        => Iterm2, Known, Supported, fix: None, msg: ["Windows SSH server", "Konsole"];
    windows_ssh_mintty_gets_iterm2: windows_ssh().with_var("TERM_PROGRAM", "mintty")
        => Iterm2, Known, Supported, fix: None, msg: ["Windows SSH server", "mintty"];
    windows_ssh_cterm_gets_iterm2: windows_ssh().with_var("TERM_PROGRAM", "cterm")
        => Iterm2, Known, Supported, fix: None, msg: ["Windows SSH server", "cterm"];
    windows_ssh_lc_terminal_iterm2: windows_ssh().with_var("LC_TERMINAL", "iTerm2")
        => Iterm2, Known, Supported, fix: None, msg: ["Windows SSH server", "LC_TERMINAL=iTerm2"];
    windows_ssh_kitty_is_blocked: windows_ssh().with_var("TERM_PROGRAM", "kitty")
        => None, Known, BlockedTransport, fix: None, msg: ["Windows SSH server", "removes Kitty and Sixel"];
    windows_ssh_ghostty_is_blocked: windows_ssh().with_var("TERM_PROGRAM", "ghostty")
        => None, Known, BlockedTransport, fix: None, msg: ["Windows SSH server"];
    windows_ssh_without_hints_is_blocked: windows_ssh()
        => None, Known, BlockedTransport, fix: None, msg: ["Windows SSH server"];
    windows_ssh_forced_kitty_is_blocked: windows_ssh().with_var("BISCUIT_GRAPHICS", "kitty")
        => None, Forced, BlockedTransport, fix: None, msg: ["BISCUIT_GRAPHICS=kitty", "Kitty"];
    windows_ssh_forced_sixel_is_blocked: windows_ssh().with_var("BISCUIT_GRAPHICS", "sixel")
        => None, Forced, BlockedTransport, fix: None, msg: ["BISCUIT_GRAPHICS=sixel", "Sixel"];
    windows_ssh_forced_iterm2_passes: windows_ssh().with_var("BISCUIT_GRAPHICS", "iterm2")
        => Iterm2, Forced, Supported, fix: None, msg: ["BISCUIT_GRAPHICS=iterm2"];
    // Controls: the same variables off Windows follow the ordinary SSH rules.
    non_windows_ssh_wezterm_control: ssh().with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    non_windows_ssh_without_hints_control: ssh()
        => None, Unknown, Unknown, fix: None, msg: ["SSH"];
    non_windows_ssh_forced_kitty_control: ssh().with_var("BISCUIT_GRAPHICS", "kitty")
        => Kitty, Forced, Supported, fix: None, msg: ["BISCUIT_GRAPHICS=kitty"];
}

// Windows Terminal.
route_rows! {
    windows_terminal_1_22_gets_sixel: windows_terminal(v(1, 22, 0))
        => Sixel, Known, Supported, fix: None, msg: ["Windows Terminal 1.22.0"];
    windows_terminal_1_23_gets_sixel: windows_terminal(v(1, 23, 0))
        => Sixel, Known, Supported, fix: None, msg: ["Windows Terminal"];
    windows_terminal_1_21_predates_sixel: windows_terminal(v(1, 21, 3))
        => None, Known, NoSupport, fix: Some("upgrade Windows Terminal"), msg: ["predates Sixel"];
    windows_terminal_unknown_version: windows_terminal(None)
        => None, Unknown, Unknown, fix: Some("BISCUIT_GRAPHICS=sixel"), msg: ["WT_SESSION"];
    windows_terminal_term_program_wins:
        windows_terminal(v(1, 22, 0)).with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
}

// Zellij.
route_rows! {
    zellij_unknown_version: zellij(None, None).with_var("TERM_PROGRAM", "WezTerm")
        => None, Unknown, Unknown, fix: None, msg: ["version unreadable", "not run"];
    zellij_0_44_predates_kitty: zellij(v(0, 44, 1), None).with_var("TERM_PROGRAM", "kitty")
        => None, Known, NoSupport, fix: Some("upgrade Zellij"), msg: ["predates Kitty graphics"];
    // The pane query replaced the inherited-identity rule: without an answer
    // a Kitty-family host is not enough.
    zellij_0_45_inside_wezterm_without_a_probe_is_unknown:
        zellij(v(0, 45, 0), None).with_var("TERM_PROGRAM", "WezTerm")
        => None, Unknown, Unknown, fix: None, msg: ["Zellij 0.45.0", "not run"];
    zellij_0_45_inside_iterm_never_iterm2:
        zellij(v(0, 45, 0), None).with_var("TERM_PROGRAM", "iTerm.app")
        => None, Unknown, Unknown, fix: None, msg: ["Zellij 0.45.0"];
    zellij_enabled_true_verified: zellij(v(0, 45, 0), Some(true))
        => Kitty, Verified, Supported, fix: None, msg: ["answered a Kitty graphics query"];
    zellij_enabled_false_verified: zellij(v(0, 45, 0), Some(false))
        => None, Verified, NoSupport, fix: None, msg: ["Kitty graphics disabled"];
    zellij_enabled_true_below_version_floor: zellij(v(0, 44, 0), Some(true))
        => None, Known, NoSupport, fix: Some("upgrade Zellij"), msg: ["predates Kitty graphics"];
    zellij_enabled_false_unknown_version: zellij(None, Some(false))
        => None, Verified, NoSupport, fix: None, msg: ["version unreadable", "Kitty graphics disabled"];
    zellij_forced_iterm2_is_blocked:
        zellij(v(0, 45, 0), None).with_var("BISCUIT_GRAPHICS", "iterm2")
        => None, Forced, BlockedTransport, fix: None, msg: ["BISCUIT_GRAPHICS=iterm2", "Zellij"];
    zellij_forced_kitty: zellij(None, None).with_var("BISCUIT_GRAPHICS", "kitty")
        => Kitty, Forced, Supported, fix: None, msg: ["BISCUIT_GRAPHICS=kitty"];
}

// tmux: the visible-tmux rows live in `graphics_tmux_matrix.rs`.
route_rows! {
    direct_transport_overrides_detected_tmux:
        tty()
            .with_var("BISCUIT_GRAPHICS_TRANSPORT", "direct")
            .with_var("TMUX", "/tmp/tmux-501/default,1,0")
            .with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
}

// BISCUIT_GRAPHICS overrides.
route_rows! {
    override_auto_uses_detection:
        tty().with_var("BISCUIT_GRAPHICS", "auto").with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    override_kitty_on_apple_terminal:
        tty().with_var("BISCUIT_GRAPHICS", "kitty").with_var("TERM_PROGRAM", "Apple_Terminal")
        => Kitty, Forced, Supported, fix: None, msg: ["forced by BISCUIT_GRAPHICS=kitty"];
    override_iterm2_on_apple_terminal:
        tty().with_var("BISCUIT_GRAPHICS", "iterm2").with_var("TERM_PROGRAM", "Apple_Terminal")
        => Iterm2, Forced, Supported, fix: None, msg: ["forced by BISCUIT_GRAPHICS=iterm2"];
    override_sixel_on_apple_terminal:
        tty().with_var("BISCUIT_GRAPHICS", "sixel").with_var("TERM_PROGRAM", "Apple_Terminal")
        => Sixel, Forced, Supported, fix: None, msg: ["forced by BISCUIT_GRAPHICS=sixel"];
    override_none_on_wezterm:
        tty().with_var("BISCUIT_GRAPHICS", "none").with_var("TERM_PROGRAM", "WezTerm")
        => None, Forced, UserDisabled, fix: None, msg: ["BISCUIT_GRAPHICS=none"];
    override_forced_format_is_never_substituted:
        tty().with_var("BISCUIT_GRAPHICS", "kitty").with_var("TERM_PROGRAM", "iTerm.app")
        => Kitty, Forced, Supported, fix: None, msg: ["forced by BISCUIT_GRAPHICS=kitty"];
}

// Input robustness: one shared validator, its shapes tested once.
route_rows! {
    graphics_value_absent_uses_detection: tty().with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    graphics_value_empty_is_invalid: tty().with_var("BISCUIT_GRAPHICS", "")
        => None, Forced, InvalidInput, fix: Some(GRAPHICS_LIST), msg: [GRAPHICS_LIST, "BISCUIT_GRAPHICS"];
    graphics_value_uppercase_is_invalid: tty().with_var("BISCUIT_GRAPHICS", "KITTY")
        => None, Forced, InvalidInput, fix: Some(GRAPHICS_LIST), msg: [GRAPHICS_LIST, "KITTY"];
    graphics_value_unknown_word_is_invalid: tty().with_var("BISCUIT_GRAPHICS", "potato")
        => None, Forced, InvalidInput, fix: Some(GRAPHICS_LIST), msg: [GRAPHICS_LIST, "potato"];
    graphics_value_leading_space_is_invalid: tty().with_var("BISCUIT_GRAPHICS", " kitty")
        => None, Forced, InvalidInput, fix: Some(GRAPHICS_LIST), msg: [GRAPHICS_LIST];
    graphics_value_trailing_newline_is_invalid: tty().with_var("BISCUIT_GRAPHICS", "kitty\n")
        => None, Forced, InvalidInput, fix: Some(GRAPHICS_LIST), msg: [GRAPHICS_LIST];
    graphics_value_invalid_is_never_coerced_to_auto:
        tty().with_var("BISCUIT_GRAPHICS", "KITTY").with_var("TERM_PROGRAM", "WezTerm")
        => None, Forced, InvalidInput, fix: Some(GRAPHICS_LIST), msg: [GRAPHICS_LIST];
    graphics_value_control_characters_are_escaped:
        tty().with_var("BISCUIT_GRAPHICS", "\x1b[31m")
        => None, Forced, InvalidInput, fix: Some(GRAPHICS_LIST), msg: [GRAPHICS_LIST];
    transport_value_direct_control:
        tty().with_var("BISCUIT_GRAPHICS_TRANSPORT", "direct").with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Known, Supported, fix: None, msg: [KITTY_MSG];
    transport_value_uppercase_is_invalid:
        tty().with_var("BISCUIT_GRAPHICS_TRANSPORT", "TMUX").with_var("TERM_PROGRAM", "WezTerm")
        => None, Forced, InvalidInput, fix: Some("auto, direct, tmux"),
           msg: ["BISCUIT_GRAPHICS_TRANSPORT", "auto, direct, tmux"];
}

#[test]
fn graphics_value_control_characters_leave_no_raw_escape() {
    let route = resolve_graphics_route(&tty().with_var("BISCUIT_GRAPHICS", "\x1b[31m"));
    assert!(!route.reason.message.contains('\x1b'), "{:?}", route.reason.message);
    assert!(
        !route.reason.fix.as_deref().unwrap_or_default().contains('\x1b'),
        "{:?}",
        route.reason.fix
    );
}

#[cfg(unix)]
#[test]
fn graphics_value_non_utf8_is_invalid() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let route = resolve_graphics_route(
        &tty()
            .with_var("BISCUIT_GRAPHICS", OsStr::from_bytes(b"kit\xfftty"))
            .with_var("TERM_PROGRAM", "WezTerm"),
    );
    check(
        &route,
        GraphicsFormat::None,
        RouteConfidence::Forced,
        RouteReasonKind::InvalidInput,
        Some(GRAPHICS_LIST),
        &["not valid UTF-8", GRAPHICS_LIST],
    );
}

#[test]
fn image_support_projects_each_format() {
    let rows = [
        (tty().with_var("TERM_PROGRAM", "WezTerm"), ImageSupport::Kitty),
        (tty().with_var("TERM_PROGRAM", "iTerm.app"), ImageSupport::ITerm),
        (windows_terminal(v(1, 22, 0)), ImageSupport::Sixel),
        (tty().with_var("TERM_PROGRAM", "Apple_Terminal"), ImageSupport::None),
        (windows_ssh().with_var("TERM_PROGRAM", "WezTerm"), ImageSupport::ITerm),
    ];
    for (inputs, expected) in rows {
        assert_eq!(
            resolve_graphics_route(&inputs).image_support(),
            expected,
            "{inputs:?}"
        );
    }
}
