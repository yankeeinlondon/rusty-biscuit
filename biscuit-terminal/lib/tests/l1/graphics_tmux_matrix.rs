//! Routes through a visible tmux (`TMUX` set), through the public API: the
//! version floor, `allow-passthrough` on, off, and unreadable, the outer
//! terminal named by the attached clients, whether Kitty is placed with
//! Unicode placeholders, Sixel drawn by tmux itself, and when each probe
//! runs.
//!
//! Every row is pure: tmux facts and probe results are written into
//! `RouteInputs`, never read from a live tmux (the L2 sibling
//! `level2_graphics_tmux` does that).

use biscuit_terminal::discovery::detection::{
    Da1Reply, GraphicsFormat, GraphicsPlacement, GraphicsTransport, KittyReply, Passthrough,
    ProbePlan, ProbeReply, ProbeResult, RouteConfidence, RouteInputs, RouteReasonKind, TmuxClient,
    TmuxFacts, ToolVersion, probe_plan, resolve_graphics_route,
};

const TMUX: &str = "/tmp/tmux-501/default,1234,0";

/// A client tmux sends UTF-8 and 24-bit color (`RGB`), as it does for
/// terminals whose terminfo or identity advertises them.
fn client(termname: &str, termtype: &str) -> TmuxClient {
    client_with(termname, termtype, true, "bpaste,clipboard,focus,RGB,title")
}

fn client_with(termname: &str, termtype: &str, utf8: bool, termfeatures: &str) -> TmuxClient {
    TmuxClient {
        termname: termname.to_string(),
        termtype: termtype.to_string(),
        utf8,
        termfeatures: termfeatures.to_string(),
    }
}

/// A process inside tmux `version` whose pane has `passthrough`, with
/// `clients` attached to its session (`None`: the list was unreadable).
fn tmux(
    version: Option<(u32, u32)>,
    passthrough: Passthrough,
    clients: Option<Vec<TmuxClient>>,
) -> RouteInputs {
    let mut inputs = RouteInputs::new(true)
        .with_var("TMUX", TMUX)
        .with_var("TMUX_PANE", "%3")
        // tmux sets these in every pane; neither names the outer terminal.
        .with_var("TERM_PROGRAM", "tmux")
        .with_var("TERM", "tmux-256color");
    inputs.tmux = TmuxFacts {
        version: version.map(|(major, minor)| ToolVersion::new(major, minor, 0)),
        passthrough,
        clients,
    };
    inputs
}

fn on(clients: &[TmuxClient]) -> RouteInputs {
    tmux(Some((3, 4)), Passthrough::On, Some(clients.to_vec()))
}

fn unread(clients: &[TmuxClient]) -> RouteInputs {
    tmux(Some((3, 4)), Passthrough::Unknown, Some(clients.to_vec()))
}

fn tunneled(mut inputs: RouteInputs, kitty: KittyReply, da1: Option<&[u8]>) -> RouteInputs {
    inputs.probe.tunneled = Some(ProbeResult::Replied(ProbeReply {
        kitty,
        da1: da1.and_then(Da1Reply::find),
    }));
    inputs
}

const DA1: Option<&[u8]> = Some(b"\x1b[?62;22c");

/// A client whose terminal tmux draws Sixel for (`terminal-features`
/// lists `sixel` for it, as tmux's default does for WezTerm).
fn sixel_client(termname: &str) -> TmuxClient {
    client_with(termname, "", true, "bpaste,focus,RGB,sixel,title")
}

/// `inputs` with tmux's own answer to the unwrapped DA1 request.
fn tmux_da1(mut inputs: RouteInputs, da1: Option<&[u8]>) -> RouteInputs {
    inputs.probe.direct = Some(ProbeResult::Replied(ProbeReply {
        kitty: KittyReply::NoReply,
        da1: da1.and_then(Da1Reply::find),
    }));
    inputs
}

