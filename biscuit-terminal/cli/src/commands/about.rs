use clap::{Args as ClapArgs, CommandFactory};
use color_eyre::Result;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::discovery::app_metadata::{
    current_config_os_target, ConfigDocument, ConfigFormat, ConfigSource, EnvFactMap, SettingValue,
    SettingLocators, TerminalAppMetadata,
};
use biscuit_terminal::discovery::detection::{get_terminal_app, TerminalApp};
use biscuit_terminal::terminal::Terminal;
use sniff::programs::{find_program_with_source, ProgramMetadata};

use crate::commands::{shared::terminal_for_render, CliContext, Run};

/// Display name and aliases for one supported terminal app.
struct AppName {
    app: TerminalApp,
    canonical: &'static str,
    aliases: &'static [&'static str],
}

/// Supported terminal apps with user-facing canonical names and aliases.
///
/// The order defines both the help/error list and tie-breaking priority for
/// ambiguous prefix/contains matches.
const APP_NAMES: &[AppName] = &[
    AppName {
        app: TerminalApp::Kitty,
        canonical: "Kitty",
        aliases: &[],
    },
    AppName {
        app: TerminalApp::Wezterm,
        canonical: "WezTerm",
        aliases: &["Wezterm"],
    },
    AppName {
        app: TerminalApp::Alacritty,
        canonical: "Alacritty",
        aliases: &[],
    },
    AppName {
        app: TerminalApp::Ghostty,
        canonical: "Ghostty",
        aliases: &[],
    },
    AppName {
        app: TerminalApp::ITerm2,
        canonical: "iTerm2",
        aliases: &["iTerm"],
    },
    AppName {
        app: TerminalApp::AppleTerminal,
        canonical: "Apple Terminal",
        aliases: &["Terminal"],
    },
    AppName {
        app: TerminalApp::WindowsTerminal,
        canonical: "Windows Terminal",
        aliases: &["WindowsTerminal"],
    },
    AppName {
        app: TerminalApp::Warp,
        canonical: "Warp",
        aliases: &[],
    },
    AppName {
        app: TerminalApp::GnomeTerminal,
        canonical: "GNOME Terminal",
        aliases: &["GnomeTerminal"],
    },
    AppName {
        app: TerminalApp::VsCode,
        canonical: "VS Code",
        aliases: &["VSCode", "Code"],
    },
    AppName {
        app: TerminalApp::Konsole,
        canonical: "Konsole",
        aliases: &[],
    },
    AppName {
        app: TerminalApp::Foot,
        canonical: "Foot",
        aliases: &[],
    },
    AppName {
        app: TerminalApp::Contour,
        canonical: "Contour",
        aliases: &[],
    },
];

/// Arguments for `bt about [APP]`.
#[derive(ClapArgs, Debug, Clone)]
pub struct AboutArgs {
    /// Terminal app to report on.
    ///
    /// Matches exact, prefix, then contains against the list of supported apps.
    /// When omitted, the currently detected terminal is used.
    #[arg(value_name = "APP")]
    pub app: Option<String>,
}

impl Run for AboutArgs {
    fn run(self, ctx: &CliContext) -> Result<()> {
        let app = match self.app {
            Some(name) => resolve_app_name(&name)?,
            None => get_terminal_app(),
        };

        let report = build_about_report(&app);

        if ctx.json {
            println!("{}", serde_json::to_string_pretty(&AboutJsonReport::from(&report))?);
            return Ok(());
        }

        let term = terminal_for_render(ctx.plain);
        println!("{}", render_about_report(&report, &term));
        Ok(())
    }
}

