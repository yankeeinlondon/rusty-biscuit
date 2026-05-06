use std::path::PathBuf;

use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState};
use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;
use color_eyre::eyre::{Result, eyre};
use secrecy::SecretString;
use tracing_subscriber::EnvFilter;

const COMPLETIONS_HELP: &str = r#"
SHELL COMPLETIONS

Enable dynamic shell completions for messenger.

Examples:
  # Bash - add to ~/.bashrc or ~/.bash_profile
  echo 'source <(COMPLETE=bash messenger)' >> ~/.bashrc

  # Zsh - add to ~/.zshrc
  echo 'source <(COMPLETE=zsh messenger)' >> ~/.zshrc

  # Fish - add to config
  echo 'COMPLETE=fish messenger | source' >> ~/.config/fish/config.fish

  # Disable completions
  COMPLETE=0
"#;

use messenger_cli::{config, info, install, receipt_store, setup};

use config::{Config, RouteConfig, RouteProvider, RouteUrgency};

#[derive(Parser)]
#[command(name = "messenger", about = "Send messages to any platform")]
#[command(disable_help_subcommand = true)]
struct Cli {
    /// Enable developer tracing output. Repeat for more detail.
    #[arg(long, action = ArgAction::Count, global = true)]
    debug: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(disable_help_subcommand = true)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// Send a message.
    Send {
        /// The message text to send (Markdown supported unless --plain is used).
        ///
        /// Optional: desktop notifications accept a title-only send via `--title`.
        message: Option<String>,

        /// Provider to use for an ad-hoc route.
        #[arg(long)]
        provider: Option<RouteProvider>,

        /// Channel or recipient for an ad-hoc route.
        #[arg(long)]
        channel: Option<String>,

        /// Named route from config file.
        #[arg(long)]
        route: Option<String>,

        /// Reply target as a saved receipt path or MessageRef/SendReceipt JSON.
        #[arg(long)]
        reply_to: Option<String>,

        /// Path to an image to attach.
        #[arg(long)]
        image: Option<PathBuf>,

        /// Path to a file to attach.
        #[arg(long)]
        file: Option<PathBuf>,

        /// Send silently (no notification sound).
        #[arg(long)]
        silent: bool,

        /// Use strict compatibility mode.
        #[arg(long)]
        strict: bool,

        /// Send as plain text (disable Markdown rendering).
        #[arg(long)]
        plain: bool,

        /// Attach a geographic location (format: "LAT,LON").
        #[arg(long, value_name = "LAT,LON")]
        location: Option<String>,

        /// Title for providers that distinguish a summary line (desktop notifications).
        #[arg(long)]
        title: Option<String>,

        /// Subtitle (desktop notifications only).
        #[arg(long)]
        subtitle: Option<String>,

        /// Icon name or path (desktop notifications).
        #[arg(long)]
        icon: Option<String>,

        /// Category / thread identifier (desktop notifications).
        #[arg(long)]
        category: Option<String>,

        /// Urgency level (desktop notifications).
        #[arg(long, value_enum)]
        urgency: Option<RouteUrgency>,

        /// Expiry timeout in milliseconds (desktop notifications).
        #[arg(long, value_name = "MS")]
        timeout_ms: Option<u32>,

        /// Replace an existing desktop notification by its ID.
        #[arg(long, value_name = "ID")]
        replace_id: Option<String>,

        /// Group identifier for desktop notifications that support grouping.
        #[arg(long, value_name = "ID")]
        group_id: Option<String>,

        /// Progress current value (desktop notifications).
        #[arg(long, value_name = "N")]
        progress_current: Option<u32>,

        /// Progress total value (desktop notifications).
        #[arg(long, value_name = "N")]
        progress_total: Option<u32>,

        /// Badge count for app icon (desktop notifications).
        #[arg(long, value_name = "N")]
        badge_count: Option<u32>,

        /// Action button in "id:label" format (desktop notifications). Repeatable.
        #[arg(long, value_name = "ID:LABEL")]
        action: Vec<String>,
    },
    /// Replace an existing desktop notification using a saved receipt.
    Replace {
        /// Path to a saved receipt from a prior desktop send.
        receipt: String,

        /// Updated message text.
        message: Option<String>,

        /// Updated title.
        #[arg(long)]
        title: Option<String>,

        /// Use plain text instead of Markdown.
        #[arg(long)]
        plain: bool,

        /// Updated subtitle.
        #[arg(long)]
        subtitle: Option<String>,

        /// Updated icon name or path.
        #[arg(long)]
        icon: Option<String>,

        /// Updated category.
        #[arg(long)]
        category: Option<String>,

        /// Updated urgency.
        #[arg(long, value_enum)]
        urgency: Option<RouteUrgency>,

        /// Updated timeout in milliseconds.
        #[arg(long, value_name = "MS")]
        timeout_ms: Option<u32>,

        /// Updated progress current value.
        #[arg(long, value_name = "N")]
        progress_current: Option<u32>,

        /// Updated progress total value.
        #[arg(long, value_name = "N")]
        progress_total: Option<u32>,

        /// Updated badge count.
        #[arg(long, value_name = "N")]
        badge_count: Option<u32>,

        /// Path to an image to attach.
        #[arg(long)]
        image: Option<PathBuf>,
    },
    /// Dismiss a delivered desktop notification using a saved receipt.
    Dismiss {
        /// Path to a saved receipt from a prior desktop send.
        receipt: String,
    },
    /// Interactive provider configuration.
    Setup {
        /// Provider to configure.
        provider: Option<RouteProvider>,
    },
    /// Interactive provider configuration (alias for setup).
    Init {
        /// Provider to configure.
        provider: Option<RouteProvider>,
    },
    /// Show host detection, notification helpers, and configured routes.
    Info {
        /// Emit a JSON record instead of styled terminal output.
        #[arg(long)]
        json: bool,
    },
    /// Install one or more notification helpers via the host package manager.
    Install {
        /// Skip interactive selection — install every uninstalled helper that
        /// applies to the host (or every helper named via `--helper`).
        #[arg(long)]
        yes: bool,
        /// Restrict the install set to the named helpers. Repeatable.
        #[arg(long, value_name = "NAME")]
        helper: Vec<String>,
        /// Show what would be installed without executing the commands.
        #[arg(long)]
        dry_run: bool,
    },
    /// Show shell completions setup instructions.
    #[command(after_help = COMPLETIONS_HELP)]
    Completions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedRoute {
    name: Option<String>,
    route: RouteConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    CompleteEnv::with_factory(Cli::command).complete();

    color_eyre::install()?;
    let cli = Cli::parse();
    init_tracing(cli.debug)?;

    match cli.command {
        Commands::Send {
            message,
            provider,
            channel,
            route,
            reply_to,
            image,
            file,
            silent,
            strict,
            plain,
            location,
            title,
            subtitle,
            icon,
            category,
            urgency,
            timeout_ms,
            replace_id,
            group_id,
            progress_current,
            progress_total,
            badge_count,
            action,
        } => {
            send_message(SendArgs {
                message,
                provider,
                channel,
                route,
                reply_to,
                image,
                file,
                silent,
                strict,
                plain,
                location,
                title,
                subtitle,
                icon,
                category,
                urgency,
                timeout_ms,
                replace_id,
                group_id,
                progress_current,
                progress_total,
                badge_count,
                action,
            })
            .await?;
        }
        Commands::Replace {
            receipt,
            message,
            title,
            plain,
            subtitle,
            icon,
            category,
            urgency,
            timeout_ms,
            progress_current,
            progress_total,
            badge_count,
            image,
        } => {
            replace_notification(ReplaceArgs {
                receipt,
                message,
                title,
                plain,
                subtitle,
                icon,
                category,
                urgency,
                timeout_ms,
                progress_current,
                progress_total,
                badge_count,
                image,
            })
            .await?;
        }
        Commands::Dismiss { receipt } => {
            dismiss_notification(receipt).await?;
        }
        Commands::Setup { provider } | Commands::Init { provider } => {
            setup::run(provider)?;
        }
        Commands::Info { json } => {
            info::run(json)?;
        }
        Commands::Install {
            yes,
            helper,
            dry_run,
        } => {
            install::run(install::InstallArgs {
                yes,
                helpers: helper,
                dry_run,
            })?;
        }
        Commands::Completions => {
            print!("{}", COMPLETIONS_HELP.trim_start());
        }
    }

    Ok(())
}

fn init_tracing(debug_level: u8) -> Result<()> {
    let env_log = std::env::var("RUST_LOG").ok();
    if debug_level == 0 && env_log.is_none() {
        return Ok(());
    }

    let default_filter = match debug_level {
        0 | 1 => "messenger=info,messenger_cli=info",
        2 => "messenger=debug,messenger_cli=debug",
        _ => "messenger=trace,messenger_cli=trace",
    };

    let (filter, invalid_env) = match env_log.as_deref() {
        Some(value) => match value.parse::<EnvFilter>() {
            Ok(filter) => (filter, None),
            Err(error) => (
                default_filter.parse::<EnvFilter>()?,
                Some((value.to_owned(), error.to_string())),
            ),
        },
        None => (default_filter.parse::<EnvFilter>()?, None),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .compact()
        .with_target(true)
        .try_init()
        .map_err(|error| eyre!(error.to_string()))?;

    if let Some((value, error)) = invalid_env {
        tracing::warn!(
            env = %value,
            error = %error,
            fallback = %default_filter,
            "invalid RUST_LOG value; using default filter"
        );
    }

    Ok(())
}

struct SendArgs {
    message: Option<String>,
    provider: Option<RouteProvider>,
    channel: Option<String>,
    route: Option<String>,
    reply_to: Option<String>,
    image: Option<PathBuf>,
    file: Option<PathBuf>,
    silent: bool,
    strict: bool,
    plain: bool,
    location: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    icon: Option<String>,
    category: Option<String>,
    urgency: Option<RouteUrgency>,
    timeout_ms: Option<u32>,
    replace_id: Option<String>,
    group_id: Option<String>,
    progress_current: Option<u32>,
    progress_total: Option<u32>,
    badge_count: Option<u32>,
    action: Vec<String>,
}

#[tracing::instrument(skip_all, fields(route = tracing::field::Empty, provider = tracing::field::Empty))]
async fn send_message(args: SendArgs) -> Result<()> {
    let SendArgs {
        message: message_text,
        provider: provider_opt,
        channel: channel_opt,
        route: route_opt,
        reply_to,
        image,
        file,
        silent,
        strict,
        plain,
        location,
        title,
        subtitle,
        icon,
        category,
        urgency,
        timeout_ms,
        replace_id,
        group_id,
        progress_current,
        progress_total,
        badge_count,
        action,
    } = args;

    let message_text = message_text.as_deref().map(unescape);

    let config = Config::load()?;
    let resolved_route = resolve_route(provider_opt, channel_opt, route_opt, &config)?;
    let route_label = resolved_route.name.as_deref().unwrap_or("<ad-hoc>");
    let provider_kind = resolved_route.route.provider();
    tracing::Span::current().record("route", tracing::field::display(route_label));
    tracing::Span::current().record("provider", tracing::field::display(provider_kind));
    tracing::info!(provider = %provider_kind, route = route_label, "starting CLI send");
    tracing::debug!(
        has_reply = reply_to.is_some(),
        has_image = image.is_some(),
        has_file = file.is_some(),
        silent,
        strict,
        plain,
        has_location = location.is_some(),
        has_title = title.is_some(),
        "building message from CLI arguments"
    );

    let mut messenger = messenger::Messenger::new();
    register_provider(&mut messenger, &resolved_route.route)?;

    let mut message = match message_text.as_deref() {
        Some(text) if !text.is_empty() => {
            if plain {
                messenger::Message::text(text)
            } else {
                messenger::Message::markdown(text)
            }
        }
        _ => messenger::Message {
            title: None,
            body: None,
            attachments: Vec::new(),
            location: None,
            metadata: std::collections::BTreeMap::new(),
        },
    };

    if let Some(t) = title {
        message = message.title(t);
    }
    if let Some(ref loc) = location {
        let (lat, lon) = parse_location(loc)?;
        message = message.with_location(lat, lon);
    }
    if let Some(image_path) = image {
        message = message.image(image_path);
    }
    if let Some(file_path) = file {
        message = message.file(file_path);
    }

    let target = build_target(&resolved_route.route)?;
    let mut dispatch = messenger::Dispatch::to(target);

    if let Some(reply_spec) = reply_to {
        dispatch = dispatch.reply_to(receipt_store::load_message_ref(&reply_spec)?);
    }
    if silent {
        dispatch = dispatch.silent();
    }
    if strict {
        dispatch = dispatch.strict();
    }

    if provider_kind == RouteProvider::Desktop {
        let overrides = build_desktop_overrides(DesktopOverrideInputs {
            subtitle,
            icon,
            category,
            urgency,
            timeout_ms,
            replace_id,
            group_id,
            progress_current,
            progress_total,
            badge_count,
            action,
        });
        if !overrides_is_empty(&overrides) {
            dispatch = dispatch.with_overrides(messenger::ProviderOverrides::Desktop(overrides));
        }
    }

    let plan = messenger.plan_send(dispatch, &message)?;
    emit_compatibility_warnings(&plan.warnings);

    let receipt = messenger.send_planned(plan).await?;
    let receipt_path = receipt_store::save_receipt(&receipt, resolved_route.name.as_deref())?;
    tracing::info!(
        provider = %receipt.provider,
        raw_id = %receipt.raw_id,
        receipt_path = %receipt_path.display(),
        "CLI send complete"
    );
    eprintln!(
        "Sent via {} (id: {})\nReceipt: {}",
        receipt.provider,
        receipt.raw_id,
        receipt_path.display()
    );

    Ok(())
}

struct DesktopOverrideInputs {
    subtitle: Option<String>,
    icon: Option<String>,
    category: Option<String>,
    urgency: Option<RouteUrgency>,
    timeout_ms: Option<u32>,
    replace_id: Option<String>,
    group_id: Option<String>,
    progress_current: Option<u32>,
    progress_total: Option<u32>,
    badge_count: Option<u32>,
    action: Vec<String>,
}

fn build_desktop_overrides(inputs: DesktopOverrideInputs) -> messenger::DesktopOverrides {
    let progress = match (inputs.progress_current, inputs.progress_total) {
        (Some(current), Some(total)) => Some(messenger::NotificationProgress { current, total }),
        _ => None,
    };

    let actions = inputs
        .action
        .into_iter()
        .filter_map(|s| {
            let parts: Vec<&str> = s.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some(messenger::NotificationAction {
                    id: parts[0].to_string(),
                    label: parts[1].to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    messenger::DesktopOverrides {
        subtitle: inputs.subtitle,
        app_name: None,
        category: inputs.category,
        urgency: inputs.urgency.map(route_urgency_to_messenger),
        timeout_ms: inputs.timeout_ms,
        icon: inputs.icon.map(icon_string_to_messenger),
        replace_id: inputs.replace_id,
        group_id: inputs.group_id,
        actions,
        progress,
        badge_count: inputs.badge_count,
    }
}

fn overrides_is_empty(o: &messenger::DesktopOverrides) -> bool {
    o.subtitle.is_none()
        && o.app_name.is_none()
        && o.category.is_none()
        && o.urgency.is_none()
        && o.timeout_ms.is_none()
        && o.icon.is_none()
        && o.replace_id.is_none()
        && o.group_id.is_none()
        && o.actions.is_empty()
        && o.progress.is_none()
        && o.badge_count.is_none()
}

fn route_urgency_to_messenger(urgency: RouteUrgency) -> messenger::NotificationUrgency {
    match urgency {
        RouteUrgency::Low => messenger::NotificationUrgency::Low,
        RouteUrgency::Normal => messenger::NotificationUrgency::Normal,
        RouteUrgency::Critical => messenger::NotificationUrgency::Critical,
    }
}

fn icon_string_to_messenger(value: String) -> messenger::NotificationIcon {
    if value.contains(std::path::MAIN_SEPARATOR) || value.starts_with('.') {
        messenger::NotificationIcon::Path(PathBuf::from(value))
    } else {
        messenger::NotificationIcon::Named(value)
    }
}

fn emit_compatibility_warnings(warnings: &[messenger::CompatibilityWarning]) {
    for warning in warnings {
        if warning.provider == messenger::ProviderKind::Desktop
            && warning.feature == "markdown rendering"
        {
            let status = Status::from_prose(
                "the <b>Desktop</b> platform will drop any Markdown formatting provided",
            )
            .state(StatusState::Info);
            eprintln!("{}", status.render_optimistic(Some(80)));
        } else {
            eprintln!("{warning}");
        }
    }
}

struct ReplaceArgs {
    receipt: String,
    message: Option<String>,
    title: Option<String>,
    plain: bool,
    subtitle: Option<String>,
    icon: Option<String>,
    category: Option<String>,
    urgency: Option<RouteUrgency>,
    timeout_ms: Option<u32>,
    progress_current: Option<u32>,
    progress_total: Option<u32>,
    badge_count: Option<u32>,
    image: Option<PathBuf>,
}

#[tracing::instrument(skip_all)]
async fn replace_notification(args: ReplaceArgs) -> Result<()> {
    let message_text = args.message.as_deref().map(unescape);

    let receipt = receipt_store::load_receipt(&args.receipt)?;
    if receipt.provider != messenger::ProviderKind::Desktop {
        return Err(eyre!(
            "replace only supports Desktop receipts; got {}",
            receipt.provider
        ));
    }

    let config = Config::load()?;
    let route_name = config
        .routes
        .iter()
        .find(|(_, route)| route.provider() == RouteProvider::Desktop)
        .map(|(name, _)| name.clone());

    let route = route_name
        .as_ref()
        .and_then(|name| config.routes.get(name).cloned())
        .unwrap_or_else(RouteConfig::desktop_default);

    let mut message = match message_text.as_deref() {
        Some(text) if !text.is_empty() => {
            if args.plain {
                messenger::Message::text(text)
            } else {
                messenger::Message::markdown(text)
            }
        }
        _ => messenger::Message {
            title: None,
            body: None,
            attachments: Vec::new(),
            location: None,
            metadata: std::collections::BTreeMap::new(),
        },
    };

    if let Some(t) = args.title {
        message = message.title(t);
    }
    if let Some(image_path) = args.image {
        message = message.image(image_path);
    }

    let dispatch = messenger::Dispatch::to(messenger::Target::desktop());
    let overrides = build_desktop_overrides(DesktopOverrideInputs {
        subtitle: args.subtitle,
        icon: args.icon,
        category: args.category,
        urgency: args.urgency,
        timeout_ms: args.timeout_ms,
        replace_id: None,
        group_id: None,
        progress_current: args.progress_current,
        progress_total: args.progress_total,
        badge_count: args.badge_count,
        action: Vec::new(),
    });
    let dispatch = if overrides_is_empty(&overrides) {
        dispatch
    } else {
        dispatch.with_overrides(messenger::ProviderOverrides::Desktop(overrides))
    };

    let prepared = messenger::PreparedMessage::new(&message);

    // Build a standalone DesktopNotificationProvider from route config.
    let desktop_provider = build_desktop_provider_from_route(&route)?;

    let new_receipt = desktop_provider
        .replace(&receipt, &dispatch, &prepared)
        .await?;
    let receipt_path = receipt_store::save_receipt(&new_receipt, route_name.as_deref())?;
    tracing::info!(
        provider = %new_receipt.provider,
        raw_id = %new_receipt.raw_id,
        receipt_path = %receipt_path.display(),
        "CLI replace complete"
    );
    eprintln!(
        "Replaced via {} (id: {})\nReceipt: {}",
        new_receipt.provider,
        new_receipt.raw_id,
        receipt_path.display()
    );

    Ok(())
}

async fn dismiss_notification(receipt_spec: String) -> Result<()> {
    let receipt = receipt_store::load_receipt(&receipt_spec)?;
    if receipt.provider != messenger::ProviderKind::Desktop {
        return Err(eyre!(
            "dismiss only supports Desktop receipts; got {}",
            receipt.provider
        ));
    }

    let config = Config::load()?;
    let route = config
        .routes
        .iter()
        .find(|(_, route)| route.provider() == RouteProvider::Desktop)
        .map(|(_, route)| route.clone())
        .unwrap_or_else(RouteConfig::desktop_default);

    let desktop_provider = build_desktop_provider_from_route(&route)?;

    desktop_provider.dismiss(&receipt).await?;
    tracing::info!(raw_id = %receipt.raw_id, "CLI dismiss complete");
    eprintln!("Dismissed notification {}", receipt.raw_id);

    Ok(())
}

fn build_desktop_provider_from_route(
    route: &RouteConfig,
) -> Result<messenger::DesktopNotificationProvider> {
    match route {
        RouteConfig::Desktop { .. } => {
            let config = build_desktop_config_from_route(route)?;
            Ok(messenger::DesktopNotificationProvider::new(config))
        }
        _ => Err(eyre!("expected Desktop route")),
    }
}

/// Convert a CLI `RouteConfig::Desktop` into the library's `DesktopConfig`.
///
/// Centralises the field mapping (including helper preference resolution
/// from the per-OS config plus the `MESSENGER_DESKTOP_PREFER_HELPERS`
/// env override) so both `register_provider` and the standalone provider
/// builder agree on the resolved configuration.
fn build_desktop_config_from_route(route: &RouteConfig) -> Result<messenger::DesktopConfig> {
    match route {
        RouteConfig::Desktop {
            app_name,
            default_title,
            icon,
            category,
            urgency,
            timeout_ms,
            actions,
            progress,
            badge_count,
            windows,
            macos,
            linux,
        } => Ok(messenger::DesktopConfig {
            app_name: app_name.clone(),
            default_title: default_title.clone(),
            category: category.clone(),
            urgency: route_urgency_to_messenger(*urgency),
            timeout_ms: *timeout_ms,
            icon: icon.clone().map(icon_string_to_messenger),
            actions: actions
                .iter()
                .map(|a| messenger::NotificationAction {
                    id: a.id.clone(),
                    label: a.label.clone(),
                })
                .collect(),
            progress: progress.map(|p| messenger::NotificationProgress {
                current: p.current,
                total: p.total,
            }),
            badge_count: *badge_count,
            windows: messenger::WindowsDesktopConfig {
                app_id: windows.app_id.clone(),
                prefer_helpers: config::resolve_prefer_helpers(&windows.prefer_helpers),
            },
            macos: messenger::MacOsDesktopConfig {
                bundle_id: macos.bundle_id.clone(),
                strategy: match macos.strategy {
                    config::RouteMacOsStrategy::Auto => messenger::MacOsNotificationStrategy::Auto,
                    config::RouteMacOsStrategy::NativeUserNotifications => {
                        messenger::MacOsNotificationStrategy::NativeUserNotifications
                    }
                    config::RouteMacOsStrategy::AppleScript => {
                        messenger::MacOsNotificationStrategy::AppleScript
                    }
                },
                prefer_helpers: config::resolve_prefer_helpers(&macos.prefer_helpers),
            },
            linux: messenger::LinuxDesktopConfig {
                desktop_entry: linux.desktop_entry.clone(),
                prefer_helpers: config::resolve_prefer_helpers(&linux.prefer_helpers),
            },
        }),
        _ => Err(eyre!("expected Desktop route")),
    }
}

fn resolve_route(
    provider_opt: Option<RouteProvider>,
    channel_opt: Option<String>,
    route_opt: Option<String>,
    config: &Config,
) -> Result<ResolvedRoute> {
    if let Some(provider) = provider_opt {
        if provider.requires_target() {
            let channel = channel_opt
                .as_deref()
                .ok_or_else(|| eyre!("--channel is required when using --provider {provider}"))?;
            tracing::debug!(provider = %provider, channel = %channel, "using ad-hoc route");
            return Ok(ResolvedRoute {
                name: None,
                route: RouteConfig::from_provider_and_target(provider, channel),
            });
        }
        if channel_opt.is_some() {
            return Err(eyre!(
                "--channel is not supported for provider {provider}; desktop notifications target the local host"
            ));
        }
        tracing::debug!(provider = %provider, "using ad-hoc targetless route");
        return Ok(ResolvedRoute {
            name: None,
            route: RouteConfig::from_provider_and_target(provider, String::new()),
        });
    }

    if let Some(route_name) = route_opt {
        let route = config
            .routes
            .get(&route_name)
            .cloned()
            .ok_or_else(|| eyre!("route '{route_name}' not found in config"))?;
        tracing::debug!(route = %route_name, "using named route");
        return Ok(ResolvedRoute {
            name: Some(route_name),
            route,
        });
    }

    if let Some(default_name) = &config.default_route {
        let route = config
            .routes
            .get(default_name)
            .cloned()
            .ok_or_else(|| eyre!("default route '{default_name}' not found in config"))?;
        tracing::debug!(route = %default_name, "using default route");
        return Ok(ResolvedRoute {
            name: Some(default_name.clone()),
            route,
        });
    }

    Err(eyre!(
        "no route specified. Use --provider/--channel, --route, or set default_route in ~/.messenger.json"
    ))
}

fn register_provider(messenger: &mut messenger::Messenger, route: &RouteConfig) -> Result<()> {
    tracing::debug!(provider = %route.provider(), "registering provider from CLI route");
    match route {
        RouteConfig::Discord {
            bot_token,
            bot_token_env,
            ..
        } => {
            let token = resolve_secret(bot_token.as_deref(), bot_token_env)?;
            messenger.register(Box::new(
                messenger::provider::discord::DiscordProvider::new(
                    messenger::provider::discord::DiscordConfig {
                        bot_token: SecretString::from(token),
                    },
                ),
            ));
        }
        RouteConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            let url = resolve_secret(webhook_url.as_deref(), webhook_url_env)?;
            let provider = messenger::provider::discord_webhook::DiscordWebhookProvider::try_new(
                messenger::provider::discord_webhook::DiscordWebhookConfig {
                    webhook_url: SecretString::from(url),
                },
            )?;
            messenger.register(Box::new(provider));
        }
        RouteConfig::Slack {
            bot_token,
            bot_token_env,
            ..
        } => {
            let token = resolve_secret(bot_token.as_deref(), bot_token_env)?;
            messenger.register(Box::new(messenger::provider::slack::SlackProvider::new(
                messenger::provider::slack::SlackConfig {
                    bot_token: SecretString::from(token),
                    api_base_url: None,
                },
            )));
        }
        RouteConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            let url = resolve_secret(webhook_url.as_deref(), webhook_url_env)?;
            let provider = messenger::provider::slack_webhook::SlackWebhookProvider::try_new(
                messenger::provider::slack_webhook::SlackWebhookConfig {
                    webhook_url: SecretString::from(url),
                },
            )?;
            messenger.register(Box::new(provider));
        }
        RouteConfig::Signal {
            rpc_url,
            rpc_url_env,
            account,
            account_env,
            ..
        } => {
            let rpc_url = resolve_secret(rpc_url.as_deref(), rpc_url_env)?;
            let account = resolve_secret(account.as_deref(), account_env)?;
            messenger.register(Box::new(messenger::provider::signal::SignalProvider::new(
                messenger::provider::signal::SignalConfig { rpc_url, account },
            )));
        }
        RouteConfig::WhatsApp {
            access_token,
            access_token_env,
            phone_number_id,
            phone_number_id_env,
            ..
        } => {
            let token = resolve_secret(access_token.as_deref(), access_token_env)?;
            let phone_id = resolve_secret(phone_number_id.as_deref(), phone_number_id_env)?;
            messenger.register(Box::new(
                messenger::provider::whatsapp::WhatsAppProvider::new(
                    messenger::provider::whatsapp::WhatsAppConfig {
                        access_token: SecretString::from(token),
                        phone_number_id: phone_id,
                        api_version: None,
                        api_base_url: None,
                    },
                ),
            ));
        }
        RouteConfig::Telegram {
            bot_token,
            bot_token_env,
            ..
        } => {
            let token = resolve_secret(bot_token.as_deref(), bot_token_env)?;
            messenger.register(Box::new(
                messenger::provider::telegram::TelegramProvider::new(
                    messenger::provider::telegram::TelegramConfig {
                        bot_token: SecretString::from(token),
                        api_base_url: None,
                    },
                ),
            ));
        }
        RouteConfig::Desktop { .. } => {
            let config = build_desktop_config_from_route(route)?;
            messenger.register(Box::new(messenger::DesktopNotificationProvider::new(
                config,
            )));
        }
    }
    Ok(())
}

fn build_target(route: &RouteConfig) -> Result<messenger::Target> {
    tracing::debug!(provider = %route.provider(), "building provider target");
    match route {
        RouteConfig::Discord { channel_id, .. } => {
            Ok(messenger::Target::discord_channel(channel_id))
        }
        // TODO: Support thread_id for Discord webhook routes when the CLI
        // introduces a thread-routing flag (e.g. `--thread <id>`).
        RouteConfig::DiscordWebhook { .. } => Ok(messenger::Target::discord_webhook()),
        RouteConfig::Slack { channel_id, .. } => Ok(messenger::Target::slack_channel(channel_id)),
        RouteConfig::SlackWebhook { .. } => Ok(messenger::Target::slack_webhook()),
        RouteConfig::Signal { recipient, .. } => {
            if recipient.starts_with('+') {
                Ok(messenger::Target::signal_user(
                    messenger::target::SignalAddress::Phone(recipient.clone()),
                ))
            } else {
                Ok(messenger::Target::signal_group(recipient))
            }
        }
        RouteConfig::WhatsApp { recipient, .. } => {
            Ok(messenger::Target::whatsapp_recipient(recipient))
        }
        RouteConfig::Telegram { chat_id, .. } => {
            let chat_id = if let Ok(id) = chat_id.parse::<i64>() {
                messenger::target::TelegramChatId::Id(id)
            } else {
                messenger::target::TelegramChatId::Username(chat_id.clone())
            };
            Ok(messenger::Target::telegram_chat(chat_id))
        }
        RouteConfig::Desktop { .. } => Ok(messenger::Target::desktop()),
    }
}

/// Resolve a secret: use the direct value if present and non-empty, otherwise
/// look up the env var.
///
/// Empty and whitespace-only direct values fall through to the env var lookup
/// so that a config entry like `"webhook_url": ""` does not shadow a valid env
/// var. This matches the secret-resolution precedence described in the
/// `messenger/features/2026-04-19-slack-webhook/spec.md` CLI requirements.
fn resolve_secret(value: Option<&str>, env_name: &str) -> Result<String> {
    if let Some(v) = value
        && !v.trim().is_empty()
    {
        tracing::trace!("using direct config value");
        return Ok(v.to_string());
    }
    tracing::trace!(env = %env_name, "resolving secret from environment");
    std::env::var(env_name).map_err(|_| {
        eyre!(
            "no value configured and environment variable {env_name} is not set. \
             Either store the value in your route config or set {env_name}."
        )
    })
}

/// Replace common backslash escape sequences with their actual characters.
///
/// Supports `\n`, `\t`, `\r`, and `\\`. All other backslash-letter
/// combinations are left untouched.
fn unescape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Parse a "LAT,LON" string into (f64, f64).
fn parse_location(s: &str) -> Result<(f64, f64)> {
    let parts: Vec<&str> = s.splitn(2, ',').collect();
    if parts.len() != 2 {
        return Err(eyre!(
            "invalid location format: expected \"LAT,LON\" (e.g. \"34.05,-118.24\")"
        ));
    }
    let lat: f64 = parts[0]
        .trim()
        .parse()
        .map_err(|_| eyre!("invalid latitude: \"{}\"", parts[0].trim()))?;
    let lon: f64 = parts[1]
        .trim()
        .parse()
        .map_err(|_| eyre!("invalid longitude: \"{}\"", parts[1].trim()))?;
    Ok((lat, lon))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn unescape_converts_backslash_n_to_newline() {
        assert_eq!(unescape("hello\\nworld"), "hello\nworld");
    }

    #[test]
    fn unescape_converts_backslash_t_to_tab() {
        assert_eq!(unescape("col1\\tcol2"), "col1\tcol2");
    }

    #[test]
    fn unescape_converts_backslash_r_to_cr() {
        assert_eq!(unescape("line1\\rline2"), "line1\rline2");
    }

    #[test]
    fn unescape_converts_double_backslash_to_single() {
        assert_eq!(unescape("path\\\\to\\\\file"), "path\\to\\file");
    }

    #[test]
    fn unescape_leaves_unknown_escapes_intact() {
        assert_eq!(unescape("hello\\xworld"), "hello\\xworld");
    }

    #[test]
    fn unescape_leaves_trailing_backslash_intact() {
        assert_eq!(unescape("ends with \\"), "ends with \\");
    }

    #[test]
    fn unescape_handles_empty_string() {
        assert_eq!(unescape(""), "");
    }

    #[test]
    fn unescape_handles_no_escapes() {
        assert_eq!(unescape("plain text"), "plain text");
    }

    #[test]
    fn unescape_handles_mixed_escapes() {
        assert_eq!(
            unescape("line1\\nline2\\tcol1\\tcol2\\\\end"),
            "line1\nline2\tcol1\tcol2\\end"
        );
    }

    #[test]
    fn resolve_route_builds_ad_hoc_provider_route() {
        let resolved = resolve_route(
            Some(RouteProvider::Slack),
            Some("C012345".into()),
            None,
            &Config::default(),
        )
        .unwrap();

        assert_eq!(resolved.name, None);
        assert_eq!(
            resolved.route,
            RouteConfig::Slack {
                channel_id: "C012345".into(),
                bot_token: None,
                bot_token_env: "SLACK_BOT_TOKEN".into(),
            }
        );
    }

    #[test]
    fn resolve_route_prefers_named_route() {
        let mut config = Config {
            default_route: Some("slack.default".into()),
            routes: HashMap::new(),
        };
        config.routes.insert(
            "slack.ops".into(),
            RouteConfig::Slack {
                channel_id: "C012345".into(),
                bot_token: None,
                bot_token_env: "SLACK_BOT_TOKEN".into(),
            },
        );

        let resolved = resolve_route(None, None, Some("slack.ops".into()), &config).unwrap();
        assert_eq!(resolved.name.as_deref(), Some("slack.ops"));
    }

    #[test]
    fn build_target_maps_signal_phone_to_direct_target() {
        let target = build_target(&RouteConfig::Signal {
            recipient: "+15551234567".into(),
            rpc_url: None,
            rpc_url_env: "SIGNAL_RPC_URL".into(),
            account: None,
            account_env: "SIGNAL_ACCOUNT".into(),
        })
        .unwrap();

        assert!(matches!(
            target,
            messenger::Target::Signal(messenger::target::SignalTarget::User(_))
        ));
    }

    #[test]
    fn build_target_maps_telegram_username_to_username_target() {
        let target = build_target(&RouteConfig::Telegram {
            chat_id: "@ops".into(),
            bot_token: None,
            bot_token_env: "TELEGRAM_BOT_TOKEN".into(),
        })
        .unwrap();

        assert!(matches!(
            target,
            messenger::Target::Telegram(messenger::target::TelegramTarget {
                chat_id: messenger::target::TelegramChatId::Username(_),
                ..
            })
        ));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_secret_prefers_direct_value_over_env() {
        unsafe {
            std::env::remove_var("DISCORD_WEBHOOK_URL");
        }
        let resolved = resolve_secret(
            Some("https://discord.com/api/v10/webhooks/1/direct"),
            "DISCORD_WEBHOOK_URL",
        )
        .unwrap();
        assert_eq!(resolved, "https://discord.com/api/v10/webhooks/1/direct");
    }

    #[test]
    #[serial_test::serial]
    fn resolve_secret_falls_back_to_env_var() {
        unsafe {
            std::env::set_var(
                "DISCORD_WEBHOOK_URL",
                "https://discord.com/api/v10/webhooks/1/from-env",
            );
        }
        let resolved = resolve_secret(None, "DISCORD_WEBHOOK_URL").unwrap();
        assert_eq!(resolved, "https://discord.com/api/v10/webhooks/1/from-env");
        unsafe {
            std::env::remove_var("DISCORD_WEBHOOK_URL");
        }
    }

    #[test]
    #[serial_test::serial]
    fn resolve_secret_errors_when_neither_value_nor_env_is_set() {
        unsafe {
            std::env::remove_var("DISCORD_WEBHOOK_URL");
        }
        let err = resolve_secret(None, "DISCORD_WEBHOOK_URL").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("DISCORD_WEBHOOK_URL"),
            "error should mention the missing env var, got: {msg}"
        );
    }

    #[test]
    fn build_target_maps_discord_webhook_route_to_webhook_target() {
        let target = build_target(&RouteConfig::DiscordWebhook {
            webhook_url: Some("https://discord.com/api/v10/webhooks/1/abc".into()),
            webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
        })
        .unwrap();

        assert!(matches!(target, messenger::Target::DiscordWebhook(_)));
    }

    #[test]
    fn resolve_route_builds_discord_webhook_ad_hoc_route() {
        let resolved = resolve_route(
            Some(RouteProvider::DiscordWebhook),
            Some("https://discord.com/api/v10/webhooks/1/abc".into()),
            None,
            &Config::default(),
        )
        .unwrap();

        assert_eq!(resolved.name, None);
        assert_eq!(
            resolved.route,
            RouteConfig::DiscordWebhook {
                webhook_url: Some("https://discord.com/api/v10/webhooks/1/abc".into()),
                webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
            }
        );
    }

    // Setup-flow smoke test: the interactive `inquire` prompt chain cannot run
    // unattended, so we verify the non-interactive postcondition — a pre-built
    // RouteConfig::DiscordWebhook is preserved through a save/load cycle and
    // reports the correct provider kind. The interactive walkthrough is
    // covered by the setup section of the user guide.
    #[test]
    fn discord_webhook_route_survives_config_save_load_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("messenger.json");

        let route = RouteConfig::DiscordWebhook {
            webhook_url: Some("https://discord.com/api/v10/webhooks/1/abc".into()),
            webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
        };

        let mut config = Config {
            default_route: Some("discord.webhook.alerts".into()),
            routes: HashMap::new(),
        };
        config
            .routes
            .insert("discord.webhook.alerts".into(), route.clone());

        config.save_to_path(&path).unwrap();
        let loaded = Config::load_from_path(&path).unwrap();

        assert_eq!(loaded, config);
        let stored = loaded.routes.get("discord.webhook.alerts").unwrap();
        assert_eq!(stored, &route);
        assert_eq!(stored.provider(), RouteProvider::DiscordWebhook);
    }

    #[test]
    fn register_provider_returns_error_for_malformed_discord_webhook_url() {
        let route = RouteConfig::DiscordWebhook {
            webhook_url: Some("not a url".into()),
            webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
        };

        let mut messenger = messenger::Messenger::new();
        let err = register_provider(&mut messenger, &route).unwrap_err();

        let msg = format!("{err}");
        assert!(
            msg.contains("Discord webhook URL"),
            "error should surface webhook URL validation, got: {msg}"
        );
    }

    #[test]
    fn build_target_maps_slack_webhook_route_to_webhook_target() {
        let target = build_target(&RouteConfig::SlackWebhook {
            webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXXX".into()),
            webhook_url_env: "SLACK_WEBHOOK_URL".into(),
        })
        .unwrap();

        assert!(matches!(target, messenger::Target::SlackWebhook(_)));
    }

    #[test]
    fn resolve_route_builds_slack_webhook_ad_hoc_route() {
        let resolved = resolve_route(
            Some(RouteProvider::SlackWebhook),
            Some("https://hooks.slack.com/services/T000/B000/XXXXX".into()),
            None,
            &Config::default(),
        )
        .unwrap();

        assert_eq!(resolved.name, None);
        assert_eq!(
            resolved.route,
            RouteConfig::SlackWebhook {
                webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXXX".into()),
                webhook_url_env: "SLACK_WEBHOOK_URL".into(),
            }
        );
    }

    // Setup-flow smoke test: the interactive `inquire` prompt chain cannot run
    // unattended, so we verify the non-interactive postcondition — a pre-built
    // RouteConfig::SlackWebhook is preserved through a save/load cycle and
    // reports the correct provider kind.
    #[test]
    fn slack_webhook_route_survives_config_save_load_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("messenger.json");

        let route = RouteConfig::SlackWebhook {
            webhook_url: Some("https://hooks.slack.com/services/T000/B000/XXXXX".into()),
            webhook_url_env: "SLACK_WEBHOOK_URL".into(),
        };

        let mut config = Config {
            default_route: Some("slack.webhook.alerts".into()),
            routes: HashMap::new(),
        };
        config
            .routes
            .insert("slack.webhook.alerts".into(), route.clone());

        config.save_to_path(&path).unwrap();
        let loaded = Config::load_from_path(&path).unwrap();

        assert_eq!(loaded, config);
        let stored = loaded.routes.get("slack.webhook.alerts").unwrap();
        assert_eq!(stored, &route);
        assert_eq!(stored.provider(), RouteProvider::SlackWebhook);
    }

    #[test]
    fn register_provider_returns_error_for_malformed_slack_webhook_url() {
        let route = RouteConfig::SlackWebhook {
            webhook_url: Some("not a url".into()),
            webhook_url_env: "SLACK_WEBHOOK_URL".into(),
        };

        let mut messenger = messenger::Messenger::new();
        let err = register_provider(&mut messenger, &route).unwrap_err();

        let msg = format!("{err}");
        assert!(
            msg.contains("Slack webhook"),
            "error should surface webhook URL validation, got: {msg}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn resolve_secret_empty_direct_value_falls_back_to_env() {
        unsafe {
            std::env::set_var(
                "SLACK_WEBHOOK_URL",
                "https://hooks.slack.com/services/T000/B000/FROM-ENV",
            );
        }
        let resolved = resolve_secret(Some(""), "SLACK_WEBHOOK_URL").unwrap();
        assert_eq!(
            resolved,
            "https://hooks.slack.com/services/T000/B000/FROM-ENV"
        );
        unsafe {
            std::env::remove_var("SLACK_WEBHOOK_URL");
        }
    }

    #[test]
    #[serial_test::serial]
    fn resolve_secret_whitespace_only_direct_value_falls_back_to_env() {
        unsafe {
            std::env::set_var(
                "SLACK_WEBHOOK_URL",
                "https://hooks.slack.com/services/T000/B000/FROM-ENV",
            );
        }
        let resolved = resolve_secret(Some("   "), "SLACK_WEBHOOK_URL").unwrap();
        assert_eq!(
            resolved,
            "https://hooks.slack.com/services/T000/B000/FROM-ENV"
        );
        unsafe {
            std::env::remove_var("SLACK_WEBHOOK_URL");
        }
    }

    #[test]
    #[serial_test::serial]
    fn resolve_secret_falls_back_to_default_env_name_for_slack_webhook() {
        // Deserializing `{"provider":"slack-webhook"}` with both fields absent
        // populates `webhook_url_env` via `default_slack_webhook_url_env()`,
        // which must be `"SLACK_WEBHOOK_URL"` so env-only routes work out of
        // the box.
        unsafe {
            std::env::set_var(
                "SLACK_WEBHOOK_URL",
                "https://hooks.slack.com/services/T000/B000/DEFAULT-ENV",
            );
        }
        let parsed: RouteConfig = serde_json::from_str(r#"{"provider":"slack-webhook"}"#).unwrap();
        let env_name = match parsed {
            RouteConfig::SlackWebhook {
                webhook_url,
                webhook_url_env,
            } => {
                assert!(webhook_url.is_none());
                webhook_url_env
            }
            other => panic!("unexpected route: {other:?}"),
        };
        assert_eq!(env_name, "SLACK_WEBHOOK_URL");
        let resolved = resolve_secret(None, &env_name).unwrap();
        assert_eq!(
            resolved,
            "https://hooks.slack.com/services/T000/B000/DEFAULT-ENV"
        );
        unsafe {
            std::env::remove_var("SLACK_WEBHOOK_URL");
        }
    }

    #[test]
    #[serial_test::serial]
    fn resolve_secret_errors_when_neither_direct_value_nor_env_for_slack_webhook() {
        unsafe {
            std::env::remove_var("SLACK_WEBHOOK_URL");
        }
        let err = resolve_secret(None, "SLACK_WEBHOOK_URL").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("SLACK_WEBHOOK_URL"),
            "error should mention the missing env var, got: {msg}"
        );
    }

    #[test]
    fn compatibility_warning_format_matches_cli_output() {
        let warning = messenger::CompatibilityWarning {
            provider: messenger::ProviderKind::Slack,
            feature: "attachments",
        };

        assert_eq!(
            warning.to_string(),
            "⚠️ the attachments feature is not supported on Slack and will be dropped"
        );
    }

    #[test]
    fn desktop_markdown_warning_renders_as_info_status() {
        let _warning = messenger::CompatibilityWarning {
            provider: messenger::ProviderKind::Desktop,
            feature: "markdown rendering",
        };

        let status = Status::from_prose(
            "the <b>Desktop</b> platform will drop any Markdown formatting provided",
        )
        .state(StatusState::Info);
        let rendered = status.render_optimistic(Some(80));

        assert!(
            rendered.contains("Desktop"),
            "rendered output should contain 'Desktop': {rendered}"
        );
        assert!(
            rendered.contains("will drop any Markdown formatting provided"),
            "rendered output should contain message: {rendered}"
        );
    }

    #[test]
    fn resolve_route_builds_desktop_ad_hoc_route_without_channel() {
        let resolved =
            resolve_route(Some(RouteProvider::Desktop), None, None, &Config::default()).unwrap();

        assert_eq!(resolved.name, None);
        assert_eq!(resolved.route.provider(), RouteProvider::Desktop);
        assert!(matches!(resolved.route, RouteConfig::Desktop { .. }));
    }

    #[test]
    fn resolve_route_rejects_channel_for_desktop_provider() {
        let err = resolve_route(
            Some(RouteProvider::Desktop),
            Some("ignored".into()),
            None,
            &Config::default(),
        )
        .unwrap_err();

        let msg = format!("{err}");
        assert!(
            msg.contains("--channel is not supported"),
            "expected --channel rejection for desktop, got: {msg}"
        );
    }

    #[test]
    fn resolve_route_still_requires_channel_for_chat_provider() {
        let err =
            resolve_route(Some(RouteProvider::Slack), None, None, &Config::default()).unwrap_err();

        let msg = format!("{err}");
        assert!(
            msg.contains("--channel is required"),
            "expected --channel required for Slack, got: {msg}"
        );
    }

    #[test]
    fn build_target_maps_desktop_route_to_desktop_target() {
        let target = build_target(&RouteConfig::desktop_default()).unwrap();
        assert!(matches!(target, messenger::Target::Desktop(_)));
    }

    #[test]
    fn desktop_route_survives_config_save_load_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("messenger.json");

        let route = RouteConfig::Desktop {
            app_name: "Messenger".into(),
            default_title: Some("Build Status".into()),
            icon: Some("dialog-information".into()),
            category: Some("im.received".into()),
            urgency: config::RouteUrgency::Critical,
            timeout_ms: Some(5000),
            actions: Vec::new(),
            progress: None,
            badge_count: None,
            windows: config::DesktopWindowsConfig {
                app_id: Some("RustyBiscuit.Messenger".into()),
                prefer_helpers: Vec::new(),
            },
            macos: config::DesktopMacOsConfig {
                bundle_id: Some("com.rustybiscuit.messenger".into()),
                strategy: config::RouteMacOsStrategy::NativeUserNotifications,
                prefer_helpers: Vec::new(),
            },
            linux: config::DesktopLinuxConfig {
                desktop_entry: Some("messenger".into()),
                prefer_helpers: Vec::new(),
            },
        };

        let mut config = Config {
            default_route: Some("desktop.local".into()),
            routes: HashMap::new(),
        };
        config.routes.insert("desktop.local".into(), route.clone());

        config.save_to_path(&path).unwrap();
        let loaded = Config::load_from_path(&path).unwrap();

        assert_eq!(loaded, config);
        let stored = loaded.routes.get("desktop.local").unwrap();
        assert_eq!(stored, &route);
        assert_eq!(stored.provider(), RouteProvider::Desktop);
    }

    #[test]
    fn desktop_route_parses_spec_example_config() {
        let raw = r#"{
            "provider": "desktop",
            "app_name": "Messenger",
            "default_title": "Messenger",
            "icon": "dialog-information",
            "category": "im.received",
            "urgency": "normal",
            "timeout_ms": 5000,
            "windows": { "app_id": "RustyBiscuit.Messenger" },
            "macos": { "bundle_id": "com.rustybiscuit.messenger", "strategy": "auto" },
            "linux": { "desktop_entry": "messenger" }
        }"#;

        let parsed: RouteConfig = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.provider(), RouteProvider::Desktop);
        match parsed {
            RouteConfig::Desktop {
                app_name,
                default_title,
                icon,
                category,
                urgency,
                timeout_ms,
                actions,
                progress,
                badge_count,
                windows,
                macos,
                linux,
            } => {
                assert_eq!(app_name, "Messenger");
                assert_eq!(default_title.as_deref(), Some("Messenger"));
                assert_eq!(icon.as_deref(), Some("dialog-information"));
                assert_eq!(category.as_deref(), Some("im.received"));
                assert_eq!(urgency, config::RouteUrgency::Normal);
                assert_eq!(timeout_ms, Some(5000));
                assert!(actions.is_empty());
                assert!(progress.is_none());
                assert!(badge_count.is_none());
                assert_eq!(windows.app_id.as_deref(), Some("RustyBiscuit.Messenger"));
                assert_eq!(
                    macos.bundle_id.as_deref(),
                    Some("com.rustybiscuit.messenger")
                );
                assert_eq!(macos.strategy, config::RouteMacOsStrategy::Auto);
                assert_eq!(linux.desktop_entry.as_deref(), Some("messenger"));
            }
            other => panic!("expected RouteConfig::Desktop, got: {other:?}"),
        }
    }

