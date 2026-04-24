use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredReceipt {
    pub route_name: Option<String>,
    pub receipt: messenger::SendReceipt,
}

pub fn save_receipt(receipt: &messenger::SendReceipt, route_name: Option<&str>) -> Result<PathBuf> {
    let receipts_dir = receipts_dir()?;
    std::fs::create_dir_all(&receipts_dir)?;

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| eyre!("system clock is before the Unix epoch: {error}"))?
        .as_millis();
    let filename = format!(
        "{}-{}.json",
        timestamp_ms,
        receipt.provider.to_string().to_lowercase()
    );
    let path = receipts_dir.join(filename);

    let stored = StoredReceipt {
        route_name: route_name.map(ToOwned::to_owned),
        receipt: receipt.clone(),
    };
    tracing::debug!(
        path = %path.display(),
        provider = %receipt.provider,
        route_name = route_name.unwrap_or("<ad-hoc>"),
        "saving receipt"
    );
    std::fs::write(&path, serde_json::to_string_pretty(&stored)?)?;
    tracing::debug!(path = %path.display(), "saved receipt");

    Ok(path)
}

pub fn load_receipt(spec: &str) -> Result<messenger::SendReceipt> {
    tracing::debug!(spec = %spec, "loading receipt");
    let candidate_path = Path::new(spec);
    if candidate_path.exists() {
        tracing::debug!(path = %candidate_path.display(), "loading receipt from file");
        let contents = std::fs::read_to_string(candidate_path)?;
        return parse_receipt_contents(&contents);
    }

    // Try parsing the spec directly as a SendReceipt JSON
    messenger::SendReceipt::from_json_str(spec).map_err(|_| {
        eyre!("unable to parse receipt. Pass a saved receipt path or a SendReceipt JSON blob.")
    })
}

pub fn load_message_ref(spec: &str) -> Result<messenger::MessageRef> {
    tracing::debug!(spec = %spec, "loading message ref");
    let candidate_path = Path::new(spec);
    if candidate_path.exists() {
        tracing::debug!(path = %candidate_path.display(), "loading message ref from file");
        let contents = std::fs::read_to_string(candidate_path)?;
        return parse_message_ref_contents(&contents);
    }

    parse_message_ref_contents(spec)
}

fn parse_receipt_contents(contents: &str) -> Result<messenger::SendReceipt> {
    tracing::trace!("attempting StoredReceipt parse");
    if let Ok(stored) = serde_json::from_str::<StoredReceipt>(contents) {
        tracing::trace!("parsed as StoredReceipt");
        return Ok(stored.receipt);
    }
    tracing::trace!("attempting SendReceipt parse");
    if let Ok(receipt) = messenger::SendReceipt::from_json_str(contents) {
        tracing::trace!("parsed as SendReceipt");
        return Ok(receipt);
    }

    Err(eyre!(
        "unable to parse receipt. Pass a saved receipt path or a SendReceipt JSON blob."
    ))
}

fn parse_message_ref_contents(contents: &str) -> Result<messenger::MessageRef> {
    tracing::trace!("attempting StoredReceipt parse");
    if let Ok(stored) = serde_json::from_str::<StoredReceipt>(contents) {
        tracing::trace!("parsed as StoredReceipt");
        return Ok(stored.receipt.message_ref);
    }
    tracing::trace!("attempting SendReceipt parse");
    if let Ok(receipt) = messenger::SendReceipt::from_json_str(contents) {
        tracing::trace!("parsed as SendReceipt");
        return Ok(receipt.message_ref);
    }
    tracing::trace!("attempting MessageRef parse");
    if let Ok(message_ref) = messenger::MessageRef::from_json_str(contents) {
        tracing::trace!("parsed as MessageRef");
        return Ok(message_ref);
    }

    Err(eyre!(
        "unable to parse --reply-to. Pass a saved receipt path, a SendReceipt JSON blob, or a MessageRef JSON blob."
    ))
}

fn receipts_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| eyre!("could not determine home directory"))?;
    Ok(home.join(".messenger").join("receipts"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_message_ref_json() {
        let input = r#"{"slack":{"channel_id":"C123","thread_ts":"1712345678.000100"}}"#;
        let parsed = load_message_ref(input).unwrap();

        assert_eq!(
            parsed,
            messenger::MessageRef::Slack {
                channel_id: "C123".into(),
                thread_ts: "1712345678.000100".into(),
            }
        );
    }

    #[test]
    fn parse_stored_receipt_json_to_extract_message_ref() {
        let input = r#"{
            "route_name": "slack.ops",
            "receipt": {
                "provider": "slack",
                "message_ref": {
                    "slack": {
                        "channel_id": "C123",
                        "thread_ts": "1712345678.000100"
                    }
                },
                "raw_id": "1712345678.000100",
                "metadata": {}
            }
        }"#;

        let parsed = parse_message_ref_contents(input).unwrap();
        assert_eq!(
            parsed,
            messenger::MessageRef::Slack {
                channel_id: "C123".into(),
                thread_ts: "1712345678.000100".into(),
            }
        );
    }

    #[test]
    fn parse_send_receipt_json_to_extract_message_ref() {
        let input = r#"{
            "provider": "discord",
            "message_ref": {
                "discord": {
                    "channel_id": "456",
                    "message_id": "987654321"
                }
            },
            "raw_id": "987654321",
            "metadata": {}
        }"#;

        let parsed = parse_message_ref_contents(input).unwrap();
        assert_eq!(
            parsed,
            messenger::MessageRef::Discord {
                channel_id: "456".into(),
                message_id: "987654321".into(),
            }
        );
    }

    #[test]
    fn parse_message_ref_json_round_trip() {
        let message_ref = messenger::MessageRef::Telegram {
            chat_id: messenger::receipt::TelegramChatRef::Id(-1001234567),
            message_id: 42,
            thread_id: None,
        };

        let json = message_ref.to_pretty_json().unwrap();
        let parsed = parse_message_ref_contents(&json).unwrap();

        assert_eq!(parsed, message_ref);
    }
}
