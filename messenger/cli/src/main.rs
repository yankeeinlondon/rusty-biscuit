use std::path::PathBuf;

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

mod config;
mod receipt_store;
mod setup;

use config::{Config, RouteConfig, RouteProvider};

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
enum Commands {
    /// Send a message.
    Send {
        /// The message text to send (Markdown supported unless --plain is used).
        message: String,

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
        } => {
            send_message(
                &message, provider, channel, route, reply_to, image, file, silent, strict, plain,
                location,
            )
            .await?;
        }
        Commands::Setup { provider } | Commands::Init { provider } => {
            setup::run(provider)?;
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

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all, fields(route = tracing::field::Empty, provider = tracing::field::Empty))]
async fn send_message(
    message_text: &str,
    provider_opt: Option<RouteProvider>,
    channel_opt: Option<String>,
    route_opt: Option<String>,
    reply_to: Option<String>,
    image: Option<PathBuf>,
    file: Option<PathBuf>,
    silent: bool,
    strict: bool,
    plain: bool,
    location: Option<String>,
) -> Result<()> {
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
        "building message from CLI arguments"
    );

    let mut messenger = messenger::Messenger::new();
    register_provider(&mut messenger, &resolved_route.route)?;

    let mut message = if plain {
        messenger::Message::text(message_text)
    } else {
        messenger::Message::markdown(message_text)
    };

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

fn emit_compatibility_warnings(warnings: &[messenger::CompatibilityWarning]) {
    for warning in warnings {
        eprintln!("{warning}");
    }
}

fn resolve_route(
    provider_opt: Option<RouteProvider>,
    channel_opt: Option<String>,
    route_opt: Option<String>,
    config: &Config,
) -> Result<ResolvedRoute> {
    if let Some(provider) = provider_opt {
        let channel = channel_opt
            .as_deref()
            .ok_or_else(|| eyre!("--channel is required when using --provider"))?;
        tracing::debug!(provider = %provider, channel = %channel, "using ad-hoc route");
        return Ok(ResolvedRoute {
            name: None,
            route: RouteConfig::from_provider_and_target(provider, channel),
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
            messenger.register(Box::new(
                messenger::provider::discord_webhook::DiscordWebhookProvider::new(
                    messenger::provider::discord_webhook::DiscordWebhookConfig {
                        webhook_url: SecretString::from(url),
                    },
                ),
            ));
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
    }
}

/// Resolve a secret: use the direct value if present, otherwise look up the env var.
fn resolve_secret(value: Option<&str>, env_name: &str) -> Result<String> {
    if let Some(v) = value {
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
}
