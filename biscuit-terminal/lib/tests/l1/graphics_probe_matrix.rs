//! The graphics probe through the public API: the reply parsers, the routes
//! each probe answer resolves to, and which environments are never probed.
//!
//! Every row is pure: probe results are written into `RouteInputs`, never
//! produced by a live terminal (the PTY tests in `level1_graphics_probe`
//! cover the live exchange).

use biscuit_terminal::discovery::detection::{
    Da1Reply, GraphicsFormat, GraphicsPlacement, GraphicsRoute, GraphicsTransport, KittyReply,
    ProbeMode, ProbePlan, ProbeReply, ProbeResult, ProbeSkip, RouteConfidence, RouteInputs,
    RouteReasonKind, ToolVersion, probe::parse_kitty_reply, probe_plan, resolve_graphics_route,
};

const SSH: &str = "10.0.0.1 50000 10.0.0.2 22";
const ID: u32 = 4242;

// ---------------------------------------------------------------------------
// Reply parsers
// ---------------------------------------------------------------------------

macro_rules! kitty_rows {
    ($($name:ident: $bytes:expr => $expected:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_eq!(parse_kitty_reply($bytes, ID), $expected);
        }
    )*};
}

kitty_rows! {
    kitty_ok_for_the_id_is_accepted: b"\x1b_Gi=4242;OK\x1b\\" => KittyReply::Accepted;
    kitty_ok_with_more_control_keys_is_accepted:
        b"\x1b_Gi=4242,p=0;OK\x1b\\" => KittyReply::Accepted;
    kitty_fragment_without_terminator_is_no_reply: b"\x1b_Gi=4242;O" => KittyReply::NoReply;
    kitty_fragment_without_payload_end_is_no_reply: b"\x1b_Gi=4242;OK\x1b" => KittyReply::NoReply;
    kitty_wrong_id_is_no_reply: b"\x1b_Gi=4243;OK\x1b\\" => KittyReply::NoReply;
    kitty_id_prefix_is_not_the_id: b"\x1b_Gi=42420;OK\x1b\\" => KittyReply::NoReply;
    kitty_error_reply_is_rejected: b"\x1b_Gi=4242;ENOTSUPPORTED:unsupported format\x1b\\"
        => KittyReply::Rejected("ENOTSUPPORTED:unsupported format".to_string());
    kitty_reply_inside_typeahead_is_accepted: b"ls -la\x1b_Gi=4242;OK\x1b\\\r"
        => KittyReply::Accepted;
    kitty_reply_after_the_da1_reply_is_still_found: b"\x1b[?62;22c\x1b_Gi=4242;OK\x1b\\"
        => KittyReply::Accepted;
    kitty_reply_among_color_replies_is_accepted:
        b"\x1b]10;rgb:ffff/ffff/ffff\x07\x1b_Gi=4242;OK\x1b\\\x1b]11;rgb:0000/0000/0000\x07"
        => KittyReply::Accepted;
    kitty_duplicate_ok_is_accepted: b"\x1b_Gi=4242;OK\x1b\\\x1b_Gi=4242;OK\x1b\\"
        => KittyReply::Accepted;
    kitty_duplicate_disagreeing_replies_conflict:
        b"\x1b_Gi=4242;OK\x1b\\\x1b_Gi=4242;EINVAL:bad\x1b\\" => KittyReply::Conflicting;
    kitty_other_id_then_ours_is_accepted:
        b"\x1b_Gi=7;EINVAL:x\x1b\\\x1b_Gi=4242;OK\x1b\\" => KittyReply::Accepted;
    kitty_nothing_is_no_reply: b"" => KittyReply::NoReply;
}

/// A reply split at every byte boundary parses only once it is whole.
#[test]
fn kitty_reply_fragmented_at_every_byte_parses_only_when_complete() {
    let whole = b"\x1b_Gi=4242;OK\x1b\\";
    for cut in 0..whole.len() {
        assert_eq!(parse_kitty_reply(&whole[..cut], ID), KittyReply::NoReply, "cut {cut}");
    }
    assert_eq!(parse_kitty_reply(whole, ID), KittyReply::Accepted);
}

