//! `messenger info` — render host detection and helper election order.
//!
//! The text output uses [`biscuit_terminal::Prose`] markup so users see the
//! same styling vocabulary as the rest of the CLI. The JSON output is a
//! flat record so external tooling can ingest the same data without
//! parsing ANSI escapes.

use std::collections::BTreeMap;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table as TerminalTable, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use color_eyre::eyre::Result;
use serde::Serialize;
use sniff::os::OsType;
use sniff::programs::ProgramMetadata;
use sniff::programs::host_capability::HostCapabilities;
use sniff::programs::notification_helpers::InstalledNotificationHelpers;
use sniff::programs::{
    ExecutableIndex, InstallationMethod, build_install_plan, get_install_command,
};
use strum::IntoEnumIterator;

use crate::config::{Config, RouteConfig, RouteProvider};

/// JSON-friendly record returned by `messenger info --json`.
#[derive(Debug, Serialize)]
pub struct InfoReport {
    pub host_os: String,
    pub active_daemon: Option<DaemonRecord>,
    pub bundle_id: Option<String>,
    pub app_id: Option<String>,
    pub helpers: Vec<HelperRecord>,
    pub election_order: Vec<String>,
    pub routes: Vec<RouteRecord>,
}

#[derive(Debug, Serialize)]
pub struct DaemonRecord {
    pub name: String,
    pub vendor: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HelperRecord {
    pub name: String,
    pub binary_name: String,
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub install_hint: Option<String>,
    pub website: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct RouteRecord {
    pub name: String,
    pub provider: String,
    pub is_default: bool,
}

/// Build the [`InfoReport`] from sniff detection plus the loaded config.
///
/// `config_helpers` is the per-OS `prefer_helpers` list for the current
/// host (after env-var merge); empty means use the library default order.
///
/// Builds an [`ExecutableIndex`] once and reuses it for both
/// [`HostCapabilities`] detection and notification-helper detection so PATH is
/// scanned only once across the whole report. [`HostCapabilities`] is loaded
/// once and threaded through helper rendering so the on-disk cache is read at
/// most once per invocation. Helper records are rendered in parallel because
/// each one may spawn a `--version` subprocess.
pub fn build_report(config: &Config, config_helpers: &[String]) -> InfoReport {
    let index = ExecutableIndex::build();
    let host = HostCapabilities::load_or_detect_with_index(&index);
    let os_type = host.os_type;
    let helpers_info = InstalledNotificationHelpers::new_with_index(&index);

    let helper_variants: Vec<sniff::programs::NotificationHelper> =
        sniff::programs::NotificationHelper::iter().collect();
    let helpers: Vec<HelperRecord> =
        render_helpers_in_parallel(&helper_variants, &helpers_info, os_type, &host);

    let election_order = compute_election_order(&helpers_info, os_type, config_helpers);

    let mut routes: Vec<RouteRecord> = config
        .routes
        .iter()
        .map(|(name, route)| RouteRecord {
            name: name.clone(),
            provider: route.provider().to_string(),
            is_default: config.default_route.as_deref() == Some(name.as_str()),
        })
        .collect();
    routes.sort_by(|a, b| a.name.cmp(&b.name));

    let (bundle_id, app_id) = config
        .routes
        .values()
        .find_map(|r| match r {
            RouteConfig::Desktop { windows, macos, .. } => {
                Some((macos.bundle_id.clone(), windows.app_id.clone()))
            }
            _ => None,
        })
        .unwrap_or((None, None));

    let active_daemon = helpers_info.active_daemon.as_ref().map(|d| DaemonRecord {
        name: d.name.clone(),
        vendor: d.vendor.clone(),
        version: d.version.clone(),
    });

    InfoReport {
        host_os: os_type.to_string(),
        active_daemon,
        bundle_id,
        app_id,
        helpers,
        election_order,
        routes,
    }
}

/// Render every helper record in parallel.
///
/// Version detection spawns a per-helper subprocess with up to a 3-second
/// timeout. Running the helpers serially (originally: a 6-element `map`)
/// pessimised wall time to `sum(per-helper probe time)`; parallelising drops
/// it to `max(per-helper probe time)`, which is a meaningful win on cold
/// macOS where `terminal-notifier --version` and `alerter --version` each
/// pay app-bundle startup costs.
fn render_helpers_in_parallel(
    helpers: &[sniff::programs::NotificationHelper],
    info: &InstalledNotificationHelpers,
    os_type: OsType,
    host: &HostCapabilities,
) -> Vec<HelperRecord> {
    let mut records: Vec<Option<HelperRecord>> = (0..helpers.len()).map(|_| None).collect();
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(helpers.len());
        for (i, slot) in records.iter_mut().enumerate() {
            let helper = helpers[i];
            handles.push(scope.spawn(move || {
                *slot = Some(helper_record(info, helper, os_type, host));
            }));
        }
        for handle in handles {
            let _ = handle.join();
        }
    });
    records
        .into_iter()
        .map(|r| r.expect("helper record set"))
        .collect()
}