/// Resolve an explicit app name to a `TerminalApp`, falling back to prefix and
/// contains matches. Fuzzy matches are case-insensitive.
fn resolve_app_name(name: &str) -> Result<TerminalApp> {
    let input = name.to_lowercase();

    // Exact match.
    for entry in APP_NAMES {
        if entry.canonical.to_lowercase() == input {
            return Ok(entry.app.clone());
        }
        for alias in entry.aliases {
            if alias.to_lowercase() == input {
                return Ok(entry.app.clone());
            }
        }
    }

    // Prefix match.
    for entry in APP_NAMES {
        if entry.canonical.to_lowercase().starts_with(&input) {
            return Ok(entry.app.clone());
        }
        for alias in entry.aliases {
            if alias.to_lowercase().starts_with(&input) {
                return Ok(entry.app.clone());
            }
        }
    }

    // Contains match.
    for entry in APP_NAMES {
        if entry.canonical.to_lowercase().contains(&input) {
            return Ok(entry.app.clone());
        }
        for alias in entry.aliases {
            if alias.to_lowercase().contains(&input) {
                return Ok(entry.app.clone());
            }
        }
    }

    invalid_app_error(name)
}

/// Print a clap-style usage error and exit with code 2.
fn invalid_app_error(name: &str) -> Result<TerminalApp> {
    let valid: Vec<_> = APP_NAMES.iter().map(|entry| entry.canonical).collect();
    let mut cmd = crate::args::Args::command();
    let message = format!(
        "Unknown terminal app: {name}\n\nValid apps: {}",
        valid.join(", ")
    );
    cmd.error(clap::error::ErrorKind::InvalidValue, message).exit()
}

/// The report model for `bt about`.
#[derive(Debug, Serialize)]
pub struct AboutReport {
    /// User-facing app name.
    pub app: String,
    /// Internal enum variant name.
    pub variant: String,
    /// Whether this app is the currently detected terminal.
    pub is_current_terminal: bool,
    /// Installation detection result.
    pub install_status: InstallStatus,
    /// OS target used for config resolution.
    pub os_target: String,
    /// Default config-file candidates for this OS target.
    pub config_candidates: Vec<String>,
    /// Config-relocating environment variables.
    pub env_overrides: Vec<EnvOverrideReport>,
    /// Resolved in-use config file, if any.
    pub resolved_config: Option<ResolvedConfigReport>,
    /// Extracted settings from the resolved config.
    pub settings: Vec<SettingReport>,
    /// Environment facts declared by metadata, with live values only when current.
    pub env_facts: Vec<EnvFactReport>,
    /// Plist cache note for macOS plist-backed apps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plist_cache_note: Option<String>,
    #[serde(skip)]
    config_format: Option<ConfigFormat>,
    #[serde(skip)]
    config_location_env: Vec<EnvOverrideReport>,
    #[serde(skip)]
    config_candidates_by_os: ConfigCandidatesJson,
}

/// The spec-shaped machine-readable report for `bt about --json`.
#[derive(Debug, Serialize)]
struct AboutJsonReport {
    app: String,
    variant: String,
    is_current: bool,
    installed: bool,
    install_status: InstallStatus,
    os_target: String,
    config: ConfigJson,
    env: BTreeMap<String, EnvFactJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plist_cache_note: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConfigJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<ConfigFormat>,
    location_env: Vec<LocationEnvJson>,
    candidates: ConfigCandidatesJson,
    resolved_file: Option<String>,
    resolved_source: Option<ResolvedSourceJson>,
    settings: BTreeMap<String, SettingJson>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct ConfigCandidatesJson {
    linux: Vec<String>,
    macos: Vec<String>,
    windows: Vec<String>,
    wsl1: Vec<String>,
    wsl2: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LocationEnvJson {
    var: &'static str,
    kind: String,
}

#[derive(Debug, Serialize)]
struct ResolvedSourceJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    env_var: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate: Option<usize>,
}

#[derive(Debug, Serialize)]
struct SettingJson {
    path: String,
    value: Option<String>,
}

#[derive(Debug, Serialize)]
struct EnvFactJson {
    vars: Vec<String>,
    value: Option<String>,
}

impl From<&AboutReport> for AboutJsonReport {
    fn from(report: &AboutReport) -> Self {
        let resolved_file = report
            .resolved_config
            .as_ref()
            .map(|config| config.path.clone());
        let resolved_source = report
            .resolved_config
            .as_ref()
            .and_then(|config| resolved_source_json(&config.source));

        Self {
            app: report.app.clone(),
            variant: report.variant.clone(),
            is_current: report.is_current_terminal,
            installed: matches!(report.install_status, InstallStatus::Installed { .. }),
            install_status: report.install_status.clone(),
            os_target: report.os_target.clone(),
            config: ConfigJson {
                format: report.config_format,
                location_env: report
                    .config_location_env
                    .iter()
                    .map(|env| LocationEnvJson {
                        var: env.var,
                        kind: env.kind.clone(),
                    })
                    .collect(),
                candidates: report.config_candidates_by_os.clone(),
                resolved_file,
                resolved_source,
                settings: report
                    .settings
                    .iter()
                    .map(|setting| {
                        (
                            setting.key.clone(),
                            SettingJson {
                                path: setting.locator.clone(),
                                value: match &setting.value {
                                    SettingValue::Found { value } => Some(value.clone()),
                                    _ => None,
                                },
                            },
                        )
                    })
                    .collect(),
            },
            env: report
                .env_facts
                .iter()
                .map(|fact| {
                    (
                        fact.key.clone(),
                        EnvFactJson {
                            vars: fact.vars.clone(),
                            value: fact.value.clone(),
                        },
                    )
                })
                .collect(),
            plist_cache_note: report.plist_cache_note.clone(),
        }
    }
}

/// Installation detection result.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum InstallStatus {
    /// Found an executable or macOS bundle for the app.
    Installed {
        /// Path to the discovered executable/bundle binary.
        path: String,
        /// How the installation was discovered.
        source: String,
    },
    /// The app has a probe target but nothing was found.
    NotInstalled,
    /// The app has no binary/bundle probe target in the metadata.
    Unknown,
}

