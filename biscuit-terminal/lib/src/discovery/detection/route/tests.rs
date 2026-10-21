use super::*;

#[test]
fn versions_parse_from_tool_output() {
    assert_eq!(ToolVersion::parse("zellij 0.45.1\n"), Some(ToolVersion::new(0, 45, 1)));
    assert_eq!(ToolVersion::parse("1.22.10942.0"), Some(ToolVersion::new(1, 22, 10942)));
    assert_eq!(ToolVersion::parse("tmux 3.4"), Some(ToolVersion::new(3, 4, 0)));
    assert_eq!(ToolVersion::parse("tmux next-3.5a"), Some(ToolVersion::new(3, 5, 0)));
    assert_eq!(ToolVersion::parse("zellij"), None);
    assert_eq!(ToolVersion::parse("7"), None);
    assert_eq!(ToolVersion::parse(""), None);
}

#[test]
fn version_floors_compare_by_component_not_text() {
    assert!(ToolVersion::new(1, 22, 0) >= WINDOWS_TERMINAL_SIXEL_MIN);
    assert!(ToolVersion::new(1, 100, 0) >= WINDOWS_TERMINAL_SIXEL_MIN);
    assert!(ToolVersion::new(2, 0, 0) >= WINDOWS_TERMINAL_SIXEL_MIN);
    assert!(ToolVersion::new(1, 21, 99999) < WINDOWS_TERMINAL_SIXEL_MIN);
    assert!(ToolVersion::new(1, 9, 0) < WINDOWS_TERMINAL_SIXEL_MIN);
    assert!(ToolVersion::new(0, 45, 0) >= ZELLIJ_KITTY_MIN);
    assert!(ToolVersion::new(0, 44, 9) < ZELLIJ_KITTY_MIN);
}

#[test]
fn reason_display_appends_the_fix_only_when_present() {
    let without = GraphicsRoute::unavailable(
        RouteReasonKind::Unknown,
        RouteConfidence::Unknown,
        "no idea",
    );
    assert_eq!(without.reason.to_string(), "no idea");
    let with = without.with_fix("try this");
    assert_eq!(with.reason.to_string(), "no idea (try this)");
}

/// `from_process` reads exactly the variables resolution consults, keeping
/// a non-UTF-8 override as the raw value so the validator rejects it.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn from_process_keeps_a_non_utf8_override_for_the_validator() {
    use std::os::unix::ffi::OsStrExt;

    let raw = OsStr::from_bytes(b"kit\xffty");
    // SAFETY: serialized; nextest also runs each test in its own process.
    unsafe {
        std::env::set_var(GRAPHICS_ENV, raw);
        std::env::set_var("TERM_PROGRAM", "WezTerm");
    }
    let inputs = RouteInputs::from_process().with_output_is_tty(true);
    unsafe {
        std::env::remove_var(GRAPHICS_ENV);
    }

    assert_eq!(inputs.var_os(GRAPHICS_ENV), Some(raw));
    let route = resolve_graphics_route(&inputs);
    assert_eq!(route.format, GraphicsFormat::None);
    assert_eq!(route.reason.kind, RouteReasonKind::InvalidInput);
    assert!(route.reason.message.contains("not valid UTF-8"), "{}", route.reason.message);
}

#[test]
#[serial_test::serial]
fn from_process_reads_the_override_variables() {
    // SAFETY: serialized; nextest also runs each test in its own process.
    unsafe {
        std::env::set_var(GRAPHICS_ENV, "iterm2");
        std::env::set_var(GRAPHICS_TRANSPORT_ENV, "direct");
        std::env::remove_var("TMUX");
        std::env::remove_var("ZELLIJ");
        std::env::remove_var("SSH_CONNECTION");
    }
    let inputs = RouteInputs::from_process().with_output_is_tty(true);
    unsafe {
        std::env::remove_var(GRAPHICS_ENV);
        std::env::remove_var(GRAPHICS_TRANSPORT_ENV);
    }

    let route = resolve_graphics_route(&inputs);
    assert_eq!(route.format, GraphicsFormat::Iterm2);
    assert_eq!(route.confidence, RouteConfidence::Forced);
}

