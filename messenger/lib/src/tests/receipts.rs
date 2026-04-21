use crate::receipt::{MessageRef, ProviderKind, SendReceipt};
use std::collections::BTreeMap;

#[cfg(feature = "signal")]
use crate::receipt::{SignalAuthor, SignalThreadKey};

#[cfg(feature = "telegram")]
use crate::receipt::TelegramChatRef;

// Test #3: MessageRef::provider_kind tests
#[cfg(feature = "discord")]
#[test]
fn message_ref_provider_kind_discord() {
    let msg_ref = MessageRef::Discord {
        channel_id: "123".into(),
        message_id: "456".into(),
    };
    assert_eq!(msg_ref.provider_kind(), ProviderKind::Discord);
}

#[cfg(feature = "slack")]
#[test]
fn message_ref_provider_kind_slack() {
    let msg_ref = MessageRef::Slack {
        channel_id: "C123".into(),
        thread_ts: "123.456".into(),
    };
    assert_eq!(msg_ref.provider_kind(), ProviderKind::Slack);
}

#[cfg(feature = "slack")]
#[test]
fn message_ref_provider_kind_slack_webhook() {
    let msg_ref = MessageRef::SlackWebhook { thread_ts: None };
    assert_eq!(msg_ref.provider_kind(), ProviderKind::SlackWebhook);
}

#[cfg(feature = "slack")]
#[test]
fn message_ref_provider_kind_slack_webhook_with_thread() {
    let msg_ref = MessageRef::SlackWebhook {
        thread_ts: Some("123.456".into()),
    };
    assert_eq!(msg_ref.provider_kind(), ProviderKind::SlackWebhook);
}

#[cfg(feature = "signal")]
#[test]
fn message_ref_provider_kind_signal() {
    let msg_ref = MessageRef::Signal {
        thread: SignalThreadKey::Direct("+15551234567".into()),
        author: SignalAuthor::Phone("+15551234567".into()),
        timestamp_ms: 1234567890,
    };
    assert_eq!(msg_ref.provider_kind(), ProviderKind::Signal);
}

#[cfg(feature = "whatsapp")]
#[test]
fn message_ref_provider_kind_whatsapp() {
    let msg_ref = MessageRef::WhatsApp {
        message_id: "wamid.123".into(),
    };
    assert_eq!(msg_ref.provider_kind(), ProviderKind::WhatsApp);
}

#[cfg(feature = "telegram")]
#[test]
fn message_ref_provider_kind_telegram() {
    let msg_ref = MessageRef::Telegram {
        chat_id: TelegramChatRef::Id(12345),
        message_id: 67890,
        thread_id: None,
    };
    assert_eq!(msg_ref.provider_kind(), ProviderKind::Telegram);
}

// Test #4: MessageRef / SendReceipt JSON round-trip tests
#[cfg(feature = "discord")]
#[test]
fn message_ref_discord_json_roundtrip() {
    let msg_ref = MessageRef::Discord {
        channel_id: "123456789".into(),
        message_id: "987654321".into(),
    };
    let json = msg_ref.to_pretty_json().unwrap();
    let parsed = MessageRef::from_json_str(&json).unwrap();
    assert_eq!(msg_ref, parsed);
}

#[cfg(feature = "slack")]
#[test]
fn message_ref_slack_json_roundtrip() {
    let msg_ref = MessageRef::Slack {
        channel_id: "C123ABC".into(),
        thread_ts: "1234567890.123456".into(),
    };
    let json = msg_ref.to_pretty_json().unwrap();
    let parsed = MessageRef::from_json_str(&json).unwrap();
    assert_eq!(msg_ref, parsed);
}

#[cfg(feature = "slack")]
#[test]
fn message_ref_slack_webhook_json_roundtrip() {
    let msg_ref = MessageRef::SlackWebhook { thread_ts: None };
    let json = msg_ref.to_pretty_json().unwrap();
    let parsed = MessageRef::from_json_str(&json).unwrap();
    assert_eq!(msg_ref, parsed);
}

#[cfg(feature = "slack")]
#[test]
fn message_ref_slack_webhook_with_thread_json_roundtrip() {
    let msg_ref = MessageRef::SlackWebhook {
        thread_ts: Some("1234567890.123456".into()),
    };
    let json = msg_ref.to_pretty_json().unwrap();
    let parsed = MessageRef::from_json_str(&json).unwrap();
    assert_eq!(msg_ref, parsed);
}

#[cfg(feature = "slack")]
#[test]
fn send_receipt_slack_webhook_json_roundtrip() {
    let receipt = SendReceipt {
        provider: ProviderKind::SlackWebhook,
        message_ref: MessageRef::SlackWebhook { thread_ts: None },
        raw_id: String::new(),
        metadata: BTreeMap::from([("delivery_confirmed".into(), "true".into())]),
    };
    let json = receipt.to_pretty_json().unwrap();
    let parsed = SendReceipt::from_json_str(&json).unwrap();
    assert_eq!(receipt, parsed);
}

#[cfg(feature = "discord")]
#[test]
fn send_receipt_discord_json_roundtrip() {
    let receipt = SendReceipt {
        provider: ProviderKind::Discord,
        message_ref: MessageRef::Discord {
            channel_id: "123".into(),
            message_id: "456".into(),
        },
        raw_id: "456".into(),
        metadata: BTreeMap::from([("foo".into(), "bar".into()), ("baz".into(), "qux".into())]),
    };
    let json = receipt.to_pretty_json().unwrap();
    let parsed = SendReceipt::from_json_str(&json).unwrap();
    assert_eq!(receipt, parsed);
}

#[cfg(feature = "telegram")]
#[test]
fn send_receipt_telegram_json_roundtrip() {
    let receipt = SendReceipt {
        provider: ProviderKind::Telegram,
        message_ref: MessageRef::Telegram {
            chat_id: TelegramChatRef::Username("testuser".into()),
            message_id: 12345,
            thread_id: Some(67890),
        },
        raw_id: "12345".into(),
        metadata: BTreeMap::new(),
    };
    let json = receipt.to_pretty_json().unwrap();
    let parsed = SendReceipt::from_json_str(&json).unwrap();
    assert_eq!(receipt, parsed);
}