/// A config-relocating environment variable.
#[derive(Debug, Clone, Serialize)]
pub struct EnvOverrideReport {
    /// Variable name.
    pub var: &'static str,
    /// Whether the variable holds a directory or a file.
    pub kind: String,
    /// Current value of the variable, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Resolved config file with provenance.
#[derive(Debug, Serialize)]
pub struct ResolvedConfigReport {
    /// Resolved path.
    pub path: String,
    /// Human-readable provenance.
    pub source: String,
}

/// One setting extraction result.
#[derive(Debug, Serialize)]
pub struct SettingReport {
    /// Stable machine-readable setting key.
    #[serde(skip)]
    pub key: String,
    /// Setting display name.
    pub name: String,
    /// Locator path in the config file.
    pub locator: String,
    /// Extraction outcome.
    #[serde(flatten)]
    pub value: SettingValue,
}

/// One live environment fact.
#[derive(Debug, Serialize)]
pub struct EnvFactReport {
    /// Stable machine-readable fact key.
    #[serde(skip)]
    pub key: String,
    /// Fact display name.
    pub name: String,
    /// Candidate environment variables, in resolution order.
    pub vars: Vec<String>,
    /// First set candidate value.
    pub value: Option<String>,
}

fn build_about_report(app: &TerminalApp) -> AboutReport {
    let current = get_terminal_app();
    let is_current_terminal = std::mem::discriminant(app) == std::mem::discriminant(&current);

    let meta = app.metadata();
    let os_target = current_config_os_target();
    let resolved = app.get_config_file_resolved();

    let config_candidates = app
        .config_candidate_paths()
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    let env_overrides: Vec<EnvOverrideReport> = meta
        .map(|m| {
            m.config
                .location_env
                .iter()
                .map(|env| EnvOverrideReport {
                    var: env.var,
                    kind: format!("{:?}", env.kind).to_lowercase(),
                    value: std::env::var(env.var).ok(),
                })
                .collect()
        })
        .unwrap_or_default();

    let config_candidates_by_os = meta
        .map(|m| config_candidates_json(&m.config.locations))
        .unwrap_or_default();

    let resolved_config = resolved.as_ref().map(|r| ResolvedConfigReport {
        path: r.path.display().to_string(),
        source: format_config_source(&r.source),
    });

    let settings = if let Some(meta) = meta {
        match resolved.as_ref() {
            Some(resolved) => {
                let doc = ConfigDocument::load(&resolved.path, resolved.format);
                collect_settings(&meta.config.settings, |locator| doc.extract(locator))
            }
            None => collect_settings(&meta.config.settings, |_| SettingValue::Unavailable {
                reason: "no config file",
                value: (),
            }),
        }
    } else {
        Vec::new()
    };

    let env_facts = meta
        .map(|m| collect_env_facts(&m.env_facts, is_current_terminal))
        .unwrap_or_default();

    let plist_cache_note =
        if meta.map(|m| m.config.format == ConfigFormat::Plist).unwrap_or(false) {
            Some(
                "macOS caches plist files via cfprefsd; values shown here may not reflect the \
                 config file on disk until the cache flushes."
                    .to_string(),
            )
        } else {
            None
        };

    let config_location_env = env_overrides.clone();

    AboutReport {
        app: canonical_name(app).to_string(),
        variant: format!("{:?}", app),
        is_current_terminal,
        install_status: detect_install_status(app, meta),
        os_target: format!("{:?}", os_target),
        config_candidates,
        env_overrides,
        resolved_config,
        settings,
        env_facts,
        plist_cache_note,
        config_format: meta.map(|m| m.config.format),
        config_location_env,
        config_candidates_by_os,
    }
}

fn config_candidates_json(
    locations: &biscuit_terminal::discovery::app_metadata::OsConfigLocations,
) -> ConfigCandidatesJson {
    fn templates(
        candidates: &'static [biscuit_terminal::discovery::app_metadata::ConfigCandidate],
    ) -> Vec<String> {
        candidates
            .iter()
            .map(|candidate| candidate.template.to_string())
            .collect()
    }

    ConfigCandidatesJson {
        linux: templates(locations.linux),
        macos: templates(locations.macos),
        windows: templates(locations.windows),
        wsl1: templates(locations.wsl1),
        wsl2: templates(locations.wsl2),
    }
}

fn resolved_source_json(source: &str) -> Option<ResolvedSourceJson> {
    if let Some(var) = source.strip_prefix("environment variable ") {
        return Some(ResolvedSourceJson {
            env_var: Some(var.to_string()),
            candidate: None,
        });
    }

    let candidate = source.strip_prefix("candidate #")?.parse().ok()?;
    Some(ResolvedSourceJson {
        env_var: None,
        candidate: Some(candidate),
    })
}

fn canonical_name(app: &TerminalApp) -> &'static str {
    APP_NAMES
        .iter()
        .find(|entry| std::mem::discriminant(&entry.app) == std::mem::discriminant(app))
        .map(|entry| entry.canonical)
        .unwrap_or("Unknown")
}