/// tmux 3.4 or later built with Sixel answers DA1 with attribute 4.
const TMUX_SIXEL_DA1: Option<&[u8]> = Some(b"\x1b[?1;2;4c");
/// tmux built without Sixel.
const TMUX_PLAIN_DA1: Option<&[u8]> = Some(b"\x1b[?1;2c");

macro_rules! tmux_rows {
    ($(
        $name:ident: $inputs:expr =>
            $format:ident, $transport:ident, $placement:ident, $confidence:ident, $kind:ident,
            fix: $fix:expr, msg: [$($phrase:expr),* $(,)?];
    )*) => {$(
        #[test]
        fn $name() {
            let route = resolve_graphics_route(&$inputs);
            assert_eq!(route.format, GraphicsFormat::$format, "format: {route:?}");
            assert_eq!(route.transport, GraphicsTransport::$transport, "transport: {route:?}");
            assert_eq!(route.placement, GraphicsPlacement::$placement, "placement: {route:?}");
            assert_eq!(route.confidence, RouteConfidence::$confidence, "confidence: {route:?}");
            assert_eq!(route.reason.kind, RouteReasonKind::$kind, "reason kind: {route:?}");
            let fix: Option<&str> = $fix;
            match fix {
                None => assert_eq!(route.reason.fix, None, "fix: {route:?}"),
                Some(expected) => assert!(
                    route.reason.fix.as_deref().is_some_and(|f| f.contains(expected)),
                    "fix lacks {expected:?}: {route:?}"
                ),
            }
            for phrase in [$($phrase),*] {
                assert!(route.reason.message.contains(phrase), "message lacks {phrase:?}: {route:?}");
            }
        }
    )*};
}

// Version floor: passthrough needs tmux 3.3.
tmux_rows! {
    version_3_2_is_blocked_with_the_version_named:
        tmux(Some((3, 2)), Passthrough::On, Some(vec![client("xterm-kitty", "")]))
        => None, Direct, Ordinary, Known, BlockedTransport, fix: Some("upgrade tmux to 3.3.0"),
           msg: ["tmux 3.2.0", "predates allow-passthrough"];
    version_3_3_is_the_floor:
        tmux(Some((3, 3)), Passthrough::On, Some(vec![client("xterm-kitty", "")]))
        => Kitty, Tmux, Placeholders, Known, Supported, fix: None, msg: ["tmux 3.3.0", "kitty"];
    version_unreadable_falls_back_without_a_fix:
        tmux(None, Passthrough::On, Some(vec![client("xterm-kitty", "")]))
        => None, Direct, Ordinary, Unknown, Unknown, fix: None, msg: ["tmux -V", "3.3.0"];
}

// allow-passthrough on, by outer terminal.
tmux_rows! {
    passthrough_on_kitty_by_termname: on(&[client("xterm-kitty", "")])
        => Kitty, Tmux, Placeholders, Known, Supported, fix: None, msg: ["allow-passthrough on", "kitty"];
    passthrough_on_kitty_by_termtype: on(&[client("xterm-256color", "kitty(0.39.1)")])
        => Kitty, Tmux, Placeholders, Known, Supported, fix: None, msg: ["kitty"];
    passthrough_on_ghostty: on(&[client("xterm-ghostty", "ghostty 1.1.3")])
        => Kitty, Tmux, Placeholders, Known, Supported, fix: None, msg: ["Ghostty"];
    passthrough_on_wezterm_by_termname: on(&[client("wezterm", "")])
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["WezTerm"];
    passthrough_on_iterm2_by_termtype: on(&[client("xterm-256color", "iTerm2 3.5.10")])
        => Iterm2, Tmux, Ordinary, Known, Supported, fix: None, msg: ["iTerm2"];
    passthrough_on_unidentified_client_sends_kitty_unverified:
        on(&[client("xterm-256color", "")])
        => Kitty, Tmux, Ordinary, Unknown, Supported, fix: None, msg: ["not identified", "unverified"];
    passthrough_on_no_client_attached_is_unidentified: on(&[])
        => Kitty, Tmux, Ordinary, Unknown, Supported, fix: None, msg: ["not identified"];
    passthrough_on_client_list_unreadable_is_unidentified:
        tmux(Some((3, 4)), Passthrough::On, None)
        => Kitty, Tmux, Ordinary, Unknown, Supported, fix: None, msg: ["not identified"];
    passthrough_on_ignores_the_servers_inherited_lc_terminal:
        on(&[client("xterm-kitty", "")]).with_var("LC_TERMINAL", "iTerm2")
        => Kitty, Tmux, Placeholders, Known, Supported, fix: None, msg: ["kitty"];
}

