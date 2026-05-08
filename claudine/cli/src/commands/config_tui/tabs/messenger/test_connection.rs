use claudine::config::claudine_config::MessengerProviderConfig;
use claudine::messaging::MessagingRouteConfig;

use super::routes::{self, build_messenger_from_fields, messenger_fields_with_name};

/// Build a temporary [`MessagingRouteConfig`] from the current modal state
/// for test-connection workflow. Combines collected fields with the current
/// buffer to produce a complete (but unsaved) webhook route.
pub fn build_test_route_from_modal(
    provider: &str,
    fields: &[(String, String)],
    buffer: &str,
    field_index: usize,
) -> Option<MessagingRouteConfig> {
    let all_field_defs = messenger_fields_with_name(provider);

    // Build complete field values including the current buffer
    let mut complete = fields.to_vec();
    if field_index < all_field_defs.len() {
        let value = if buffer.trim().is_empty() {
            all_field_defs[field_index].1.clone()
        } else {
            buffer.to_string()
        };
        if complete.len() <= field_index {
            complete.push((all_field_defs[field_index].0.clone(), value));
        } else {
            complete[field_index] = (all_field_defs[field_index].0.clone(), value);
        }
    }

    // Skip config name (index 0) to get provider-specific fields
    let provider_fields: Vec<(String, String)> = complete.into_iter().skip(1).collect();
    let config = build_messenger_from_fields(provider, &provider_fields)?;

    match config {
        MessengerProviderConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => Some(MessagingRouteConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        }),
        MessengerProviderConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        } => Some(MessagingRouteConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        }),
        _ => None,
    }
}

/// Returns true when the provider/field combination is eligible for the
/// `T` test-connection key. Only webhook providers support this workflow,
/// and only after the user has advanced past the configuration name field.
pub fn can_test(provider: &str, field_index: usize) -> bool {
    routes::is_webhook(provider) && field_index >= 1
}
