use claudine::config::claudine_config::MessengerProviderConfig;

pub mod discord_bot;
pub mod discord_webhook;
pub mod signal;
pub mod slack_bot;
pub mod slack_webhook;
pub mod whatsapp;

/// Field definition for a messenger provider input field.
/// (label, default_value, is_secret)
pub type MessengerField = (String, String, bool);

/// Ordered list of supported messenger provider identifiers.
pub const PROVIDERS: [&str; 6] = [
    discord_bot::NAME,
    discord_webhook::NAME,
    slack_bot::NAME,
    slack_webhook::NAME,
    signal::NAME,
    whatsapp::NAME,
];

/// Returns true when the provider identifier corresponds to a webhook route
/// (Discord or Slack), which gates the test-connection workflow and webhook
/// URL validation.
pub fn is_webhook(provider: &str) -> bool {
    provider == discord_webhook::NAME || provider == slack_webhook::NAME
}

/// Returns the ordered list of fields for a messenger provider.
pub fn messenger_fields(provider: &str) -> Vec<MessengerField> {
    match provider {
        discord_bot::NAME => discord_bot::fields(),
        slack_bot::NAME => slack_bot::fields(),
        signal::NAME => signal::fields(),
        whatsapp::NAME => whatsapp::fields(),
        discord_webhook::NAME => discord_webhook::fields(),
        slack_webhook::NAME => slack_webhook::fields(),
        _ => vec![],
    }
}

/// Returns the ordered list of fields for a messenger provider,
/// **including** the leading "Configuration Name" field at index 0.
pub fn messenger_fields_with_name(provider: &str) -> Vec<MessengerField> {
    let mut fields = vec![(
        "Configuration Name".to_string(),
        provider.to_string(),
        false,
    )];
    fields.extend(messenger_fields(provider));
    fields
}

/// Build the actual `MessengerProviderConfig` from collected field values.
pub fn build_messenger_from_fields(
    provider: &str,
    fields: &[(String, String)],
) -> Option<MessengerProviderConfig> {
    match provider {
        discord_bot::NAME => Some(discord_bot::build(fields)),
        slack_bot::NAME => Some(slack_bot::build(fields)),
        signal::NAME => Some(signal::build(fields)),
        whatsapp::NAME => Some(whatsapp::build(fields)),
        discord_webhook::NAME => Some(discord_webhook::build(fields)),
        slack_webhook::NAME => Some(slack_webhook::build(fields)),
        _ => None,
    }
}