fn da1(bytes: &[u8]) -> Option<Da1Reply> {
    Da1Reply::find(bytes)
}

#[test]
fn da1_attribute_4_is_sixel() {
    let reply = da1(b"\x1b[?64;1;4;22c").expect("DA1 reply");
    assert_eq!(reply.class, 64);
    assert_eq!(reply.attributes, vec![1, 4, 22]);
    assert!(reply.has_sixel());
}

#[test]
fn da1_without_attribute_4_is_not_sixel() {
    assert!(!da1(b"\x1b[?62;22c").unwrap().has_sixel());
}

#[test]
fn da1_digit_4_inside_another_number_is_not_sixel() {
    assert!(!da1(b"\x1b[?64;14;42c").unwrap().has_sixel());
}

#[test]
fn da1_class_4_is_not_sixel() {
    assert!(!da1(b"\x1b[?4;22c").unwrap().has_sixel());
}

#[test]
fn da1_with_no_attributes_parses() {
    let reply = da1(b"\x1b[?6c").unwrap();
    assert_eq!((reply.class, reply.attributes.len()), (6, 0));
}

#[test]
fn da1_fragment_or_other_csi_is_not_a_reply() {
    assert_eq!(da1(b"\x1b[?64;4"), None);
    assert_eq!(da1(b"\x1b[12;4R"), None);
    assert_eq!(da1(b"\x1b[?2026;2$y"), None);
    assert_eq!(da1(b"4;4;4c"), None);
}

#[test]
fn da1_after_typeahead_and_other_replies_is_found() {
    let reply = da1(b"x\x1b[?2026;2$y\x1b[?62;4c").unwrap();
    assert!(reply.has_sixel());
}

#[test]
fn probe_reply_keeps_kitty_and_da1_separately() {
    let reply = ProbeReply::parse(b"\x1b_Gi=4242;OK\x1b\\\x1b[?62;4c", ID);
    assert_eq!(reply.kitty, KittyReply::Accepted);
    assert!(reply.da1.unwrap().has_sixel());
}

// ---------------------------------------------------------------------------
// Routes from probe answers
// ---------------------------------------------------------------------------

fn tty() -> RouteInputs {
    RouteInputs::new(true)
}

fn replied(kitty: KittyReply, da1_bytes: Option<&[u8]>) -> ProbeResult {
    ProbeResult::Replied(ProbeReply {
        kitty,
        da1: da1_bytes.and_then(Da1Reply::find),
    })
}

const NO_SIXEL: Option<&[u8]> = Some(b"\x1b[?62;22c");
const SIXEL: Option<&[u8]> = Some(b"\x1b[?62;4;22c");

fn direct(mut inputs: RouteInputs, result: ProbeResult) -> RouteInputs {
    inputs.probe.direct = Some(result);
    inputs
}

fn tunneled(mut inputs: RouteInputs, result: ProbeResult) -> RouteInputs {
    inputs.probe.tunneled = Some(result);
    inputs
}

fn hidden_tmux() -> RouteInputs {
    tty()
        .with_var("SSH_CONNECTION", SSH)
        .with_var("TERM", "tmux-256color")
}

fn declared_tmux() -> RouteInputs {
    tty()
        .with_var("SSH_CONNECTION", SSH)
        .with_var("BISCUIT_GRAPHICS_TRANSPORT", "tmux")
}

fn zellij(version: Option<ToolVersion>) -> RouteInputs {
    let mut inputs = tty().with_var("ZELLIJ", "0");
    inputs.zellij_version = version;
    inputs
}

fn windows_terminal(version: Option<ToolVersion>) -> RouteInputs {
    let mut inputs = tty().with_var("WT_SESSION", "0f0e0d0c-0000-0000-0000-000000000000");
    inputs.windows_terminal_version = version;
    inputs
}

fn v(major: u32, minor: u32) -> Option<ToolVersion> {
    Some(ToolVersion::new(major, minor, 0))
}

