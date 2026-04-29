//! `messenger info` — render host detection and helper election order.
//!
//! The text output uses [`biscuit_terminal::Prose`] markup so users see the
//! same styling vocabulary as the rest of the CLI. The JSON output is a
//! flat record so external tooling can ingest the same data without
//! parsing ANSI escapes.

use std::collections::BTreeMap;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{
    Table as TerminalTable, TableCellContent, TableColumn,
};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use color_eyre::eyre::Result;
use serde::Serialize;
use sniff::os::OsType;
use sniff::programs::ProgramMetadata;
use sniff::programs::find_program::ExecutableIndex;
use sniff::programs::host_capability::HostCapabilities;
use sniff::programs::install_plan::build_install_plan;
use sniff::programs::notification_helpers::InstalledNotificationHelpers;
use sniff::programs::types::InstallationMethod;
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
    let helpers: Vec<HelperRecord> = render_helpers_in_parallel(
        &helper_variants,
        &helpers_info,
        os_type,
        &host,
    );

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
    records.into_iter().map(|r| r.expect("helper record set")).collect()
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
    let version = if installed { info.version(helper).ok() } else { None };
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
    sniff::programs::installer::get_install_command(method)
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
    for helper in default_helper_order(os_type) {
        if !order.contains(&helper) && info.is_installed(helper) {
            order.push(helper);
        }
    }
    order.into_iter().map(|h| h.binary_name().to_string()).collect()
}

fn helper_matches_os(helper: sniff::programs::NotificationHelper, os_type: OsType) -> bool {
    let availability = helper.info().os_availability;
    availability.is_empty() || availability.contains(&os_type)
}

fn default_helper_order(os_type: OsType) -> Vec<sniff::programs::NotificationHelper> {
    use sniff::programs::NotificationHelper as H;
    match os_type {
        OsType::Linux => vec![H::Dunstify, H::NotifySend],
        OsType::MacOS => vec![H::TerminalNotifier, H::Alerter],
        OsType::Windows => vec![H::SnoreToast, H::BurntToast],
        _ => Vec::new(),
    }
}

/// Render `messenger info` in human-friendly text mode.
pub fn render_text(report: &InfoReport) -> String {
    let term = Terminal::default();
    let mut out = String::new();

    out.push_str(&Prose::new(format!("<b>Host OS:</b> {}", report.host_os)).render(&term));
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
            .render(&term),
        );
        out.push('\n');
    }
    if let Some(bundle_id) = &report.bundle_id {
        out.push_str(
            &Prose::new(format!("<b>macOS bundle id:</b> <dim>{}</dim>", bundle_id))
                .render(&term),
        );
        out.push('\n');
    }
    if let Some(app_id) = &report.app_id {
        out.push_str(
            &Prose::new(format!("<b>Windows app id:</b> <dim>{}</dim>", app_id)).render(&term),
        );
        out.push('\n');
    }

    out.push('\n');
    out.push_str(&render_helper_table(report, &term));

    out.push('\n');
    out.push_str(&Prose::new("<b>Election order on this host</b>").render(&term));
    out.push('\n');
    if report.election_order.is_empty() {
        out.push_str(
            &Prose::new("  <dim>(no helpers installed; falling back to native backend)</dim>")
                .render(&term),
        );
        out.push('\n');
    } else {
        for (i, helper) in report.election_order.iter().enumerate() {
            out.push_str(
                &Prose::new(format!("  <dim>{}.</dim> {}", i + 1, helper)).render(&term),
            );
            out.push('\n');
        }
    }

    out.push('\n');
    out.push_str(&Prose::new("<b>Configured routes</b>").render(&term));
    out.push('\n');
    if report.routes.is_empty() {
        out.push_str(
            &Prose::new("  <dim>(none configured; run `messenger setup`)</dim>").render(&term),
        );
        out.push('\n');
    } else {
        for route in &report.routes {
            let marker = if route.is_default { " <green>★</green>" } else { "" };
            out.push_str(
                &Prose::new(format!(
                    "  <b>{}</b> <dim>→</dim> {}{marker}",
                    route.name, route.provider
                ))
                .render(&term),
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
        print!("{}", render_text(&report));
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
        let text = render_text(&report);
        assert!(text.contains("Host OS:"));
        assert!(text.contains("Notification helpers"));
        assert!(text.contains("Election order"));
        assert!(text.contains("Configured routes"));
    }

    #[test]
    fn render_text_marks_default_route_with_star() {
        let mut config = Config::default();
        config.default_route = Some("desktop.local".into());
        config
            .routes
            .insert("desktop.local".into(), RouteConfig::desktop_default());
        let report = build_report(&config, &[]);
        let text = render_text(&report);
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
}