// Unicode placeholders: only when every client is Kitty or Ghostty and tmux
// sends each of them UTF-8 and 24-bit color, since the cells carry the image
// ID in their foreground color.
tmux_rows! {
    placeholders_name_themselves_in_the_reason: on(&[client("xterm-kitty", "")])
        => Kitty, Tmux, Placeholders, Known, Supported, fix: None, msg: ["Unicode placeholders"];
    placeholders_for_kitty_and_ghostty_together:
        on(&[client("xterm-kitty", ""), client("xterm-ghostty", "")])
        => Kitty, Tmux, Placeholders, Known, Supported, fix: None, msg: ["kitty", "Ghostty"];
    placeholders_need_rgb: on(&[client_with("xterm-kitty", "", true, "bpaste,title")])
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None,
           msg: ["placed ordinarily", "24-bit color (the RGB terminal feature)"];
    placeholders_need_utf8: on(&[client_with("xterm-kitty", "", false, "RGB")])
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["placed ordinarily", "UTF-8"];
    placeholders_need_rgb_without_any_features: on(&[client_with("xterm-ghostty", "", true, "")])
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["placed ordinarily"];
    placeholders_need_every_client_capable:
        on(&[client("xterm-kitty", ""), client_with("xterm-kitty", "", true, "title")])
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["placed ordinarily"];
    wezterm_with_rgb_is_never_placeholders: on(&[client("wezterm", "")])
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["WezTerm"];
    kitty_and_wezterm_place_ordinarily_without_a_placeholder_note:
        on(&[client("xterm-kitty", ""), client("wezterm", "")])
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["kitty, WezTerm"];
    forced_kitty_to_a_kitty_client_uses_placeholders:
        on(&[client("xterm-kitty", "")]).with_var("BISCUIT_GRAPHICS", "kitty")
        => Kitty, Tmux, Placeholders, Forced, Supported, fix: None, msg: ["Unicode placeholders"];
    forced_iterm2_to_a_kitty_client_is_never_placeholders:
        on(&[client("xterm-kitty", "")]).with_var("BISCUIT_GRAPHICS", "iterm2")
        => Iterm2, Tmux, Ordinary, Forced, Supported, fix: None, msg: ["BISCUIT_GRAPHICS=iterm2"];
    unidentified_client_verified_by_the_probe_places_ordinarily:
        tunneled(unread(&[client("xterm-256color", "")]), KittyReply::Accepted, None)
        => Kitty, Tmux, Ordinary, Verified, Supported, fix: None, msg: ["answered the Kitty graphics query"];
}

// Clients that disagree: a format all of them render, or fallback.
tmux_rows! {
    clients_kitty_and_wezterm_share_kitty:
        on(&[client("xterm-kitty", ""), client("wezterm", "")])
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["kitty", "WezTerm"];
    clients_iterm2_and_wezterm_share_iterm2:
        on(&[client("xterm-256color", "iTerm2 3.5"), client("wezterm", "")])
        => Iterm2, Tmux, Ordinary, Known, Supported, fix: None, msg: ["iTerm2", "WezTerm"];
    clients_kitty_and_iterm2_share_nothing:
        on(&[client("xterm-kitty", ""), client("xterm-256color", "iTerm2 3.5")])
        => None, Direct, Ordinary, Known, NoSupport, fix: None, msg: ["share no pixel format", "kitty", "iTerm2"];
    clients_iterm2_and_unidentified_share_nothing:
        on(&[client("xterm-256color", "iTerm2 3.5"), client("xterm-256color", "")])
        => None, Direct, Ordinary, Known, NoSupport, fix: None, msg: ["unidentified terminal"];
    clients_kitty_and_unidentified_share_kitty:
        on(&[client("xterm-kitty", ""), client("screen", "")])
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["kitty", "unidentified terminal"];
    duplicate_clients_are_named_once:
        on(&[client("xterm-kitty", ""), client("xterm-kitty", "")])
        => Kitty, Tmux, Placeholders, Known, Supported, fix: None, msg: ["to kitty"];
}