fn format_config_source(source: &ConfigSource) -> String {
    match source {
        ConfigSource::EnvVar(var) => format!("environment variable {var}"),
        ConfigSource::Candidate(idx) => format!("candidate #{idx}"),
    }
}

fn detect_install_status(app: &TerminalApp, meta: Option<&TerminalAppMetadata>) -> InstallStatus {
    detect_install_status_with(app, meta, |probe| {
        find_program_with_source(probe).map(|(path, source)| (path, source.to_string()))
    })
}

fn detect_install_status_with(
    app: &TerminalApp,
    meta: Option<&TerminalAppMetadata>,
    finder: impl Fn(&str) -> Option<(PathBuf, String)>,
) -> InstallStatus {
    let Some(meta) = meta else {
        return InstallStatus::Unknown;
    };

    let probes = install_probe_names(app, meta);
    if probes.is_empty() {
        return InstallStatus::Unknown;
    }

    for probe in probes {
        if let Some((path, source)) = finder(probe) {
            return InstallStatus::Installed {
                path: path.display().to_string(),
                source,
            };
        }
    }

    InstallStatus::NotInstalled
}

fn install_probe_names(
    app: &TerminalApp,
    meta: &TerminalAppMetadata,
) -> Vec<&'static str> {
    if let Some(bin_name) = meta.bin_name {
        return vec![bin_name];
    }

    if meta.bundle_id.is_none() {
        return Vec::new();
    }

    match app {
        TerminalApp::ITerm2 => vec![sniff::programs::TerminalApp::ITerm2.binary_name()],
        TerminalApp::Warp => vec![sniff::programs::TerminalApp::Warp.binary_name()],
        TerminalApp::AppleTerminal => vec!["Terminal"],
        _ => Vec::new(),
    }
}