fn helper_record(
    info: &InstalledNotificationHelpers,
    helper: sniff::programs::NotificationHelper,
    os_type: OsType,
    host: &HostCapabilities,
) -> HelperRecord {
    let path_info = info.path_with_source(helper);
    let installed = path_info.is_some();
    let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
    let version = if installed {
        info.version(helper).ok()
    } else {
        None
    };
    let install_hint = best_install_hint(helper, os_type, host);

    HelperRecord {
        name: helper.display_name().to_string(),
        binary_name: helper.binary_name().to_string(),
        installed,
        path,
        version,
        install_hint,
        website: helper.website().to_string(),
        description: helper.description().to_string(),
    }
}

/// Best-effort install hint string for a helper, preferring methods whose
/// package manager is actually present on the host.
fn best_install_hint(
    helper: sniff::programs::NotificationHelper,
    os_type: OsType,
    host: &HostCapabilities,
) -> Option<String> {
    let info = helper.info();
    if !info.os_availability.is_empty() && !info.os_availability.contains(&os_type) {
        return None;
    }
    let plan = build_install_plan(&helper, host);
    plan.chosen()
        .map(|option| option.kind.clone())
        .or_else(|| info.installation_methods.first().cloned())
        .as_ref()
        .map(method_to_hint)
}

fn method_to_hint(method: &InstallationMethod) -> String {
    get_install_command(method)
        .unwrap_or_else(|_| format!("{} {}", method.manager_name(), method.package_name()))
}

/// Compute the per-host helper election order from the relevant helpers
/// (filtered by OS) plus the resolved `prefer_helpers` preference list.
fn compute_election_order(
    info: &InstalledNotificationHelpers,
    os_type: OsType,
    config_helpers: &[String],
) -> Vec<String> {
    let env_and_config = crate::config::resolve_prefer_helpers(config_helpers);
    let mut order: Vec<sniff::programs::NotificationHelper> = Vec::new();
    for helper in env_and_config {
        if helper_matches_os(helper, os_type) && info.is_installed(helper) {
            order.push(helper);
        }
    }
    for helper in default_helper_order(os_type, info) {
        if !order.contains(&helper) && info.is_installed(helper) {
            order.push(helper);
        }
    }
    order
        .into_iter()
        .map(|h| h.binary_name().to_string())
        .collect()
}

fn helper_matches_os(helper: sniff::programs::NotificationHelper, os_type: OsType) -> bool {
    let availability = helper.info().os_availability;
    availability.is_empty() || availability.contains(&os_type)
}

/// Default helper election order for a given host OS.
///
/// On Linux the order tracks the active D-Bus notification daemon: when the
/// bus owner is dunst, `dunstify` ships richer features (action callbacks,
/// stack tags, blocking `--wait`) and is preferred. When the daemon is
/// anything else (GNOME Shell, mako, plasma, …) `dunstify`'s `--wait` /
/// `-A` round-trips no longer work, so `notify-send` is the safer first
/// pick. The fallback is consulted in order, so the deprioritized helper
/// still appears in the list — election simply tries the better fit first.
fn default_helper_order(
    os_type: OsType,
    info: &InstalledNotificationHelpers,
) -> Vec<sniff::programs::NotificationHelper> {
    use sniff::programs::NotificationHelper as H;
    match os_type {
        OsType::Linux => {
            let daemon_is_dunst = info
                .active_daemon
                .as_ref()
                .map(|daemon| daemon.name.eq_ignore_ascii_case("dunst"))
                .unwrap_or(false);
            if daemon_is_dunst {
                vec![H::Dunstify, H::NotifySend]
            } else {
                vec![H::NotifySend, H::Dunstify]
            }
        }
        OsType::MacOS => vec![H::TerminalNotifier, H::Alerter],
        OsType::Windows => vec![H::SnoreToast, H::BurntToast],
        _ => Vec::new(),
    }
}