#[allow(clippy::too_many_arguments)]
fn check(
    route: &GraphicsRoute,
    format: GraphicsFormat,
    transport: GraphicsTransport,
    confidence: RouteConfidence,
    kind: RouteReasonKind,
    fix: Option<&str>,
    phrases: &[&str],
) {
    assert_eq!(route.format, format, "format: {route:?}");
    assert_eq!(route.transport, transport, "transport: {route:?}");
    assert_eq!(route.placement, GraphicsPlacement::Ordinary, "placement: {route:?}");
    assert_eq!(route.confidence, confidence, "confidence: {route:?}");
    assert_eq!(route.reason.kind, kind, "reason kind: {route:?}");
    match (fix, route.reason.fix.as_deref()) {
        (None, actual) => assert_eq!(actual, None, "fix: {route:?}"),
        (Some(expected), Some(actual)) => {
            assert!(actual.contains(expected), "fix {actual:?} lacks {expected:?}")
        }
        (Some(expected), None) => panic!("expected a fix containing {expected:?}: {route:?}"),
    }
    for phrase in phrases {
        assert!(
            route.reason.message.contains(phrase),
            "message {:?} lacks {phrase:?}",
            route.reason.message
        );
    }
}

macro_rules! probe_rows {
    ($(
        $name:ident: $inputs:expr =>
            $format:ident, $transport:ident, $confidence:ident, $kind:ident,
            fix: $fix:expr, msg: [$($phrase:expr),* $(,)?];
    )*) => {$(
        #[test]
        fn $name() {
            let route = resolve_graphics_route(&$inputs);
            check(
                &route,
                GraphicsFormat::$format,
                GraphicsTransport::$transport,
                RouteConfidence::$confidence,
                RouteReasonKind::$kind,
                $fix,
                &[$($phrase),*],
            );
        }
    )*};
}