fn collect_settings(
    locators: &SettingLocators,
    value_for: impl Fn(&str) -> SettingValue,
) -> Vec<SettingReport> {
    let all = [
        ("ipc", "IPC", locators.ipc),
        ("font", "Font", locators.font),
        ("font_size", "Font Size", locators.font_size),
        ("theme", "Theme", locators.theme),
        ("background_color", "Background Color", locators.background_color),
        ("opacity", "Opacity", locators.opacity),
        ("foreground_color", "Foreground Color", locators.foreground_color),
        ("cursor_color", "Cursor Color", locators.cursor_color),
        ("cursor_style", "Cursor Style", locators.cursor_style),
        ("selection_colors", "Selection Colors", locators.selection_colors),
        ("color_scheme", "Color Scheme", locators.color_scheme),
        ("bold_font", "Bold Font", locators.bold_font),
        ("italic_font", "Italic Font", locators.italic_font),
        ("line_height", "Line Height", locators.line_height),
        ("window_padding", "Window Padding", locators.window_padding),
        ("scrollback_lines", "Scrollback Lines", locators.scrollback_lines),
        ("shell_program", "Shell Program", locators.shell_program),
    ];

    all.iter()
        .filter_map(|(key, name, locator)| {
            let locator = (*locator)?;
            Some(SettingReport {
                key: (*key).to_string(),
                name: (*name).to_string(),
                locator: locator.path.to_string(),
                value: value_for(locator.path),
            })
        })
        .collect()
}

fn collect_env_facts(map: &EnvFactMap, include_live_values: bool) -> Vec<EnvFactReport> {
    let facts: &[(&str, &str, &[&str])] = &[
        ("pid", "PID", map.pid),
        ("window_id", "Window ID", map.window_id),
        ("pane_id", "Pane ID", map.pane_id),
        ("public_key", "Public Key", map.public_key),
        ("ipc_address", "IPC Address", map.ipc_address),
        ("session_id", "Session ID", map.session_id),
        ("config_dir", "Config Directory", map.config_dir),
        ("resources_dir", "Resources Directory", map.resources_dir),
        ("version", "Version", map.version),
        ("profile", "Profile", map.profile),
    ];

    facts
        .iter()
        .filter(|(_, _, candidates)| !candidates.is_empty())
        .map(|(key, name, candidates)| EnvFactReport {
            key: (*key).to_string(),
            name: (*name).to_string(),
            vars: candidates.iter().map(|var| (*var).to_string()).collect(),
            value: if include_live_values {
                candidates.iter().find_map(|var| std::env::var(var).ok())
            } else {
                None
            },
        })
        .collect()
}

