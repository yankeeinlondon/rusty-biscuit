use claudine::config::claudine_config::MessengerProviderConfig;

use super::MessengerField;

pub const NAME: &str = "discord";

pub fn fields() -> Vec<MessengerField> {
    vec![
        ("Channel ID".to_string(), String::new(), false),
        (
            "Bot Token Env Var".to_string(),
            "DISCORD_BOT_TOKEN".to_string(),
            false,
        ),
    ]
}

pub fn build(fields: &[(String, String)]) -> MessengerProviderConfig {
    MessengerProviderConfig::Discord {
        channel_id: fields.first().map(|(_, v)| v.clone()).unwrap_or_default(),
        bot_token_env: fields
            .get(1)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "DISCORD_BOT_TOKEN".to_string()),
    }
}