// An endpoint the environment cannot identify.
probe_rows! {
    unknown_control_without_a_probe_is_unknown: tty()
        => None, Direct, Unknown, Unknown, fix: None, msg: ["not identified", "not run"];
    unknown_kitty_ok_is_verified_kitty: direct(tty(), replied(KittyReply::Accepted, NO_SIXEL))
        => Kitty, Direct, Verified, Supported, fix: None, msg: ["answered the Kitty graphics query"];
    unknown_kitty_ok_without_da1_is_verified_kitty: direct(tty(), replied(KittyReply::Accepted, None))
        => Kitty, Direct, Verified, Supported, fix: None, msg: ["Kitty"];
    unknown_sixel_da1_is_verified_sixel: direct(tty(), replied(KittyReply::NoReply, SIXEL))
        => Sixel, Direct, Verified, Supported, fix: None, msg: ["DA1 attribute 4"];
    unknown_kitty_beats_sixel: direct(tty(), replied(KittyReply::Accepted, SIXEL))
        => Kitty, Direct, Verified, Supported, fix: None, msg: ["Kitty"];
    unknown_rejection_is_verified_no_support:
        direct(tty(), replied(KittyReply::Rejected("ENOTSUPPORTED:x".into()), NO_SIXEL))
        => None, Direct, Verified, NoSupport, fix: None, msg: ["rejected", "ENOTSUPPORTED"];
    unknown_rejection_with_sixel_is_sixel:
        direct(tty(), replied(KittyReply::Rejected("EINVAL:x".into()), SIXEL))
        => Sixel, Direct, Verified, Supported, fix: None, msg: ["DA1 attribute 4"];
    unknown_da1_only_is_verified_no_support: direct(tty(), replied(KittyReply::NoReply, NO_SIXEL))
        => None, Direct, Verified, NoSupport, fix: None, msg: ["without Kitty graphics or Sixel"];
    unknown_silence_is_unknown_not_negative: direct(tty(), replied(KittyReply::NoReply, None))
        => None, Direct, Unknown, Unknown, fix: None, msg: ["no reply before its deadline"];
    unknown_conflicting_replies_are_unknown: direct(tty(), replied(KittyReply::Conflicting, NO_SIXEL))
        => None, Direct, Unknown, Unknown, fix: None, msg: ["conflicting"];
    unknown_busy_input_skip_is_unknown:
        direct(tty(), ProbeResult::Skipped(ProbeSkip::BusyInput))
        => None, Direct, Unknown, Unknown, fix: None, msg: ["input was waiting"];
    unknown_unavailable_probe_is_unknown:
        direct(tty(), ProbeResult::Skipped(ProbeSkip::Unavailable("open /dev/tty: denied".into())))
        => None, Direct, Unknown, Unknown, fix: None, msg: ["could not run", "denied"];
    ssh_without_hints_kitty_ok_is_verified:
        direct(tty().with_var("SSH_CONNECTION", SSH), replied(KittyReply::Accepted, NO_SIXEL))
        => Kitty, Direct, Verified, Supported, fix: None, msg: ["SSH session"];
    ssh_tunneled_answer_alone_is_ignored_without_a_tmux_hint:
        tunneled(tty().with_var("SSH_CONNECTION", SSH), replied(KittyReply::Accepted, NO_SIXEL))
        => None, Direct, Unknown, Unknown, fix: None, msg: ["SSH session", "not run"];
    known_terminal_ignores_a_probe_result:
        direct(tty().with_var("TERM_PROGRAM", "Apple_Terminal"), replied(KittyReply::Accepted, SIXEL))
        => None, Direct, Known, NoSupport, fix: None, msg: ["Terminal.app"];
    ci_ignores_a_probe_result: {
        let mut inputs = direct(tty(), replied(KittyReply::Accepted, NO_SIXEL));
        inputs.is_ci = true;
        inputs
    } => None, Direct, Unknown, Unknown, fix: None, msg: ["not probed in CI"];
    probe_mode_off_is_reported: {
        let mut inputs = tty();
        inputs.probe_mode = ProbeMode::Off;
        inputs
    } => None, Direct, Unknown, Unknown, fix: None, msg: ["caller turned probing off"];
    probe_env_off_is_reported: tty().with_var("BISCUIT_TERMINAL_PROBE", "off")
        => None, Direct, Unknown, Unknown, fix: None, msg: ["BISCUIT_TERMINAL_PROBE=off"];
}

// `BISCUIT_TERMINAL_PROBE` shares the override validator; one control and one
// invalid shape prove the wiring.
probe_rows! {
    probe_env_auto_control: tty().with_var("BISCUIT_TERMINAL_PROBE", "auto").with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Direct, Known, Supported, fix: None, msg: ["WezTerm"];
    probe_env_off_keeps_known_terminals: tty().with_var("BISCUIT_TERMINAL_PROBE", "off").with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Direct, Known, Supported, fix: None, msg: ["WezTerm"];
    probe_env_uppercase_is_invalid: tty().with_var("BISCUIT_TERMINAL_PROBE", "OFF").with_var("TERM_PROGRAM", "WezTerm")
        => None, Direct, Forced, InvalidInput, fix: Some("auto, off"),
           msg: ["BISCUIT_TERMINAL_PROBE", "auto, off"];
}