    #[test]
    fn desktop_route_applies_defaults_when_minimal() {
        let raw = r#"{ "provider": "desktop" }"#;
        let parsed: RouteConfig = serde_json::from_str(raw).unwrap();
        match parsed {
            RouteConfig::Desktop {
                app_name,
                default_title,
                icon,
                category,
                urgency,
                timeout_ms,
                actions,
                progress,
                badge_count,
                windows,
                macos,
                linux,
            } => {
                assert_eq!(app_name, "Messenger");
                assert!(default_title.is_none());
                assert!(icon.is_none());
                assert!(category.is_none());
                assert_eq!(urgency, config::RouteUrgency::Normal);
                assert!(timeout_ms.is_none());
                assert!(actions.is_empty());
                assert!(progress.is_none());
                assert!(badge_count.is_none());
                assert!(windows.app_id.is_none());
                assert!(macos.bundle_id.is_none());
                assert_eq!(macos.strategy, config::RouteMacOsStrategy::Auto);
                assert!(linux.desktop_entry.is_none());
            }
            other => panic!("expected RouteConfig::Desktop, got: {other:?}"),
        }
    }

    #[test]
    fn requires_target_reports_desktop_as_targetless() {
        assert!(!RouteProvider::Desktop.requires_target());
        for other in RouteProvider::ALL
            .iter()
            .filter(|p| **p != RouteProvider::Desktop)
        {
            assert!(
                other.requires_target(),
                "chat provider {other} must still require a target"
            );
        }
    }

