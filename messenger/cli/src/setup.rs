use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::prelude::Renderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use inquire::{Confirm, InquireError, Password, PasswordDisplayMode, Select, Text};

use crate::config::{Config, RouteConfig, RouteProvider};

fn styled(text: impl Into<String>) -> String {
    Prose::new(text).render(&Terminal::default())
}

/// Run the interactive setup flow.
pub fn run(provider_arg: Option<RouteProvider>) -> Result<()> {
    let mut config = Config::load()?;

    let provider = provider_arg.unwrap_or(select_provider()?);

    // Check for existing configurations
    let existing = config.routes_for_provider(provider);
    if !existing.is_empty() {
        match handle_existing(&provider, &existing)? {
            ExistingAction::Exit => return Ok(()),
            ExistingAction::AddAnother => {}
            ExistingAction::Modify(route_name) => {
                let route = configure_provider(&provider, Some(&route_name))?;
                config.routes.insert(route_name, route);
                config.save()?;
                println!("\n{}", styled("<green>Configuration updated.</green>"));
                println!(
                    "{}",
                    styled(format!(
                        "Saved to <dim>{}</dim>",
                        Config::config_path()?.display()
                    ))
                );
                return Ok(());
            }
        }
    }

    // Configure the provider
    let route = configure_provider(&provider, None)?;

    // Ask for a route name
    let default_name = suggest_route_name(&provider, &config);
    let route_name = Text::new("Route name:")
        .with_default(&default_name)
        .with_help_message("A short label for this configuration (e.g., slack.ops, discord.alerts)")
        .prompt()
        .map_err(handle_cancel)?;

    // Ask if this should be the default route
    let set_default = if config.default_route.is_none() {
        Confirm::new("Set as default route?")
            .with_default(true)
            .prompt()
            .map_err(handle_cancel)?
    } else {
        Confirm::new(&format!(
            "Replace current default route ({}) with this one?",
            config.default_route.as_deref().unwrap_or("")
        ))
        .with_default(false)
        .prompt()
        .map_err(handle_cancel)?
    };

    config.routes.insert(route_name.clone(), route);
    if set_default {
        config.default_route = Some(route_name.clone());
    }

    config.save()?;

    println!(
        "\n{}",
        styled(format!(
            "<green>Route <b>{route_name}</b> configured.</green>"
        ))
    );
    println!(
        "{}",
        styled(format!(
            "Saved to <dim>{}</dim>",
            Config::config_path()?.display()
        ))
    );

    Ok(())
}

fn select_provider() -> Result<RouteProvider> {
    let options: Vec<RouteProvider> = RouteProvider::ALL.to_vec();
    let selected = Select::new("Which provider would you like to configure?", options)
        .with_help_message("Use arrow keys to select, Enter to confirm")
        .prompt()
        .map_err(handle_cancel)?;
    Ok(selected)
}

enum ExistingAction {
    Exit,
    AddAnother,
    Modify(String),
}

fn handle_existing(provider: &RouteProvider, existing: &[String]) -> Result<ExistingAction> {
    if existing.len() == 1 {
        println!(
            "\n{}",
            styled(format!(
                "A configuration for <b>{provider}</b> called <blue>{}</blue> already exists. What would you like to do?",
                existing[0]
            ))
        );
    } else {
        let route_list = existing.join(", ");
        println!(
            "\n{}",
            styled(format!(
                "Multiple configurations for <b>{provider}</b> already exist (<dim>{route_list}</dim>). What would you like to do?"
            ))
        );
    }

    let mut options = vec![
        "Exit".to_string(),
        format!("Add another configuration for {provider}"),
    ];
    for route in existing {
        options.push(format!("Modify \"{route}\""));
    }

    let selected = Select::new("", options).prompt().map_err(handle_cancel)?;

    if selected == "Exit" {
        return Ok(ExistingAction::Exit);
    }
    if selected.starts_with("Add another") {
        return Ok(ExistingAction::AddAnother);
    }
    // Extract route name from "Modify "route""
    if let Some(name) = selected
        .strip_prefix("Modify \"")
        .and_then(|s| s.strip_suffix('"'))
    {
        return Ok(ExistingAction::Modify(name.to_string()));
    }
    Ok(ExistingAction::Exit)
}

