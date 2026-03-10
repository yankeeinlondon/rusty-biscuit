use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredReceipt {
    pub route_name: Option<String>,
    pub receipt: messenger::SendReceipt,
}

pub fn save_receipt(
    receipt: &messenger::SendReceipt,
    route_name: Option<&str>,
) -> Result<PathBuf> {
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
    std::fs::write(&path, serde_json::to_string_pretty(&stored)?)?;

    Ok(path)
}

pub fn load_message_ref(spec: &str) -> Result<messenger::MessageRef> {
    let candidate_path = Path::new(spec);
    if candidate_path.exists() {
        let contents = std::fs::read_to_string(candidate_path)?;
        return parse_message_ref_contents(&contents);
    }

    parse_message_ref_contents(spec)
}

fn parse_message_ref_contents(contents: &str) -> Result<messenger::MessageRef> {
    if let Ok(stored) = serde_json::from_str::<StoredReceipt>(contents) {
        return Ok(stored.receipt.message_ref);
    }
    if let Ok(receipt) = messenger::SendReceipt::from_json_str(contents) {
        return Ok(receipt.message_ref);
    }
    if let Ok(message_ref) = messenger::MessageRef::from_json_str(contents) {
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
}
