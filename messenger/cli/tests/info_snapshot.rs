use biscuit_terminal::terminal::Terminal;
use messenger_cli::info::{
    DaemonRecord, HelperRecord, InfoReport, RouteRecord, render_json, render_text,
};

fn full_report() -> InfoReport {
    InfoReport {
        host_os: "Linux".into(),
        active_daemon: Some(DaemonRecord {
            name: "dunst".into(),
            vendor: Some("knopwob".into()),
            version: Some("1.9.2".into()),
        }),
        bundle_id: Some("com.example.messenger".into()),
        app_id: Some("MessengerApp".into()),
        helpers: vec![
            HelperRecord {
                name: "dunstify".into(),
                binary_name: "dunstify".into(),
                installed: true,
                path: Some("/usr/bin/dunstify".into()),
                version: Some("1.2.0".into()),
                install_hint: None,
                website: "https://dunst-project.org".into(),
                description: "Customizable notification daemon".into(),
            },
            HelperRecord {
                name: "notify-send".into(),
                binary_name: "notify-send".into(),
                installed: false,
                path: None,
                version: None,
                install_hint: Some("sudo apt install libnotify-bin".into()),
                website: "https://gitlab.gnome.org/GNOME/libnotify".into(),
                description: "Sends desktop notifications".into(),
            },
            HelperRecord {
                name: "terminal-notifier".into(),
                binary_name: "terminal-notifier".into(),
                installed: true,
                path: Some("/usr/local/bin/terminal-notifier".into()),
                version: Some("2.0.0".into()),
                install_hint: None,
                website: "https://github.com/julienXX/terminal-notifier".into(),
                description: "macOS notification sender".into(),
            },
        ],
        election_order: vec!["dunstify".into(), "notify-send".into()],
        routes: vec![
            RouteRecord {
                name: "desktop".into(),
                provider: "Desktop".into(),
                is_default: true,
            },
            RouteRecord {
                name: "ops.slack".into(),
                provider: "Slack".into(),
                is_default: false,
            },
            RouteRecord {
                name: "alerts.discord".into(),
                provider: "Discord".into(),
                is_default: false,
            },
        ],
    }
}

fn empty_report() -> InfoReport {
    InfoReport {
        host_os: "macOS".into(),
        active_daemon: None,
        bundle_id: None,
        app_id: None,
        helpers: Vec::new(),
        election_order: Vec::new(),
        routes: Vec::new(),
    }
}

#[test]
fn snapshot_full_plain_rendering() {
    let report = full_report();
    // Pin color depth so the styled snapshot is byte-identical on every host
    // (env-detected depth otherwise varies between dev and CI). See info.rs.
    let term = Terminal::builder()
        .is_tty(false)
        .width(80)
        .color_depth(biscuit_terminal::discovery::detection::ColorDepth::TrueColor)
        .build();
    let text = render_text(&report, &term);
    insta::assert_snapshot!(text);
}

#[test]
fn snapshot_full_json_rendering() {
    let report = full_report();
    let json = render_json(&report).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn snapshot_empty_plain_rendering() {
    let report = empty_report();
    // Pin color depth so the styled snapshot is byte-identical on every host
    // (env-detected depth otherwise varies between dev and CI). See info.rs.
    let term = Terminal::builder()
        .is_tty(false)
        .width(80)
        .color_depth(biscuit_terminal::discovery::detection::ColorDepth::TrueColor)
        .build();
    let text = render_text(&report, &term);
    insta::assert_snapshot!(text);
}

#[test]
fn snapshot_empty_json_rendering() {
    let report = empty_report();
    let json = render_json(&report).unwrap();
    insta::assert_snapshot!(json);
}