// allow-passthrough off: tunneled graphics are blocked and the setting named.
tmux_rows! {
    passthrough_off_names_the_setting:
        tmux(Some((3, 4)), Passthrough::Off, Some(vec![client("xterm-kitty", "")]))
        => None, Direct, Ordinary, Known, BlockedTransport, fix: Some("set -g allow-passthrough on"),
           msg: ["allow-passthrough off"];
    passthrough_off_blocks_a_forced_format:
        tmux(Some((3, 4)), Passthrough::Off, None).with_var("BISCUIT_GRAPHICS", "kitty")
        => None, Direct, Ordinary, Known, BlockedTransport, fix: Some("allow-passthrough on"),
           msg: ["BISCUIT_GRAPHICS=kitty", "allow-passthrough off"];
}

// allow-passthrough unreadable: unknown (never "off"), refined by the wrapped probe.
tmux_rows! {
    passthrough_unread_without_probe_is_unknown_without_a_fix: unread(&[client("xterm-kitty", "")])
        => None, Direct, Ordinary, Unknown, Unknown, fix: None, msg: ["could not be read", "not run"];
    passthrough_unread_silent_probe_is_unknown_without_a_fix:
        tunneled(unread(&[client("xterm-kitty", "")]), KittyReply::NoReply, None)
        => None, Direct, Ordinary, Unknown, Unknown, fix: None, msg: ["could not be read", "no reply"];
    passthrough_unread_wrapped_ok_verifies_the_identified_terminal:
        tunneled(unread(&[client("xterm-kitty", "")]), KittyReply::Accepted, None)
        => Kitty, Tmux, Placeholders, Verified, Supported, fix: None, msg: ["delivered the wrapped probe", "kitty"];
    passthrough_unread_wrapped_da1_alone_proves_passthrough:
        tunneled(unread(&[client("xterm-256color", "iTerm2 3.5")]), KittyReply::NoReply, DA1)
        => Iterm2, Tmux, Ordinary, Verified, Supported, fix: None, msg: ["iTerm2"];
    passthrough_unread_unidentified_wrapped_ok_is_kitty:
        tunneled(unread(&[]), KittyReply::Accepted, DA1)
        => Kitty, Tmux, Ordinary, Verified, Supported, fix: None, msg: ["answered the Kitty graphics query"];
    passthrough_unread_unidentified_answer_without_kitty_is_no_support:
        tunneled(unread(&[]), KittyReply::NoReply, DA1)
        => None, Direct, Ordinary, Verified, NoSupport, fix: None, msg: ["without Kitty graphics"];
    passthrough_unread_conflicting_reply_is_unknown:
        tunneled(unread(&[client("xterm-kitty", "")]), KittyReply::Conflicting, DA1)
        => None, Direct, Ordinary, Unknown, Unknown, fix: None, msg: ["conflicting"];
    passthrough_unread_forced_format_is_sent:
        unread(&[]).with_var("BISCUIT_GRAPHICS", "iterm2")
        => Iterm2, Tmux, Ordinary, Forced, Supported, fix: None, msg: ["BISCUIT_GRAPHICS=iterm2", "tmux 3.4.0"];
}

