use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::prelude::Renderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use inquire::{Confirm, InquireError, Select, Text};

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

    println!("\n{}", styled(format!("<green>Route <b>{route_name}</b> configured.</green>")));
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

    let selected = Select::new("", options)
        .prompt()
        .map_err(handle_cancel)?;

    if selected == "Exit" {
        return Ok(ExistingAction::Exit);
    }
    if selected.starts_with("Add another") {
        return Ok(ExistingAction::AddAnother);
    }
    // Extract route name from "Modify "route""
    if let Some(name) = selected.strip_prefix("Modify \"").and_then(|s| s.strip_suffix('"')) {
        return Ok(ExistingAction::Modify(name.to_string()));
    }
    Ok(ExistingAction::Exit)
}

fn configure_provider(provider: &RouteProvider, route_name: Option<&str>) -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled(format!("Configuring <b>{provider}</b>{}", route_name.map(|n| format!(" (route: <blue>{n}</blue>)")).unwrap_or_default()))
    );

    match provider {
        RouteProvider::Discord => configure_discord(),
        RouteProvider::Slack => configure_slack(),
        RouteProvider::Signal => configure_signal(),
        RouteProvider::WhatsApp => configure_whatsapp(),
        RouteProvider::Telegram => configure_telegram(),
    }
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
        styled("<dim>Find the Channel ID by right-clicking a channel with Developer Mode enabled.</dim>")
    );

    let token_env = Text::new("Environment variable for bot token:")
        .with_default("DISCORD_BOT_TOKEN")
        .with_help_message("The env var that holds your Discord bot token")
        .prompt()
        .map_err(handle_cancel)?;

    let channel_id = Text::new("Channel ID:")
        .with_placeholder("123456789012345678")
        .with_help_message("The numeric channel ID where messages will be sent")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::Discord {
        channel_id,
        bot_token_env: token_env,
    })
}

fn configure_slack() -> Result<RouteConfig> {
    println!(
        "\n{}",
        styled("<dim>Slack requires a Bot Token and a Channel ID.</dim>")
    );
    println!(
        "{}",
        styled("<dim>Create an app at https://api.slack.com/apps and install it to your workspace.</dim>")
    );
    println!(
        "{}",
        styled("<dim>The bot token starts with xoxb- and is found under OAuth & Permissions.</dim>")
    );
    println!(
        "{}",
        styled("<dim>Find the Channel ID by right-clicking a channel and selecting \"View channel details\".</dim>")
    );

    let token_env = Text::new("Environment variable for bot token:")
        .with_default("SLACK_BOT_TOKEN")
        .with_help_message("The env var that holds your Slack bot token (xoxb-...)")
        .prompt()
        .map_err(handle_cancel)?;

    let channel_id = Text::new("Channel ID:")
        .with_placeholder("C012345ABC")
        .with_help_message("The Slack channel ID (starts with C)")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::Slack {
        channel_id,
        bot_token_env: token_env,
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

    let rpc_env = Text::new("Environment variable for RPC URL:")
        .with_default("SIGNAL_RPC_URL")
        .with_help_message("The env var that holds the signal-cli JSON-RPC URL (e.g., http://localhost:7583)")
        .prompt()
        .map_err(handle_cancel)?;

    let account_env = Text::new("Environment variable for account:")
        .with_default("SIGNAL_ACCOUNT")
        .with_help_message("The env var for your registered Signal phone number (+1234567890)")
        .prompt()
        .map_err(handle_cancel)?;

    let recipient = Text::new("Recipient (phone number or group ID):")
        .with_placeholder("+15551234567")
        .with_help_message("Phone number with country code, or base64-encoded group ID")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::Signal {
        recipient,
        rpc_url_env: rpc_env,
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
        styled("<dim>The phone number ID identifies which WhatsApp Business number sends messages.</dim>")
    );

    let token_env = Text::new("Environment variable for access token:")
        .with_default("WHATSAPP_ACCESS_TOKEN")
        .with_help_message("The env var that holds your WhatsApp Cloud API access token")
        .prompt()
        .map_err(handle_cancel)?;

    let phone_number_id_env = Text::new("Environment variable for phone number ID:")
        .with_default("WHATSAPP_PHONE_NUMBER_ID")
        .with_help_message("The env var for your WhatsApp Business phone number ID")
        .prompt()
        .map_err(handle_cancel)?;

    let recipient = Text::new("Default recipient phone number:")
        .with_placeholder("+15551234567")
        .with_help_message("The phone number to send messages to (with country code)")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::WhatsApp {
        recipient,
        access_token_env: token_env,
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
        styled("<dim>Find your Chat ID by messaging @userinfobot or using the Bot API's getUpdates.</dim>")
    );

    let token_env = Text::new("Environment variable for bot token:")
        .with_default("TELEGRAM_BOT_TOKEN")
        .with_help_message("The env var that holds your Telegram bot token")
        .prompt()
        .map_err(handle_cancel)?;

    let chat_id = Text::new("Chat ID:")
        .with_placeholder("-1001234567890 or @channelname")
        .with_help_message("Numeric chat ID or @username for the target chat/channel/group")
        .prompt()
        .map_err(handle_cancel)?;

    Ok(RouteConfig::Telegram {
        chat_id,
        bot_token_env: token_env,
    })
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

fn handle_cancel(err: InquireError) -> color_eyre::eyre::Error {
    match err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            std::process::exit(0);
        }
        other => other.into(),
    }
}