fn render_about_report(report: &AboutReport, term: &Terminal) -> String {
    let mut root = Section::new(HeadingLevel::h1, format!("About {}", report.app));

    // Identity
    let mut identity = Section::new(HeadingLevel::h2, "Identity");
    let mut identity_list = UnorderedList::empty();
    identity_list
        .add(Prose::new(format!(
            "App: <b>{}</b>",
            Prose::escape_text(&report.app)
        )))
        .add(Prose::new(format!(
            "Variant: <b>{}</b>",
            Prose::escape_text(&report.variant)
        )))
        .add(Prose::new(format!(
            "Current Terminal: {}",
            if report.is_current_terminal {
                "<green>yes</green>"
            } else {
                "no"
            }
        )));
    identity.push(identity_list);
    root.push(identity);

    // Install status
    let mut install = Section::new(HeadingLevel::h2, "Install Status");
    let install_content: RenderableTerminalContent = match &report.install_status {
        InstallStatus::Installed { path, source } => {
            let mut list = UnorderedList::empty();
            list.add(Prose::new("<green>Installed</green>"))
                .add(Prose::new(format!(
                    "Path: <b>{}</b>",
                    Prose::escape_text(path)
                )))
                .add(Prose::new(format!(
                    "Source: <b>{}</b>",
                    Prose::escape_text(source)
                )));
            list.into()
        }
        InstallStatus::NotInstalled => Prose::new(
            "<yellow>Not installed</yellow> or not on PATH.",
        )
        .into(),
        InstallStatus::Unknown => Prose::new(
            "<dim>Unknown</dim> — no binary or bundle probe target is defined for this app.",
        )
        .into(),
    };
    install.push(install_content);
    root.push(install);

    // OS target
    let mut os = Section::new(HeadingLevel::h2, "OS Target");
    let mut os_list = UnorderedList::empty();
    os_list.add(Prose::new(format!(
        "<b>{}</b>",
        Prose::escape_text(&report.os_target)
    )));
    os.push(os_list);
    root.push(os);

    // Resolved config
    let mut resolved = Section::new(HeadingLevel::h2, "Resolved Config");
    if let Some(ref config) = report.resolved_config {
        let mut list = UnorderedList::empty();
        list.add(Prose::new(format!(
            "Path: <b>{}</b>",
            Prose::escape_text(&config.path)
        )))
        .add(Prose::new(format!(
            "Source: {}",
            Prose::escape_text(&config.source)
        )));
        resolved.push(list);
    } else {
        resolved.push(Prose::new(
            "<dim>No config file resolved for this app on this host.</dim>",
        ));
    }
    root.push(resolved);

    // Config candidates
    let mut candidates = Section::new(HeadingLevel::h2, "Config Candidates");
    if config_candidate_targets(&report.config_candidates_by_os)
        .iter()
        .all(|(_, paths)| paths.is_empty())
    {
        candidates.push(Prose::new("<dim>No config candidates for any OS target.</dim>"));
    } else {
        let mut table = Table::new()
            .with_title("Config Candidates")
            .with_columns(vec![
                TableColumn::new("OS Target"),
                TableColumn::new("Host"),
                TableColumn::new("Candidate templates"),
            ]);
        for (target, paths) in config_candidate_targets(&report.config_candidates_by_os) {
            let marker = if target == report.os_target {
                "active"
            } else {
                ""
            };
            let candidate_text = if paths.is_empty() {
                "none".to_string()
            } else {
                paths
                    .iter()
                    .map(|path| Prose::escape_text(path))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            table.add_row(vec![
                TableCellContent::Text(target.to_string()),
                TableCellContent::Text(marker.to_string()),
                TableCellContent::Text(candidate_text),
            ]);
        }
        candidates.push(table);
    }
    root.push(candidates);

    // Env overrides
    let mut overrides = Section::new(HeadingLevel::h2, "Config Overrides");
    if report.env_overrides.is_empty() {
        overrides.push(Prose::new("<dim>No config-relocating environment variables.</dim>"));
    } else {
        let mut list = UnorderedList::empty();
        for env in &report.env_overrides {
            let value_text = match &env.value {
                Some(v) => format!("<b>{}</b>", Prose::escape_text(v)),
                None => "<dim>unset</dim>".to_string(),
            };
            list.add(Prose::new(format!(
                "{} ({}): {}",
                Prose::escape_text(env.var),
                Prose::escape_text(&env.kind),
                value_text
            )));
        }
        overrides.push(list);
    }
    root.push(overrides);

    // Settings
    let mut settings = Section::new(HeadingLevel::h2, "Settings");
    if report.settings.is_empty() {
        settings.push(Prose::new("<dim>No settings extracted.</dim>"));
    } else {
        let mut table = Table::new()
            .with_title("Settings")
            .with_columns(vec![
                TableColumn::new("Setting"),
                TableColumn::new("Dot path"),
                TableColumn::new("Status"),
                TableColumn::new("Value"),
            ]);
        for setting in &report.settings {
            let (status, value) = setting_value_text(&setting.value);
            table.add_row(vec![
                TableCellContent::Text(Prose::escape_text(&setting.name)),
                TableCellContent::Text(setting.locator.clone()),
                TableCellContent::Text(status.to_string()),
                TableCellContent::Text(value.unwrap_or_default()),
            ]);
        }
        settings.push(table);
    }
    root.push(settings);

    // Environment facts
    let mut facts = Section::new(HeadingLevel::h2, "Environment Facts");
    if report.env_facts.is_empty() {
        facts.push(Prose::new("<dim>No environment facts declared for this app.</dim>"));
    } else {
        let mut table = Table::new()
            .with_title("Environment Facts")
            .with_columns(vec![
                TableColumn::new("Fact"),
                TableColumn::new("Candidate vars"),
                TableColumn::new("Live value"),
            ]);
        for fact in &report.env_facts {
            let value_text = if report.is_current_terminal {
                fact.value
                    .as_ref()
                    .map(|value| Prose::escape_text(value))
                    .unwrap_or_else(|| "unset".to_string())
            } else {
                "(not current terminal)".to_string()
            };
            table.add_row(vec![
                TableCellContent::Text(Prose::escape_text(&fact.name)),
                TableCellContent::Text(fact.vars.join(", ")),
                TableCellContent::Text(value_text),
            ]);
        }
        facts.push(table);
    }
    root.push(facts);

    // Plist cache note
    if let Some(ref note) = report.plist_cache_note {
        let mut note_section = Section::new(HeadingLevel::h2, "Note");
        note_section.push(Prose::new(Prose::escape_text(note)));
        root.push(note_section);
    }

    root.render(term)
}

fn config_candidate_targets(candidates: &ConfigCandidatesJson) -> [(&'static str, &[String]); 5] {
    [
        ("Linux", candidates.linux.as_slice()),
        ("MacOS", candidates.macos.as_slice()),
        ("Windows", candidates.windows.as_slice()),
        ("Wsl1", candidates.wsl1.as_slice()),
        ("Wsl2", candidates.wsl2.as_slice()),
    ]
}

fn setting_value_text(value: &SettingValue) -> (&'static str, Option<String>) {
    match value {
        SettingValue::Found { value } => ("found", Some(Prose::escape_text(value))),
        SettingValue::Absent => ("absent", None),
        SettingValue::LocatorOnly { reason } => {
            ("locator-only", Some(format!("not extractable ({reason})")))
        }
        SettingValue::Unreadable { reason } => ("unreadable", Some(Prose::escape_text(reason))),
        SettingValue::Unavailable { reason, .. } => ("unavailable", Some((*reason).to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_wins() {
        assert!(matches!(resolve_app_name("Kitty").unwrap(), TerminalApp::Kitty));
    }

    #[test]
    fn alias_match_works() {
        assert!(matches!(
            resolve_app_name("VSCode").unwrap(),
            TerminalApp::VsCode
        ));
    }

    #[test]
    fn prefix_match_falls_back() {
        assert!(matches!(resolve_app_name("Ki").unwrap(), TerminalApp::Kitty));
    }

    #[test]
    fn contains_match_falls_back() {
        assert!(matches!(
            resolve_app_name("term2").unwrap(),
            TerminalApp::ITerm2
        ));
    }

    #[test]
    fn invalid_app_errors() {
        // The function exits the process on error; this is tested at the
        // integration level via assert_cmd. The unit test only covers the
        // happy paths above.
    }

    #[test]
    fn bundle_only_metadata_reports_installed_when_sniff_finder_matches() {
        let app = TerminalApp::ITerm2;
        let status = detect_install_status_with(&app, app.metadata(), |probe| {
            assert_eq!(probe, "iterm2");
            Some((
                PathBuf::from("/Applications/iTerm.app/Contents/MacOS/iTerm2"),
                "macOS App Bundle".to_string(),
            ))
        });

        match status {
            InstallStatus::Installed { path, source } => {
                assert_eq!(path, "/Applications/iTerm.app/Contents/MacOS/iTerm2");
                assert_eq!(source, "macOS App Bundle");
            }
            other => panic!("expected installed status, got {other:?}"),
        }
    }

    #[test]
    fn bundle_only_metadata_reports_not_installed_when_sniff_finder_misses() {
        let app = TerminalApp::Warp;
        let status = detect_install_status_with(&app, app.metadata(), |probe| {
            assert_eq!(probe, "warp-terminal");
            None
        });

        assert!(matches!(status, InstallStatus::NotInstalled));
    }

    #[test]
    fn metadata_without_probe_target_still_reports_unknown() {
        let app = TerminalApp::Other("custom".to_string());
        let status = detect_install_status_with(&app, app.metadata(), |_| {
            panic!("finder should not be called without metadata")
        });

        assert!(matches!(status, InstallStatus::Unknown));
    }
}