fn configure_provider(provider: &RouteProvider, route_name: Option<&str>) -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled(format!(
            "Configuring <b>{provider}</b>{}",
            route_name
                .map(|n| format!(" (route: <blue>{n}</blue>)"))
                .unwrap_or_default()
        ))
    );

    match provider {
        RouteProvider::Discord => configure_discord(),
        RouteProvider::DiscordWebhook => configure_discord_webhook(),
        RouteProvider::Slack => configure_slack(),
        RouteProvider::SlackWebhook => configure_slack_webhook(),
        RouteProvider::Signal => configure_signal(),
        RouteProvider::WhatsApp => configure_whatsapp(),
        RouteProvider::Telegram => configure_telegram(),
        RouteProvider::Desktop => configure_desktop(),
    }
}

fn configure_discord_webhook() -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled(
            "<dim>Discord webhooks require the full webhook URL (https://discord.com/api/v10/webhooks/ID/TOKEN).</dim>"
        )
    );
    println!(
        "{}",
        styled("<dim>Create one under Server Settings → Integrations → Webhooks.</dim>")
    );
    println!(
        "{}",
        styled(
            "<dim>The URL binds both the channel and authentication — treat it as a secret.</dim>"
        )
    );

    let webhook_url = Text::new("Webhook URL:")
        .with_help_message("Full webhook URL (leave empty to use env var instead)")
        .prompt()
        .map_err(handle_cancel)?;
    let webhook_url = non_empty(webhook_url);

    let webhook_url_env = if webhook_url.is_none() {
        Text::new("Environment variable for webhook URL:")
            .with_default("DISCORD_WEBHOOK_URL")
            .with_help_message("The env var that holds your Discord webhook URL")
            .prompt()
            .map_err(handle_cancel)?
    } else {
        "DISCORD_WEBHOOK_URL".into()
    };

    Ok(RouteConfig::DiscordWebhook {
        webhook_url,
        webhook_url_env,
    })
}

fn configure_discord() -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled("<dim>Discord requires a Bot Token and a Channel ID.</dim>")
    );
    println!(
        "{}",
        styled("<dim>Create a bot at https://discord.com/developers/applications</dim>")
    );
    println!(
        "{}",
        styled(
            "<dim>Find the Channel ID by right-clicking a channel with Developer Mode enabled.</dim>"
        )
    );

    let bot_token = Text::new("Bot token:")
        .with_help_message("Your Discord bot token (leave empty to use env var instead)")
        .prompt()
        .map_err(handle_cancel)?;
    let bot_token = non_empty(bot_token);

    let bot_token_env = if bot_token.is_none() {
        Text::new("Environment variable for bot token:")
            .with_default("DISCORD_BOT_TOKEN")
            .with_help_message("The env var that holds your Discord bot token")
            .prompt()
            .map_err(handle_cancel)?
    } else {
        "DISCORD_BOT_TOKEN".into()
    };

    let channel_id = Text::new("Channel ID:")
        .with_placeholder("123456789012345678")
        .with_help_message("The numeric channel ID where messages will be sent")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::Discord {
        channel_id,
        bot_token,
        bot_token_env,
    })
}

fn configure_slack() -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled("<dim>Slack requires a Bot Token and a Channel ID.</dim>")
    );
    println!(
        "{}",
        styled(
            "<dim>Create an app at https://api.slack.com/apps and install it to your workspace.</dim>"
        )
    );
    println!(
        "{}",
        styled(
            "<dim>The bot token starts with xoxb- and is found under OAuth & Permissions.</dim>"
        )
    );
    println!(
        "{}",
        styled(
            "<dim>Find the Channel ID by right-clicking a channel and selecting \"View channel details\".</dim>"
        )
    );

    let bot_token = Text::new("Bot token:")
        .with_help_message("Your Slack bot token (xoxb-..., leave empty to use env var instead)")
        .prompt()
        .map_err(handle_cancel)?;
    let bot_token = non_empty(bot_token);

    let bot_token_env = if bot_token.is_none() {
        Text::new("Environment variable for bot token:")
            .with_default("SLACK_BOT_TOKEN")
            .with_help_message("The env var that holds your Slack bot token")
            .prompt()
            .map_err(handle_cancel)?
    } else {
        "SLACK_BOT_TOKEN".into()
    };

    let channel_id = Text::new("Channel ID:")
        .with_placeholder("C012345ABC")
        .with_help_message("The Slack channel ID (starts with C)")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::Slack {
        channel_id,
        bot_token,
        bot_token_env,
    })
}