// Windows Terminal: either signal selects Sixel.
probe_rows! {
    windows_terminal_unreadable_version_with_sixel_da1:
        direct(windows_terminal(None), replied(KittyReply::NoReply, SIXEL))
        => Sixel, Direct, Verified, Supported, fix: None, msg: ["Windows Terminal", "DA1"];
    windows_terminal_old_version_with_sixel_da1:
        direct(windows_terminal(v(1, 21)), replied(KittyReply::NoReply, SIXEL))
        => Sixel, Direct, Verified, Supported, fix: None, msg: ["DA1"];
    windows_terminal_old_version_without_sixel_da1:
        direct(windows_terminal(v(1, 21)), replied(KittyReply::NoReply, NO_SIXEL))
        => None, Direct, Known, NoSupport, fix: Some("upgrade Windows Terminal"), msg: ["predates Sixel"];
    windows_terminal_unreadable_version_without_sixel_da1:
        direct(windows_terminal(None), replied(KittyReply::NoReply, NO_SIXEL))
        => None, Direct, Unknown, Unknown, fix: Some("BISCUIT_GRAPHICS=sixel"), msg: ["no Sixel"];
    windows_terminal_new_version_needs_no_da1: windows_terminal(v(1, 22))
        => Sixel, Direct, Known, Supported, fix: None, msg: ["1.22.0"];
    // The measured system conhost strips Sixel: neither a Sixel DA1 nor an
    // inherited Windows Terminal identity selects it over Windows SSH.
    windows_ssh_never_selects_sixel_from_a_sixel_reply: {
        let mut inputs = direct(windows_terminal(v(1, 22)), replied(KittyReply::NoReply, SIXEL))
            .with_var("SSH_CONNECTION", SSH);
        inputs.is_windows = true;
        inputs
    } => None, Direct, Known, BlockedTransport, fix: None, msg: ["Windows SSH server"];
}

// Zellij: the pane query decides.
probe_rows! {
    zellij_sixel_da1_without_kitty_is_sixel:
        direct(zellij(v(0, 45)), replied(KittyReply::NoReply, SIXEL))
        => Sixel, Direct, Verified, Supported, fix: None, msg: ["Zellij 0.45.0", "Sixel"];
    zellij_older_version_with_sixel_is_sixel:
        direct(zellij(v(0, 44)), replied(KittyReply::NoReply, SIXEL))
        => Sixel, Direct, Verified, Supported, fix: None, msg: ["Zellij 0.44.0"];
    zellij_unknown_version_kitty_ok_is_kitty:
        direct(zellij(None), replied(KittyReply::Accepted, NO_SIXEL))
        => Kitty, Direct, Verified, Supported, fix: None, msg: ["version unreadable"];
    zellij_rejection_is_disabled:
        direct(zellij(v(0, 45)), replied(KittyReply::Rejected("EINVAL:x".into()), NO_SIXEL))
        => None, Direct, Verified, NoSupport, fix: None, msg: ["disabled"];
    zellij_silence_is_unknown: direct(zellij(v(0, 45)), replied(KittyReply::NoReply, None))
        => None, Direct, Unknown, Unknown, fix: None, msg: ["no reply"];
}

#[test]
fn zellij_pixel_routes_need_measured_cells() {
    for result in [
        replied(KittyReply::Accepted, NO_SIXEL),
        replied(KittyReply::NoReply, SIXEL),
    ] {
        let route = resolve_graphics_route(&direct(zellij(v(0, 45)), result));
        assert!(route.has_pixels() && route.needs_measured_cells, "{route:?}");
    }
    let direct_kitty = resolve_graphics_route(&direct(tty(), replied(KittyReply::Accepted, None)));
    assert!(!direct_kitty.needs_measured_cells);
}

