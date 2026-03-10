use std::path::PathBuf;

use clap::{Parser, Subcommand};
use color_eyre::eyre::{Result, eyre};
use secrecy::SecretString;

mod config;
mod setup;

use config::{Config, RouteConfig};

pub const VALID_PROVIDERS: &[&str] = &["discord", "slack", "signal", "whatsapp", "telegram"];

#[derive(Parser)]
#[command(name = "messenger", about = "Send messages to any platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a message (default command).
    Send {
        /// The message text to send (Markdown supported).
        message: String,

        /// Provider to use (discord, slack, signal, whatsapp, telegram).
        #[arg(long)]
        provider: Option<String>,

        /// Channel or recipient ID.
        #[arg(long)]
        channel: Option<String>,

        /// Named route from config file.
        #[arg(long)]
        route: Option<String>,

        /// Message reference to reply to (provider-specific format).
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
    },
    /// Interactive provider configuration.
    Setup {
        /// Provider to configure (discord, slack, signal, whatsapp, telegram).
        provider: Option<String>,
    },
    /// Interactive provider configuration (alias for setup).
    Init {
        /// Provider to configure (discord, slack, signal, whatsapp, telegram).
        provider: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Send {
            message,
            provider,
            channel,
            route,
            reply_to: _,
            image,
            file: _,
            silent,
            strict,
            plain,
        } => {
            send_message(
                &message, provider, channel, route, image, silent, strict, plain,
            )
            .await?;
        }
        Commands::Setup { provider } | Commands::Init { provider } => {
            setup::run(provider)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn send_message(
    message_text: &str,
    provider_opt: Option<String>,
    channel_opt: Option<String>,
    route_opt: Option<String>,
    image: Option<PathBuf>,
    silent: bool,
    strict: bool,
    plain: bool,
) -> Result<()> {
    let config = Config::load()?;

    // Resolve which route to use
    let route = resolve_route(provider_opt, channel_opt, route_opt, &config)?;

    // Build the messenger with the appropriate provider
    let mut messenger = messenger::Messenger::new();
    register_provider(&mut messenger, &route)?;

    // Build the message
    let message = if plain {
        messenger::Message::text(message_text)
    } else {
        messenger::Message::markdown(message_text)
    };

    // Add image attachment if provided
    let message = if let Some(image_path) = &image {
        message.image(image_path)
    } else {
        message
    };

    // Build the dispatch
    let target = build_target(&route)?;
    let mut dispatch = messenger::Dispatch::to(target);

    if silent {
        dispatch = dispatch.silent();
    }
    if strict {
        dispatch = dispatch.strict();
    }

    // Send
    let receipt = messenger.send(dispatch, &message).await?;
    eprintln!(
        "Sent via {} (id: {})",
        receipt.provider, receipt.raw_id
    );

    Ok(())
}

fn resolve_route(
    provider_opt: Option<String>,
    channel_opt: Option<String>,
    route_opt: Option<String>,
    config: &Config,
) -> Result<RouteConfig> {
    // Explicit provider + channel flags
    if let Some(provider) = &provider_opt {
        let channel = channel_opt.as_deref().ok_or_else(|| {
            eyre!("--channel is required when using --provider")
        })?;
        return Ok(RouteConfig {
            provider: provider.clone(),
            channel_id: channel.to_string(),
            token_env: default_token_env(provider),
        });
    }

    // Named route from config
    if let Some(route_name) = &route_opt {
        return config
            .routes
            .get(route_name)
            .cloned()
            .ok_or_else(|| eyre!("route '{route_name}' not found in config"));
    }

    // Default route
    if let Some(default_name) = &config.default_route {
        return config
            .routes
            .get(default_name)
            .cloned()
            .ok_or_else(|| eyre!("default route '{default_name}' not found in config"));
    }

    Err(eyre!(
        "no route specified. Use --provider/--channel, --route, or set default_route in ~/.messenger.json"
    ))
}

pub fn default_token_env(provider: &str) -> String {
    match provider {
        "discord" => "DISCORD_BOT_TOKEN".into(),
        "slack" => "SLACK_BOT_TOKEN".into(),
        "signal" => "SIGNAL_RPC_URL".into(),
        "whatsapp" => "WHATSAPP_ACCESS_TOKEN".into(),
        "telegram" => "TELEGRAM_BOT_TOKEN".into(),
        _ => format!("{}_TOKEN", provider.to_uppercase()),
    }
}

fn register_provider(
    messenger: &mut messenger::Messenger,
    route: &RouteConfig,
) -> Result<()> {
    match route.provider.as_str() {
        "discord" => {
            let token = resolve_env(&route.token_env)?;
            messenger.register(Box::new(
                messenger::provider::discord::DiscordProvider::new(
                    messenger::provider::discord::DiscordConfig {
                        bot_token: SecretString::from(token),
                    },
                ),
            ));
        }
        "slack" => {
            let token = resolve_env(&route.token_env)?;
            messenger.register(Box::new(
                messenger::provider::slack::SlackProvider::new(
                    messenger::provider::slack::SlackConfig {
                        bot_token: SecretString::from(token),
                        api_base_url: None,
                    },
                ),
            ));
        }
        "signal" => {
            let rpc_url = resolve_env("SIGNAL_RPC_URL")?;
            let account = resolve_env("SIGNAL_ACCOUNT")?;
            messenger.register(Box::new(
                messenger::provider::signal::SignalProvider::new(
                    messenger::provider::signal::SignalConfig {
                        rpc_url,
                        account,
                    },
                ),
            ));
        }
        "whatsapp" => {
            let token = resolve_env(&route.token_env)?;
            let phone_id = resolve_env("WHATSAPP_PHONE_NUMBER_ID")?;
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
        "telegram" => {
            let token = resolve_env(&route.token_env)?;
            messenger.register(Box::new(
                messenger::provider::telegram::TelegramProvider::new(
                    messenger::provider::telegram::TelegramConfig {
                        bot_token: SecretString::from(token),
                        api_base_url: None,
                    },
                ),
            ));
        }
        other => return Err(eyre!("unknown provider: {other}")),
    }
    Ok(())
}

fn build_target(route: &RouteConfig) -> Result<messenger::Target> {
    match route.provider.as_str() {
        "discord" => Ok(messenger::Target::discord_channel(&route.channel_id)),
        "slack" => Ok(messenger::Target::slack_channel(&route.channel_id)),
        "signal" => {
            if route.channel_id.starts_with('+') {
                Ok(messenger::Target::signal_user(
                    messenger::target::SignalAddress::Phone(route.channel_id.clone()),
                ))
            } else {
                Ok(messenger::Target::signal_group(&route.channel_id))
            }
        }
        "whatsapp" => Ok(messenger::Target::whatsapp_recipient(&route.channel_id)),
        "telegram" => {
            let chat_id = if let Ok(id) = route.channel_id.parse::<i64>() {
                messenger::target::TelegramChatId::Id(id)
            } else {
                messenger::target::TelegramChatId::Username(route.channel_id.clone())
            };
            Ok(messenger::Target::telegram_chat(chat_id))
        }
        other => Err(eyre!("unknown provider: {other}")),
    }
}

fn resolve_env(var: &str) -> Result<String> {
    std::env::var(var).map_err(|_| {
        eyre!(
            "environment variable {var} is not set. Set it before running messenger."
        )
    })
}