/// Render `messenger info` in human-friendly text mode.
pub fn render_text(report: &InfoReport, term: &Terminal) -> String {
    let mut out = String::new();

    out.push_str(&Prose::new(format!("<b>Host OS:</b> {}", report.host_os)).render(term));
    out.push('\n');
    if let Some(daemon) = &report.active_daemon {
        let vendor = daemon
            .vendor
            .as_deref()
            .map(|v| format!(" <dim>({v})</dim>"))
            .unwrap_or_default();
        let version = daemon
            .version
            .as_deref()
            .map(|v| format!(" v{v}"))
            .unwrap_or_default();
        out.push_str(
            &Prose::new(format!(
                "<b>Active daemon:</b> {}{vendor}{version}",
                daemon.name
            ))
            .render(term),
        );
        out.push('\n');
    }
    if let Some(bundle_id) = &report.bundle_id {
        out.push_str(
            &Prose::new(format!("<b>macOS bundle id:</b> <dim>{}</dim>", bundle_id)).render(term),
        );
        out.push('\n');
    }
    if let Some(app_id) = &report.app_id {
        out.push_str(
            &Prose::new(format!("<b>Windows app id:</b> <dim>{}</dim>", app_id)).render(term),
        );
        out.push('\n');
    }

    out.push('\n');
    out.push_str(&render_helper_table(report, term));

    out.push('\n');
    out.push_str(&Prose::new("<b>Election order on this host</b>").render(term));
    out.push('\n');
    if report.election_order.is_empty() {
        out.push_str(
            &Prose::new("  <dim>(no helpers installed; falling back to native backend)</dim>")
                .render(term),
        );
        out.push('\n');
    } else {
        for (i, helper) in report.election_order.iter().enumerate() {
            out.push_str(&Prose::new(format!("  <dim>{}.</dim> {}", i + 1, helper)).render(term));
            out.push('\n');
        }
    }

    out.push('\n');
    out.push_str(&Prose::new("<b>Configured routes</b>").render(term));
    out.push('\n');
    if report.routes.is_empty() {
        out.push_str(
            &Prose::new("  <dim>(none configured; run `messenger setup`)</dim>").render(term),
        );
        out.push('\n');
    } else {
        for route in &report.routes {
            let marker = if route.is_default {
                " <green>★</green>"
            } else {
                ""
            };
            out.push_str(
                &Prose::new(format!(
                    "  <b>{}</b> <dim>→</dim> {}{marker}",
                    route.name, route.provider
                ))
                .render(term),
            );
            out.push('\n');
        }
    }

    out
}

fn render_helper_table(report: &InfoReport, term: &Terminal) -> String {
    let columns = vec![
        TableColumn::new("Helper"),
        TableColumn::new("Installed")
            .with_alignment(Alignment::Center)
            .with_uniform_alignment(true),
        TableColumn::new("Version"),
        TableColumn::new("Install hint"),
    ];

    let mut table = TerminalTable::new()
        .with_columns(columns)
        .prefer_cursor_alignment();

    for helper in &report.helpers {
        let cells: Vec<TableCellContent> = vec![
            helper.name.clone().into(),
            (if helper.installed { "✓" } else { "—" }).into(),
            helper.version.clone().unwrap_or_default().into(),
            helper.install_hint.clone().unwrap_or_default().into(),
        ];
        table.add_row(cells);
    }

    let mut out = String::new();
    out.push_str(&Prose::new("<b>Notification helpers</b>").render(term));
    out.push('\n');
    out.push_str(&table.display(term).to_string());
    out
}

/// Render `messenger info` as pretty JSON.
pub fn render_json(report: &InfoReport) -> Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}

/// Resolve the helper config list relevant to the current host.
///
/// Picks the per-OS `prefer_helpers` slice from the first desktop route
/// found in the loaded config (the desktop route is targetless and unique
/// per host, so picking the first match is fine for the info renderer).
///
/// Takes `os_type` explicitly so the caller can compute it once per
/// invocation and reuse it.
pub fn config_helpers_for_host(config: &Config, os_type: OsType) -> Vec<String> {
    config
        .routes
        .values()
        .find_map(|r| match r {
            RouteConfig::Desktop {
                windows,
                macos,
                linux,
                ..
            } => Some(match os_type {
                OsType::Linux => linux.prefer_helpers.clone(),
                OsType::MacOS => macos.prefer_helpers.clone(),
                OsType::Windows => windows.prefer_helpers.clone(),
                _ => Vec::new(),
            }),
            _ => None,
        })
        .unwrap_or_default()
}