// A tmux hidden behind SSH (`TERM=tmux-*`/`screen-*`, no `TMUX`).
probe_rows! {
    hidden_tmux_wrapped_ok_confirms_tmux_transport:
        tunneled(direct(hidden_tmux(), replied(KittyReply::NoReply, NO_SIXEL)), replied(KittyReply::Accepted, NO_SIXEL))
        => Kitty, Tmux, Verified, Supported, fix: None, msg: ["TERM=tmux-256color", "wrapped"];
    hidden_tmux_wrapped_ok_with_forwarded_iterm2_is_iterm2:
        tunneled(hidden_tmux().with_var("LC_TERMINAL", "iTerm2"), replied(KittyReply::Accepted, None))
        => Iterm2, Tmux, Verified, Supported, fix: None, msg: ["wrapped"];
    hidden_tmux_plain_ok_means_no_tmux:
        tunneled(direct(hidden_tmux(), replied(KittyReply::Accepted, NO_SIXEL)), replied(KittyReply::NoReply, None))
        => Kitty, Direct, Verified, Supported, fix: None, msg: ["terminal itself"];
    hidden_tmux_plain_ok_with_forwarded_iterm2_is_iterm2:
        direct(hidden_tmux().with_var("LC_TERMINAL", "iTerm2"), replied(KittyReply::Accepted, None))
        => Iterm2, Direct, Verified, Supported, fix: None, msg: ["terminal itself"];
    hidden_tmux_unconfirmed_names_the_transport_override:
        tunneled(direct(hidden_tmux(), replied(KittyReply::NoReply, NO_SIXEL)), replied(KittyReply::NoReply, None))
        => None, Direct, Unknown, Unknown, fix: Some("BISCUIT_GRAPHICS_TRANSPORT=tmux"),
           msg: ["TERM=tmux-256color", "neither"];
    hidden_tmux_hint_beats_forwarded_identity:
        hidden_tmux().with_var("LC_TERMINAL", "iTerm2")
        => None, Direct, Unknown, Unknown, fix: Some("BISCUIT_GRAPHICS_TRANSPORT=tmux"), msg: ["TERM=tmux-256color"];
    hidden_screen_term_is_a_hint:
        tty().with_var("TERM", "screen-256color")
        => None, Direct, Unknown, Unknown, fix: Some("BISCUIT_GRAPHICS_TRANSPORT=tmux"), msg: ["TERM=screen-256color"];
    hidden_bare_tmux_term_is_a_hint:
        tty().with_var("TERM", "tmux")
        => None, Direct, Unknown, Unknown, fix: Some("BISCUIT_GRAPHICS_TRANSPORT=tmux"), msg: ["TERM=tmux"];
    screenlike_term_name_is_not_a_hint:
        tty().with_var("TERM", "screenx")
        => None, Direct, Unknown, Unknown, fix: None, msg: ["not identified"];
    gnu_screen_session_is_not_hidden_tmux:
        tty().with_var("TERM", "screen-256color").with_var("STY", "123.pts-0.host")
        => None, Direct, Unknown, Unknown, fix: None, msg: ["not identified"];
    direct_transport_override_skips_the_hint:
        tty().with_var("TERM", "tmux-256color").with_var("BISCUIT_GRAPHICS_TRANSPORT", "direct")
        => None, Direct, Unknown, Unknown, fix: None, msg: ["not identified"];
}

// `BISCUIT_GRAPHICS_TRANSPORT=tmux` without `TMUX`: a declared tmux beyond SSH.
probe_rows! {
    declared_tmux_to_identified_kitty_terminal:
        declared_tmux().with_var("TERM_PROGRAM", "WezTerm")
        => Kitty, Tmux, Forced, Supported, fix: None, msg: ["unverified", "WezTerm"];
    declared_tmux_to_forwarded_iterm2:
        declared_tmux().with_var("LC_TERMINAL", "iTerm2")
        => Iterm2, Tmux, Forced, Supported, fix: None, msg: ["unverified", "iTerm2"];
    declared_tmux_wrapped_ok_is_verified:
        tunneled(declared_tmux(), replied(KittyReply::Accepted, None))
        => Kitty, Tmux, Verified, Supported, fix: None, msg: ["wrapped"];
    declared_tmux_without_answer_or_identity:
        tunneled(declared_tmux(), replied(KittyReply::NoReply, None))
        => None, Direct, Unknown, Unknown, fix: Some("BISCUIT_GRAPHICS to the outer terminal"), msg: ["not identified"];
    declared_tmux_with_forced_format:
        declared_tmux().with_var("BISCUIT_GRAPHICS", "kitty")
        => Kitty, Tmux, Forced, Supported, fix: None, msg: ["BISCUIT_GRAPHICS=kitty"];
}

// ---------------------------------------------------------------------------
// Which environments are probed
// ---------------------------------------------------------------------------

const BOTH: ProbePlan = ProbePlan {
    direct: true,
    tunneled: true,
    direct_da1_only: false,
};
const TUNNELED: ProbePlan = ProbePlan {
    direct: false,
    tunneled: true,
    direct_da1_only: false,
};

macro_rules! plan_rows {
    ($($name:ident: $inputs:expr => $plan:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_eq!(probe_plan(&$inputs), $plan);
        }
    )*};
}