// Overrides with a visible tmux.
tmux_rows! {
    forced_kitty_through_passthrough:
        on(&[client("xterm-256color", "iTerm2 3.5")]).with_var("BISCUIT_GRAPHICS", "kitty")
        => Kitty, Tmux, Ordinary, Forced, Supported, fix: None, msg: ["forced by BISCUIT_GRAPHICS=kitty", "passthrough"];
    forced_format_keeps_the_version_floor:
        tmux(Some((3, 2)), Passthrough::On, None).with_var("BISCUIT_GRAPHICS", "kitty")
        => None, Direct, Ordinary, Known, BlockedTransport, fix: Some("upgrade tmux"), msg: ["BISCUIT_GRAPHICS=kitty", "3.2.0"];
    declared_tmux_transport_with_tmux_visible_reads_the_facts:
        on(&[client("wezterm", "")]).with_var("BISCUIT_GRAPHICS_TRANSPORT", "tmux")
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["WezTerm"];
    direct_transport_ignores_the_visible_tmux:
        tmux(Some((3, 4)), Passthrough::Off, None)
            .with_var("BISCUIT_GRAPHICS_TRANSPORT", "direct")
            .with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Direct, Ordinary, Known, Supported, fix: None, msg: ["WezTerm"];
}

// Sixel drawn by tmux itself: tmux 3.4+, every client has the `sixel`
// feature, and tmux's own DA1 reports attribute 4. It beats passthrough,
// even to a Kitty-capable client, and needs no passthrough at all.
tmux_rows! {
    native_sixel_beats_kitty_passthrough:
        tmux_da1(on(&[sixel_client("wezterm")]), TMUX_SIXEL_DA1)
        => Sixel, Direct, Ordinary, Verified, Supported, fix: None,
           msg: ["tmux 3.4.0 draws Sixel itself", "attribute 4", "sixel terminal feature"];
    native_sixel_works_with_passthrough_off:
        tmux_da1(tmux(Some((3, 4)), Passthrough::Off, Some(vec![sixel_client("wezterm")])), TMUX_SIXEL_DA1)
        => Sixel, Direct, Ordinary, Verified, Supported, fix: None, msg: ["draws Sixel itself"];
    native_sixel_works_with_passthrough_unread:
        tmux_da1(unread(&[sixel_client("xterm-256color")]), TMUX_SIXEL_DA1)
        => Sixel, Direct, Ordinary, Verified, Supported, fix: None, msg: ["draws Sixel itself"];
    native_sixel_needs_tmux_3_4:
        tmux_da1(tmux(Some((3, 3)), Passthrough::On, Some(vec![sixel_client("wezterm")])), TMUX_SIXEL_DA1)
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["allow-passthrough on", "WezTerm"];
    native_sixel_needs_attribute_4_in_tmuxs_own_da1:
        tmux_da1(on(&[sixel_client("wezterm")]), TMUX_PLAIN_DA1)
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["WezTerm"];
    native_sixel_needs_every_client_to_have_the_feature:
        tmux_da1(on(&[sixel_client("wezterm"), client("wezterm", "")]), TMUX_SIXEL_DA1)
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["WezTerm"];
    native_sixel_needs_an_attached_client: tmux_da1(on(&[]), TMUX_SIXEL_DA1)
        => Kitty, Tmux, Ordinary, Unknown, Supported, fix: None, msg: ["not identified"];
    native_sixel_needs_a_readable_client_list:
        tmux_da1(tmux(Some((3, 4)), Passthrough::On, None), TMUX_SIXEL_DA1)
        => Kitty, Tmux, Ordinary, Unknown, Supported, fix: None, msg: ["not identified"];
    native_sixel_unanswered_falls_back_to_passthrough:
        tmux_da1(on(&[sixel_client("wezterm")]), None)
        => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["WezTerm"];
    native_sixel_unestablished_with_passthrough_off_names_the_setting:
        tmux_da1(tmux(Some((3, 4)), Passthrough::Off, Some(vec![sixel_client("wezterm")])), TMUX_PLAIN_DA1)
        => None, Direct, Ordinary, Known, BlockedTransport, fix: Some("set -g allow-passthrough on"),
           msg: ["allow-passthrough off"];
    native_sixel_is_not_read_in_ci: {
        let mut inputs = tmux_da1(on(&[sixel_client("wezterm")]), TMUX_SIXEL_DA1);
        inputs.is_ci = true;
        inputs
    } => Kitty, Tmux, Ordinary, Known, Supported, fix: None, msg: ["WezTerm"];
    native_sixel_forced_goes_to_tmux_itself:
        tmux(Some((3, 4)), Passthrough::Off, Some(vec![sixel_client("wezterm")]))
            .with_var("BISCUIT_GRAPHICS", "sixel")
        => Sixel, Direct, Ordinary, Forced, Supported, fix: None,
           msg: ["BISCUIT_GRAPHICS=sixel", "drawn by tmux 3.4.0 itself"];
    native_sixel_forced_without_the_client_feature_uses_passthrough:
        on(&[client("wezterm", "")]).with_var("BISCUIT_GRAPHICS", "sixel")
        => Sixel, Tmux, Ordinary, Forced, Supported, fix: None, msg: ["passthrough"];
    native_sixel_does_not_change_a_forced_kitty:
        on(&[sixel_client("wezterm")]).with_var("BISCUIT_GRAPHICS", "kitty")
        => Kitty, Tmux, Ordinary, Forced, Supported, fix: None, msg: ["passthrough"];
}