fn configure_slack_webhook() -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled(
            "<dim>Slack Incoming Webhooks require the full webhook URL (https://hooks.slack.com/services/T.../B.../token).</dim>"
        )
    );
    println!(
        "{}",
        styled(
            "<dim>Create one under Slack app settings → Incoming Webhooks and pick a target channel.</dim>"
        )
    );
    println!(
        "{}",
        styled(
            "<dim>The URL binds both the channel and authentication — treat it as a secret.</dim>"
        )
    );

    let use_env_var = Confirm::new("Load the webhook URL from an environment variable?")
        .with_default(false)
        .with_help_message(
            "Select Yes to store only an env-var name in config; select No to enter the URL now (masked input).",
        )
        .prompt()
        .map_err(handle_cancel)?;

    if use_env_var {
        let webhook_url_env = Text::new("Environment variable for webhook URL:")
            .with_default("SLACK_WEBHOOK_URL")
            .with_help_message("The env var that holds your Slack webhook URL")
            .prompt()
            .map_err(handle_cancel)?;

        return Ok(RouteConfig::SlackWebhook {
            webhook_url: None,
            webhook_url_env,
        });
    }

    let webhook_url = Password::new("Webhook URL:")
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_help_message("Full Slack webhook URL — treated as a secret and will not be echoed")
        .without_confirmation()
        .with_validator(|input: &str| {
            if input.trim().is_empty() {
                Ok(inquire::validator::Validation::Invalid(
                    "Webhook URL cannot be empty.".into(),
                ))
            } else {
                Ok(inquire::validator::Validation::Valid)
            }
        })
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::SlackWebhook {
        webhook_url: Some(webhook_url),
        webhook_url_env: "SLACK_WEBHOOK_URL".into(),
    })
}

fn configure_signal() -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled("<dim>Signal requires a running signal-cli daemon with JSON-RPC enabled.</dim>")
    );
    println!(
        "{}",
        styled("<dim>Install signal-cli: https://github.com/AsamK/signal-cli</dim>")
    );
    println!(
        "{}",
        styled("<dim>Start the daemon: signal-cli -a +1234567890 daemon --json-rpc</dim>")
    );
    println!(
        "{}",
        styled("<dim>The recipient is a phone number (+1234567890) or a base64 group ID.</dim>")
    );

    let rpc_url = Text::new("RPC URL:")
        .with_placeholder("http://localhost:7583")
        .with_help_message("The signal-cli JSON-RPC URL (leave empty to use env var instead)")
        .prompt()
        .map_err(handle_cancel)?;
    let rpc_url = non_empty(rpc_url);

    let rpc_url_env = if rpc_url.is_none() {
        Text::new("Environment variable for RPC URL:")
            .with_default("SIGNAL_RPC_URL")
            .with_help_message("The env var that holds the signal-cli JSON-RPC URL")
            .prompt()
            .map_err(handle_cancel)?
    } else {
        "SIGNAL_RPC_URL".into()
    };

    let account = Text::new("Account phone number:")
        .with_placeholder("+1234567890")
        .with_help_message(
            "Your registered Signal phone number (leave empty to use env var instead)",
        )
        .prompt()
        .map_err(handle_cancel)?;
    let account = non_empty(account);

    let account_env = if account.is_none() {
        Text::new("Environment variable for account:")
            .with_default("SIGNAL_ACCOUNT")
            .with_help_message("The env var for your registered Signal phone number")
            .prompt()
            .map_err(handle_cancel)?
    } else {
        "SIGNAL_ACCOUNT".into()
    };

    let recipient = Text::new("Recipient (phone number or group ID):")
        .with_placeholder("+15551234567")
        .with_help_message("Phone number with country code, or base64-encoded group ID")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::Signal {
        recipient,
        rpc_url,
        rpc_url_env,
        account,
        account_env,
    })
}