    #[test]
    fn icon_string_routes_absolute_path_to_path_variant() {
        let icon = icon_string_to_messenger(format!(
            "{}tmp{}icon.png",
            std::path::MAIN_SEPARATOR,
            std::path::MAIN_SEPARATOR
        ));
        assert!(matches!(icon, messenger::NotificationIcon::Path(_)));
    }

    #[test]
    fn icon_string_routes_name_to_named_variant() {
        let icon = icon_string_to_messenger("dialog-information".into());
        assert_eq!(
            icon,
            messenger::NotificationIcon::Named("dialog-information".into())
        );
    }

    #[test]
    fn send_cli_parses_desktop_flags() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "messenger",
            "send",
            "--provider",
            "desktop",
            "--title",
            "Build",
            "--urgency",
            "critical",
            "--timeout-ms",
            "3500",
            "Green across the board",
        ])
        .unwrap();

        match cli.command {
            Commands::Send {
                message,
                provider,
                channel,
                title,
                urgency,
                timeout_ms,
                ..
            } => {
                assert_eq!(message.as_deref(), Some("Green across the board"));
                assert_eq!(provider, Some(RouteProvider::Desktop));
                assert_eq!(channel, None);
                assert_eq!(title.as_deref(), Some("Build"));
                assert_eq!(urgency, Some(config::RouteUrgency::Critical));
                assert_eq!(timeout_ms, Some(3500));
            }
            _ => panic!("expected Send subcommand"),
        }
    }

    #[test]
    fn send_cli_allows_desktop_send_without_body() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "messenger",
            "send",
            "--provider",
            "desktop",
            "--title",
            "Alert",
        ])
        .unwrap();

        match cli.command {
            Commands::Send { message, title, .. } => {
                assert!(message.is_none());
                assert_eq!(title.as_deref(), Some("Alert"));
            }
            _ => panic!("expected Send subcommand"),
        }
    }

    #[test]
    fn desktop_listed_in_provider_value_enum() {
        use clap::ValueEnum;
        let parsed = RouteProvider::from_str("desktop", true).unwrap();
        assert_eq!(parsed, RouteProvider::Desktop);
    }

    #[test]
    fn send_cli_parses_replace_id_flag() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "messenger",
            "send",
            "--provider",
            "desktop",
            "--replace-id",
            "notif-123",
            "Updated status",
        ])
        .unwrap();

        match cli.command {
            Commands::Send {
                message,
                provider,
                replace_id,
                ..
            } => {
                assert_eq!(message.as_deref(), Some("Updated status"));
                assert_eq!(provider, Some(RouteProvider::Desktop));
                assert_eq!(replace_id.as_deref(), Some("notif-123"));
            }
            _ => panic!("expected Send subcommand"),
        }
    }

    #[test]
    fn send_cli_parses_group_id_flag() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "messenger",
            "send",
            "--provider",
            "desktop",
            "--group-id",
            "build-alerts",
            "Build failed",
        ])
        .unwrap();

        match cli.command {
            Commands::Send {
                message,
                provider,
                group_id,
                ..
            } => {
                assert_eq!(message.as_deref(), Some("Build failed"));
                assert_eq!(provider, Some(RouteProvider::Desktop));
                assert_eq!(group_id.as_deref(), Some("build-alerts"));
            }
            _ => panic!("expected Send subcommand"),
        }
    }

    #[test]
    fn send_cli_parses_progress_flags() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "messenger",
            "send",
            "--provider",
            "desktop",
            "--progress-current",
            "42",
            "--progress-total",
            "100",
            "Uploading",
        ])
        .unwrap();

        match cli.command {
            Commands::Send {
                message,
                progress_current,
                progress_total,
                ..
            } => {
                assert_eq!(message.as_deref(), Some("Uploading"));
                assert_eq!(progress_current, Some(42));
                assert_eq!(progress_total, Some(100));
            }
            _ => panic!("expected Send subcommand"),
        }
    }

    #[test]
    fn send_cli_parses_badge_count_flag() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "messenger",
            "send",
            "--provider",
            "desktop",
            "--badge-count",
            "5",
            "New alerts",
        ])
        .unwrap();

        match cli.command {
            Commands::Send {
                message,
                badge_count,
                ..
            } => {
                assert_eq!(message.as_deref(), Some("New alerts"));
                assert_eq!(badge_count, Some(5));
            }
            _ => panic!("expected Send subcommand"),
        }
    }

    #[test]
    fn send_cli_parses_action_flags() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "messenger",
            "send",
            "--provider",
            "desktop",
            "--action",
            "ok:Approve",
            "--action",
            "reject:Reject",
            "Approval needed",
        ])
        .unwrap();

        match cli.command {
            Commands::Send {
                message, action, ..
            } => {
                assert_eq!(message.as_deref(), Some("Approval needed"));
                assert_eq!(action, vec!["ok:Approve", "reject:Reject"]);
            }
            _ => panic!("expected Send subcommand"),
        }
    }

    #[test]
    fn replace_cli_parses_basic_args() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "messenger",
            "replace",
            "/tmp/receipt.json",
            "Updated message",
            "--title",
            "Updated",
            "--badge-count",
            "3",
        ])
        .unwrap();

        match cli.command {
            Commands::Replace {
                receipt,
                message,
                title,
                badge_count,
                ..
            } => {
                assert_eq!(receipt, "/tmp/receipt.json");
                assert_eq!(message.as_deref(), Some("Updated message"));
                assert_eq!(title.as_deref(), Some("Updated"));
                assert_eq!(badge_count, Some(3));
            }
            _ => panic!("expected Replace subcommand"),
        }
    }

    #[test]
    fn dismiss_cli_parses_receipt_arg() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["messenger", "dismiss", "/tmp/receipt.json"]).unwrap();

        match cli.command {
            Commands::Dismiss { receipt } => {
                assert_eq!(receipt, "/tmp/receipt.json");
            }
            _ => panic!("expected Dismiss subcommand"),
        }
    }
}
