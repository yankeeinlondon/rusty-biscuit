use claudine::config::claudine_config::MessengerProviderConfig;

use super::MessengerField;

pub const NAME: &str = "slack_webhook";

pub fn fields() -> Vec<MessengerField> {
    vec![
        ("Webhook URL".to_string(), String::new(), true),
        (
            "Webhook URL Env Var".to_string(),
            "SLACK_WEBHOOK_URL".to_string(),
            false,
        ),
    ]
}

pub fn build(fields: &[(String, String)]) -> MessengerProviderConfig {
    let webhook_url = fields.first().map(|(_, v)| v.clone());
    let webhook_url = if webhook_url
        .as_ref()
        .map(|s| s.trim().is_empty())
        .unwrap_or(true)
    {
        None
    } else {
        webhook_url
    };
    MessengerProviderConfig::SlackWebhook {
        webhook_url,
        webhook_url_env: fields
            .get(1)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "SLACK_WEBHOOK_URL".to_string()),
    }
}