fn route_with(vars: &[(&str, &str)], output_is_tty: bool) -> GraphicsRoute {
    let inputs = vars
        .iter()
        .fold(RouteInputs::new(output_is_tty), |inputs, (k, v)| inputs.with_var(k, *v));
    resolve_graphics_route(&inputs)
}

/// A caller's force override upgrades only routes stopped by "no support"
/// or "unknown"; every other blocker survives it.
#[test]
fn a_caller_force_lifts_only_no_support_and_unknown() {
    let lifted = [
        route_with(&[("TERM_PROGRAM", "Apple_Terminal")], true),
        route_with(&[("TERM", "xterm-256color")], true),
        GraphicsRoute::from(ImageSupport::None),
    ];
    for route in lifted {
        let forced = route.forced_by_caller();
        assert_eq!(forced.format, GraphicsFormat::Kitty, "{route:?}");
        assert_eq!(forced.confidence, RouteConfidence::Forced);
    }

    let kept = [
        (route_with(&[("BISCUIT_GRAPHICS", "none")], true), RouteReasonKind::UserDisabled),
        (route_with(&[("BISCUIT_GRAPHICS", "KITTY")], true), RouteReasonKind::InvalidInput),
        (route_with(&[("TERM_PROGRAM", "WezTerm")], false), RouteReasonKind::NotATerminal),
    ];
    for (route, kind) in kept {
        assert_eq!(route.reason.kind, kind);
        assert_eq!(route.forced_by_caller(), route, "force must not lift {kind:?}");
    }

    let mut windows_ssh = RouteInputs::new(true).with_var("SSH_CONNECTION", "1 2 3 4");
    windows_ssh.is_windows = true;
    let blocked = resolve_graphics_route(&windows_ssh);
    assert_eq!(blocked.reason.kind, RouteReasonKind::BlockedTransport);
    assert_eq!(blocked.forced_by_caller(), blocked);

    // A pixel route is kept as it is, including a forced Sixel.
    let sixel = route_with(&[("BISCUIT_GRAPHICS", "sixel")], true);
    assert_eq!(sixel.forced_by_caller(), sixel);
    let iterm2 = route_with(&[("TERM_PROGRAM", "iTerm.app")], true);
    assert_eq!(iterm2.forced_by_caller(), iterm2);
}

#[test]
fn a_caller_supplied_image_support_is_a_known_direct_route() {
    for (support, format) in [
        (ImageSupport::Kitty, GraphicsFormat::Kitty),
        (ImageSupport::ITerm, GraphicsFormat::Iterm2),
        (ImageSupport::Sixel, GraphicsFormat::Sixel),
        (ImageSupport::None, GraphicsFormat::None),
    ] {
        let route = GraphicsRoute::from(support.clone());
        assert_eq!(route.format, format);
        assert_eq!(route.transport, GraphicsTransport::Direct);
        assert_eq!(route.placement, GraphicsPlacement::Ordinary);
        assert_eq!(route.confidence, RouteConfidence::Known);
        assert_eq!(route.image_support(), support, "projection round-trips");
    }
}

#[test]
fn the_route_serializes_as_data() {
    let route = route_with(&[("BISCUIT_GRAPHICS", "bogus")], true);
    let json = serde_json::to_value(&route).unwrap();
    assert_eq!(json["format"], "None");
    assert_eq!(json["reason"]["kind"], "InvalidInput");
    assert!(json["reason"]["fix"].as_str().unwrap().contains("auto, kitty, iterm2, sixel, none"));
    let back: GraphicsRoute = serde_json::from_value(json).unwrap();
    assert_eq!(back, route);
}
