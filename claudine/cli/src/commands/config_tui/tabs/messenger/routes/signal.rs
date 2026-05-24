use claudine::config::claudine_config::MessengerProviderConfig;

use super::MessengerField;

pub const NAME: &str = "signal";

pub fn fields() -> Vec<MessengerField> {
    vec![
        ("Recipient".to_string(), String::new(), false),
        (
            "RPC URL Env Var".to_string(),
            "SIGNAL_RPC_URL".to_string(),
            false,
        ),
        (
            "Account Env Var".to_string(),
            "SIGNAL_ACCOUNT".to_string(),
            false,
        ),
    ]
}

pub fn build(fields: &[(String, String)]) -> MessengerProviderConfig {
    MessengerProviderConfig::Signal {
        recipient: fields.first().map(|(_, v)| v.clone()).unwrap_or_default(),
        rpc_url_env: fields
            .get(1)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "SIGNAL_RPC_URL".to_string()),
        account_env: fields
            .get(2)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "SIGNAL_ACCOUNT".to_string()),
    }
}