fn configure_whatsapp() -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled("<dim>WhatsApp requires a Cloud API access token and phone number ID.</dim>")
    );
    println!(
        "{}",
        styled("<dim>Set up at https://developers.facebook.com/ under the WhatsApp product.</dim>")
    );
    println!(
        "{}",
        styled("<dim>The access token is found in the API Setup section.</dim>")
    );
    println!(
        "{}",
        styled(
            "<dim>The phone number ID identifies which WhatsApp Business number sends messages.</dim>"
        )
    );

    let access_token = Text::new("Access token:")
        .with_help_message(
            "Your WhatsApp Cloud API access token (leave empty to use env var instead)",
        )
        .prompt()
        .map_err(handle_cancel)?;
    let access_token = non_empty(access_token);

    let access_token_env = if access_token.is_none() {
        Text::new("Environment variable for access token:")
            .with_default("WHATSAPP_ACCESS_TOKEN")
            .with_help_message("The env var that holds your WhatsApp Cloud API access token")
            .prompt()
            .map_err(handle_cancel)?
    } else {
        "WHATSAPP_ACCESS_TOKEN".into()
    };

    let phone_number_id = Text::new("Phone number ID:")
        .with_help_message(
            "Your WhatsApp Business phone number ID (leave empty to use env var instead)",
        )
        .prompt()
        .map_err(handle_cancel)?;
    let phone_number_id = non_empty(phone_number_id);

    let phone_number_id_env = if phone_number_id.is_none() {
        Text::new("Environment variable for phone number ID:")
            .with_default("WHATSAPP_PHONE_NUMBER_ID")
            .with_help_message("The env var for your WhatsApp Business phone number ID")
            .prompt()
            .map_err(handle_cancel)?
    } else {
        "WHATSAPP_PHONE_NUMBER_ID".into()
    };

    let recipient = Text::new("Default recipient phone number:")
        .with_placeholder("+15551234567")
        .with_help_message("The phone number to send messages to (with country code)")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::WhatsApp {
        recipient,
        access_token,
        access_token_env,
        phone_number_id,
        phone_number_id_env,
    })
}

fn configure_telegram() -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled("<dim>Telegram requires a Bot Token and a Chat ID.</dim>")
    );
    println!(
        "{}",
        styled("<dim>Create a bot via @BotFather on Telegram (https://t.me/BotFather).</dim>")
    );
    println!(
        "{}",
        styled("<dim>The bot token looks like 123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11.</dim>")
    );
    println!(
        "{}",
        styled(
            "<dim>Find your Chat ID by messaging @userinfobot or using the Bot API's getUpdates.</dim>"
        )
    );

    let bot_token = Text::new("Bot token:")
        .with_help_message("Your Telegram bot token (leave empty to use env var instead)")
        .prompt()
        .map_err(handle_cancel)?;
    let bot_token = non_empty(bot_token);

    let bot_token_env = if bot_token.is_none() {
        Text::new("Environment variable for bot token:")
            .with_default("TELEGRAM_BOT_TOKEN")
            .with_help_message("The env var that holds your Telegram bot token")
            .prompt()
            .map_err(handle_cancel)?
    } else {
        "TELEGRAM_BOT_TOKEN".into()
    };

    let chat_id = Text::new("Chat ID:")
        .with_placeholder("-1001234567890 or @channelname")
        .with_help_message("Numeric chat ID or @username for the target chat/channel/group")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::Telegram {
        chat_id,
        bot_token,
        bot_token_env,
    })
}

fn configure_desktop() -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled(
            "<dim>Desktop notifications deliver to the host OS notification center.</dim>"
        )
    );
    println!(
        "{}",
        styled(
            "<dim>No channel or token is needed; platform-specific bootstrap (Windows shortcut, macOS strategy) is covered by the full setup flow.</dim>"
        )
    );

    let app_name = Text::new("App name:")
        .with_default("Messenger")
        .with_help_message("Name shown by the notification center")
        .prompt()
        .map_err(handle_cancel)?;

    let default_title = Text::new("Default title (optional):")
        .with_help_message("Used when a message does not supply its own title")
        .prompt()
        .map_err(handle_cancel)?;

    let mut route = RouteConfig::desktop_default();
    if let RouteConfig::Desktop {
        app_name: ref mut cfg_app_name,
        default_title: ref mut cfg_default_title,
        ..
    } = route
    {
        *cfg_app_name = app_name;
        *cfg_default_title = non_empty(default_title);
    }
    Ok(route)
}

fn suggest_route_name(provider: &RouteProvider, config: &Config) -> String {
    let base = provider.to_string();
    if !config.routes.contains_key(&base) {
        return base;
    }
    // Try provider.2, provider.3, etc.
    for i in 2.. {
        let name = format!("{base}.{i}");
        if !config.routes.contains_key(&name) {
            return name;
        }
    }
    base
}

fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

fn handle_cancel(err: InquireError) -> color_eyre::eyre::Error {
    match err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            std::process::exit(0);
        }
        other => other.into(),
    }
}