macro_rules! plan_rows {
    ($($name:ident: $inputs:expr => $plan:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_eq!(probe_plan(&$inputs), $plan);
        }
    )*};
}

const TUNNELED: ProbePlan = ProbePlan {
    direct: false,
    tunneled: true,
    direct_da1_only: false,
};

/// tmux's own DA1, without the Kitty query tmux would take as a title.
const NATIVE: ProbePlan = ProbePlan {
    direct: true,
    tunneled: false,
    direct_da1_only: true,
};

plan_rows! {
    plan_native_sixel_candidate_asks_tmux_for_its_da1_only: on(&[sixel_client("wezterm")]) => NATIVE;
    plan_native_sixel_candidate_with_passthrough_off: tmux(Some((3, 4)), Passthrough::Off, Some(vec![sixel_client("wezterm")]))
        => NATIVE;
    plan_native_sixel_candidate_with_passthrough_unread_sends_both: unread(&[sixel_client("wezterm")])
        => ProbePlan { tunneled: true, ..NATIVE };
    plan_never_native_below_3_4: tmux(Some((3, 3)), Passthrough::On, Some(vec![sixel_client("wezterm")]))
        => ProbePlan::NONE;
    plan_never_native_without_the_client_feature: on(&[client("wezterm", "")]) => ProbePlan::NONE;
    plan_never_native_forced_sixel: on(&[sixel_client("wezterm")]).with_var("BISCUIT_GRAPHICS", "sixel")
        => ProbePlan::NONE;
    plan_never_native_in_ci: {
        let mut inputs = on(&[sixel_client("wezterm")]);
        inputs.is_ci = true;
        inputs
    } => ProbePlan::NONE;
    plan_passthrough_unread_sends_the_wrapped_probe_only: unread(&[]) => TUNNELED;
    plan_never_passthrough_on: on(&[client("xterm-256color", "")]) => ProbePlan::NONE;
    plan_never_passthrough_off: tmux(Some((3, 4)), Passthrough::Off, None) => ProbePlan::NONE;
    plan_never_version_too_old: tmux(Some((3, 2)), Passthrough::Unknown, None) => ProbePlan::NONE;
    plan_never_version_unreadable: tmux(None, Passthrough::Unknown, None) => ProbePlan::NONE;
    plan_never_forced_format: unread(&[]).with_var("BISCUIT_GRAPHICS", "kitty") => ProbePlan::NONE;
    plan_never_ci: {
        let mut inputs = unread(&[]);
        inputs.is_ci = true;
        inputs
    } => ProbePlan::NONE;
}