/// Run the `messenger info` command.
pub fn run(json: bool) -> Result<()> {
    let config = Config::load()?;
    let os_type = sniff::os::detect_os_type();
    let helpers = config_helpers_for_host(&config, os_type);
    let report = build_report(&config, &helpers);
    if json {
        println!("{}", render_json(&report)?);
    } else {
        let term = Terminal::default();
        print!("{}", render_text(&report, &term));
    }
    Ok(())
}

/// Default-formatted route map indexed by name (used by snapshot tests).
#[allow(dead_code)]
pub(crate) fn route_map_for_test(config: &Config) -> BTreeMap<String, RouteProvider> {
    config
        .routes
        .iter()
        .map(|(name, route)| (name.clone(), route.provider()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_config() -> Config {
        Config::default()
    }

    #[test]
    fn build_report_returns_all_helpers() {
        let report = build_report(&empty_config(), &[]);
        assert_eq!(
            report.helpers.len(),
            sniff::programs::NotificationHelper::iter().count()
        );
    }

    #[test]
    fn render_json_round_trips() {
        let report = build_report(&empty_config(), &[]);
        let json = render_json(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("host_os").is_some());
        assert!(parsed.get("helpers").unwrap().is_array());
    }

    #[test]
    fn render_text_includes_host_os_label() {
        let report = build_report(&empty_config(), &[]);
        let term = Terminal::builder().is_tty(false).width(80).build();
        let text = render_text(&report, &term);
        assert!(text.contains("Host OS:"));
        assert!(text.contains("Notification helpers"));
        assert!(text.contains("Election order"));
        assert!(text.contains("Configured routes"));
    }

    #[test]
    fn render_text_marks_default_route_with_star() {
        let mut config = Config {
            default_route: Some("desktop.local".into()),
            ..Default::default()
        };
        config
            .routes
            .insert("desktop.local".into(), RouteConfig::desktop_default());
        let report = build_report(&config, &[]);
        let term = Terminal::builder().is_tty(false).width(80).build();
        let text = render_text(&report, &term);
        assert!(
            text.contains("desktop.local"),
            "expected route name in output: {text}"
        );
    }

    #[test]
    fn config_helpers_for_host_returns_empty_when_no_desktop_route() {
        let config = Config::default();
        assert!(config_helpers_for_host(&config, OsType::MacOS).is_empty());
    }

    /// Build a Linux [`InstalledNotificationHelpers`] populated with both
    /// helpers and an explicit active daemon — used by the rendering tests
    /// to drive deterministic output without touching the real D-Bus.
    fn linux_helpers_with_daemon(
        daemon_name: &str,
    ) -> sniff::programs::InstalledNotificationHelpers {
        use sniff::programs::ExecutableSource;
        use sniff::programs::NotificationHelper as H;
        use sniff::programs::notification_helpers::NotificationDaemon;
        use std::path::PathBuf;

        let mut info = sniff::programs::InstalledNotificationHelpers::default()
            .with_program(
                H::Dunstify,
                PathBuf::from("/usr/bin/dunstify"),
                ExecutableSource::Path,
            )
            .with_program(
                H::NotifySend,
                PathBuf::from("/usr/bin/notify-send"),
                ExecutableSource::Path,
            );
        info.active_daemon = Some(NotificationDaemon {
            name: daemon_name.into(),
            vendor: Some("test-vendor".into()),
            version: Some("1.0.0".into()),
        });
        info
    }

    #[test]
    fn linux_election_order_prefers_dunstify_when_daemon_is_dunst() {
        // Step 3.4: when the active daemon is dunst, dunstify wins because
        // its `--wait` / `-A` round-trips work end-to-end against dunst.
        let info = linux_helpers_with_daemon("dunst");
        let order = compute_election_order(&info, OsType::Linux, &[]);
        assert_eq!(
            order,
            vec!["dunstify".to_string(), "notify-send".to_string()]
        );
    }

    #[test]
    fn linux_election_order_prefers_notify_send_when_daemon_is_not_dunst() {
        // GNOME Shell, mako, plasma, … do not honour dunstify's blocking
        // `--wait`, so notify-send delivers more reliably and ranks first.
        for daemon in ["GNOME Shell", "mako", "Plasma"] {
            let info = linux_helpers_with_daemon(daemon);
            let order = compute_election_order(&info, OsType::Linux, &[]);
            assert_eq!(
                order,
                vec!["notify-send".to_string(), "dunstify".to_string()],
                "election order wrong for daemon {daemon}: {order:?}",
            );
        }
    }

    #[test]
    fn linux_election_order_prefers_notify_send_when_daemon_is_unknown() {
        // No active daemon detected (D-Bus probe failed): treat dunstify as
        // the riskier choice and let notify-send take the first slot.
        use sniff::programs::ExecutableSource;
        use sniff::programs::NotificationHelper as H;
        use std::path::PathBuf;

        let info = sniff::programs::InstalledNotificationHelpers::default()
            .with_program(
                H::Dunstify,
                PathBuf::from("/usr/bin/dunstify"),
                ExecutableSource::Path,
            )
            .with_program(
                H::NotifySend,
                PathBuf::from("/usr/bin/notify-send"),
                ExecutableSource::Path,
            );
        let order = compute_election_order(&info, OsType::Linux, &[]);
        assert_eq!(
            order,
            vec!["notify-send".to_string(), "dunstify".to_string()]
        );
    }

    #[test]
    fn render_text_includes_active_daemon_row() {
        // Step 3.4: the rendered `messenger info` text must surface the
        // detected daemon (name + vendor + version) when present, so users
        // can correlate why a particular helper sits where it does.
        let report = InfoReport {
            host_os: "Linux".into(),
            active_daemon: Some(DaemonRecord {
                name: "dunst".into(),
                vendor: Some("knopwob".into()),
                version: Some("1.9.2".into()),
            }),
            bundle_id: None,
            app_id: None,
            helpers: Vec::new(),
            election_order: vec!["dunstify".into(), "notify-send".into()],
            routes: Vec::new(),
        };

        let term = Terminal::builder().is_tty(false).width(80).build();
        let text = render_text(&report, &term);
        assert!(
            text.contains("Active daemon:"),
            "missing daemon row: {text}"
        );
        assert!(text.contains("dunst"), "missing daemon name: {text}");
        assert!(text.contains("knopwob"), "missing vendor: {text}");
        assert!(text.contains("1.9.2"), "missing version: {text}");
    }

    #[test]
    fn render_text_election_order_lists_both_linux_helpers() {
        let report = InfoReport {
            host_os: "Linux".into(),
            active_daemon: Some(DaemonRecord {
                name: "dunst".into(),
                vendor: None,
                version: None,
            }),
            bundle_id: None,
            app_id: None,
            helpers: Vec::new(),
            election_order: vec!["dunstify".into(), "notify-send".into()],
            routes: Vec::new(),
        };

        let term = Terminal::builder().is_tty(false).width(80).build();
        let text = render_text(&report, &term);
        assert!(text.contains("dunstify"), "missing dunstify row: {text}");
        assert!(
            text.contains("notify-send"),
            "missing notify-send row: {text}"
        );
        // The numbered prefix proves dunstify ranks ahead of notify-send.
        let dunstify_pos = text.find("dunstify").expect("dunstify present");
        let notify_pos = text.find("notify-send").expect("notify-send present");
        assert!(
            dunstify_pos < notify_pos,
            "expected dunstify before notify-send: {text}",
        );
    }

    fn snapshot_report() -> InfoReport {
        InfoReport {
            host_os: "Linux".into(),
            active_daemon: Some(DaemonRecord {
                name: "dunst".into(),
                vendor: Some("knopwob".into()),
                version: Some("1.9.2".into()),
            }),
            bundle_id: None,
            app_id: None,
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
            ],
        }
    }

    #[test]
    fn snapshot_plain_rendering() {
        let report = snapshot_report();
        // Pin color depth so the styled snapshot is byte-identical on every host.
        // Left unset, build() falls back to env-detected depth (TrueColor under a
        // dev COLORTERM, None under CI's TERM=dumb), which made this snapshot drift.
        let term = Terminal::builder()
            .is_tty(false)
            .width(80)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::TrueColor)
            .build();
        let text = render_text(&report, &term);
        insta::assert_snapshot!(text);
    }

    #[test]
    fn snapshot_json_rendering() {
        let report = snapshot_report();
        let json = render_json(&report).unwrap();
        insta::assert_snapshot!(json);
    }
}