fn windows(inputs: RouteInputs) -> RouteInputs {
    let mut inputs = inputs;
    inputs.is_windows = true;
    inputs
}

plan_rows! {
    plan_unknown_terminal_is_probed: tty() => ProbePlan::DIRECT;
    plan_ssh_without_hints_is_probed: tty().with_var("SSH_CONNECTION", SSH) => ProbePlan::DIRECT;
    plan_zellij_is_probed: zellij(v(0, 45)) => ProbePlan::DIRECT;
    plan_zellij_inside_kitty_is_probed: zellij(v(0, 45)).with_var("TERM_PROGRAM", "kitty") => ProbePlan::DIRECT;
    plan_hidden_tmux_sends_both: hidden_tmux() => BOTH;
    plan_declared_tmux_unidentified_sends_wrapped_only: declared_tmux() => TUNNELED;
    plan_windows_terminal_unknown_version_is_probed: windows_terminal(None) => ProbePlan::DIRECT;
    plan_windows_with_unknown_term_program_is_probed:
        windows(tty().with_var("TERM_PROGRAM", "vscode")) => ProbePlan::DIRECT;

    plan_never_known_compatible: tty().with_var("TERM_PROGRAM", "WezTerm") => ProbePlan::NONE;
    plan_never_known_iterm2: tty().with_var("TERM_PROGRAM", "iTerm.app") => ProbePlan::NONE;
    plan_never_known_incompatible: tty().with_var("TERM_PROGRAM", "Apple_Terminal") => ProbePlan::NONE;
    plan_never_windows_terminal_1_22: windows_terminal(v(1, 22)) => ProbePlan::NONE;
    plan_never_declared_tmux_to_identified_terminal:
        declared_tmux().with_var("TERM_PROGRAM", "WezTerm") => ProbePlan::NONE;
    plan_never_pipe: RouteInputs::new(false) => ProbePlan::NONE;
    plan_never_ci: {
        let mut inputs = tty();
        inputs.is_ci = true;
        inputs
    } => ProbePlan::NONE;
    plan_never_ci_hidden_tmux: {
        let mut inputs = hidden_tmux();
        inputs.is_ci = true;
        inputs
    } => ProbePlan::NONE;
    plan_never_disabled: tty().with_var("BISCUIT_GRAPHICS", "none") => ProbePlan::NONE;
    plan_never_forced_format: tty().with_var("BISCUIT_GRAPHICS", "kitty") => ProbePlan::NONE;
    plan_never_invalid_override: tty().with_var("BISCUIT_GRAPHICS", "potato") => ProbePlan::NONE;
    plan_never_invalid_transport: tty().with_var("BISCUIT_GRAPHICS_TRANSPORT", "") => ProbePlan::NONE;
    plan_never_probe_env_off: tty().with_var("BISCUIT_TERMINAL_PROBE", "off") => ProbePlan::NONE;
    plan_never_probe_env_invalid: tty().with_var("BISCUIT_TERMINAL_PROBE", "no") => ProbePlan::NONE;
    plan_never_probe_mode_off: {
        let mut inputs = zellij(v(0, 45));
        inputs.probe_mode = ProbeMode::Off;
        inputs
    } => ProbePlan::NONE;
    plan_never_windows_ssh: windows(tty().with_var("SSH_CONNECTION", SSH)) => ProbePlan::NONE;
    plan_never_windows_ssh_with_tmux_term:
        windows(hidden_tmux()) => ProbePlan::NONE;
    plan_never_legacy_windows_console: windows(tty()) => ProbePlan::NONE;
    plan_never_visible_tmux: tty().with_var("TMUX", "/tmp/tmux-501/default,1,0") => ProbePlan::NONE;
}

#[test]
fn legacy_windows_console_is_no_support() {
    let route = resolve_graphics_route(&windows(tty()));
    assert_eq!(route.reason.kind, RouteReasonKind::NoSupport, "{route:?}");
    assert!(route.reason.message.contains("Windows console"));
}
