use claudine::config::claudine_config::MessengerProviderConfig;

use super::MessengerField;

pub const NAME: &str = "whatsapp";

pub fn fields() -> Vec<MessengerField> {
    vec![
        ("Recipient".to_string(), String::new(), false),
        (
            "Access Token Env Var".to_string(),
            "WHATSAPP_ACCESS_TOKEN".to_string(),
            false,
        ),
        (
            "Phone Number ID Env Var".to_string(),
            "WHATSAPP_PHONE_NUMBER_ID".to_string(),
            false,
        ),
    ]
}

pub fn build(fields: &[(String, String)]) -> MessengerProviderConfig {
    MessengerProviderConfig::Whatsapp {
        recipient: fields.first().map(|(_, v)| v.clone()).unwrap_or_default(),
        access_token_env: fields
            .get(1)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "WHATSAPP_ACCESS_TOKEN".to_string()),
        phone_number_id_env: fields
            .get(2)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "WHATSAPP_PHONE_NUMBER_ID".to_string()),
    }
}
