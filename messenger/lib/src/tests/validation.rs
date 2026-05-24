use crate::receipt::MessageRef;
use crate::{
    Attachment, AttachmentSource, CapabilitySet, CompatibilityMode, Dispatch, Message,
    MessengerError, ProviderKind, Target, normalize_dispatch, validate_dispatch, validate_message,
};

#[cfg(any(feature = "desktop", feature = "apns", feature = "fcm"))]
use crate::validate_message_for_provider;

#[test]
fn empty_message_is_invalid() {
    let msg = Message {
        title: None,
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };
    let err = validate_message(&msg).unwrap_err();
    assert!(matches!(err, MessengerError::InvalidMessage(_)));
}

#[test]
fn non_empty_message_is_valid() {
    assert!(validate_message(&Message::text("hi")).is_ok());
}

#[cfg(feature = "desktop")]
#[test]
fn title_only_is_invalid_for_non_desktop_providers() {
    let message = Message {
        title: Some("Title".into()),
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };

    for provider in [
        ProviderKind::Discord,
        ProviderKind::DiscordWebhook,
        ProviderKind::Slack,
        ProviderKind::SlackWebhook,
        ProviderKind::Signal,
        ProviderKind::WhatsApp,
        ProviderKind::Telegram,
    ] {
        assert!(
            matches!(
                crate::validate_message_for_provider(&message, provider),
                Err(MessengerError::InvalidMessage(_))
            ),
            "title-only must be invalid for {provider}"
        );
    }
}

#[cfg(feature = "desktop")]
#[test]
fn title_only_is_valid_for_desktop_provider() {
    let message = Message {
        title: Some("Title".into()),
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };

    assert!(validate_message_for_provider(&message, ProviderKind::Desktop).is_ok());
}

#[cfg(feature = "apns")]
#[test]
fn title_only_is_valid_for_apns_provider() {
    let message = Message {
        title: Some("Title".into()),
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };

    assert!(validate_message_for_provider(&message, ProviderKind::Apns).is_ok());
}

#[cfg(feature = "fcm")]
#[test]
fn title_only_is_valid_for_fcm_provider() {
    let message = Message {
        title: Some("Title".into()),
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };

    assert!(validate_message_for_provider(&message, ProviderKind::Fcm).is_ok());
}

#[cfg(feature = "desktop")]
#[test]
fn empty_message_is_invalid_even_for_desktop() {
    let message = Message {
        title: None,
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };

    assert!(matches!(
        validate_message_for_provider(&message, ProviderKind::Desktop),
        Err(MessengerError::InvalidMessage(_))
    ));
}

#[cfg(feature = "apns")]
#[test]
fn empty_message_is_invalid_even_for_apns() {
    let message = Message {
        title: None,
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };

    assert!(matches!(
        validate_message_for_provider(&message, ProviderKind::Apns),
        Err(MessengerError::InvalidMessage(_))
    ));
}

#[cfg(feature = "fcm")]
#[test]
fn empty_message_is_invalid_even_for_fcm() {
    let message = Message {
        title: None,
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };

    assert!(matches!(
        validate_message_for_provider(&message, ProviderKind::Fcm),
        Err(MessengerError::InvalidMessage(_))
    ));
}

#[cfg(all(feature = "discord", feature = "slack"))]
#[test]
fn mismatched_reply_to_target() {
    let dispatch = Dispatch::to(Target::discord_channel("123")).reply_to(MessageRef::Slack {
        channel_id: "C123".into(),
        thread_ts: "123.456".into(),
    });
    let msg = Message::text("hi");
    let caps = CapabilitySet::all();

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(err, MessengerError::InvalidMessage(_)));
}

#[cfg(feature = "slack")]
#[test]
fn mismatched_reply_to_target_slack_webhook_with_slack_ref() {
    let dispatch = Dispatch::to(Target::slack_webhook()).reply_to(MessageRef::Slack {
        channel_id: "C123".into(),
        thread_ts: "123.456".into(),
    });
    let msg = Message::text("hi");
    let caps = CapabilitySet::all();

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::SlackWebhook).unwrap_err();
    assert!(matches!(err, MessengerError::InvalidMessage(_)));
}

#[cfg(feature = "slack")]
#[test]
fn matched_reply_to_target_slack_webhook_ok() {
    let dispatch = Dispatch::to(Target::slack_webhook()).reply_to(MessageRef::SlackWebhook {
        thread_ts: Some("123.456".into()),
    });
    let msg = Message::text("hi");
    let caps = CapabilitySet::all();

    assert!(validate_dispatch(&dispatch, &msg, &caps, ProviderKind::SlackWebhook).is_ok());
}

#[cfg(feature = "discord")]
#[test]
fn matched_reply_to_target_ok() {
    let dispatch = Dispatch::to(Target::discord_channel("123")).reply_to(MessageRef::Discord {
        channel_id: "123".into(),
        message_id: "456".into(),
    });
    let msg = Message::text("hi");
    let caps = CapabilitySet::all();

    assert!(validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).is_ok());
}

#[cfg(feature = "discord")]
#[test]
fn strict_mode_rejects_unsupported_markdown() {
    let dispatch = Dispatch::to(Target::discord_channel("123")).strict();
    let msg = Message::markdown("**bold**");
    let caps = CapabilitySet {
        supports_markdown_rendering: false,
        ..CapabilitySet::none()
    };

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(
        err,
        MessengerError::UnsupportedFeature {
            feature: "markdown rendering",
            ..
        }
    ));
}

#[cfg(feature = "discord")]
#[test]
fn strict_mode_rejects_unsupported_summarized() {
    let dispatch = Dispatch::to(Target::discord_channel("123")).strict();
    let msg = Message::summarized("s", "**m**");
    let caps = CapabilitySet {
        supports_markdown_rendering: false,
        ..CapabilitySet::none()
    };

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(
        err,
        MessengerError::UnsupportedFeature {
            feature: "markdown rendering",
            ..
        }
    ));
}

#[cfg(feature = "discord")]
#[test]
fn strict_mode_rejects_unsupported_attachments() {
    let dispatch = Dispatch::to(Target::discord_channel("123")).strict();
    let msg = Message::text("hi").attachment(Attachment {
        kind: crate::AttachmentKind::Document,
        source: AttachmentSource::Bytes {
            filename: "report.txt".into(),
            mime_type: "text/plain".into(),
            data: bytes::Bytes::from_static(b"ok"),
        },
        caption: None,
        alt_text: None,
    });
    let caps = CapabilitySet {
        supported_attachment_kinds: std::collections::BTreeSet::new(),
        ..CapabilitySet::none()
    };

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(
        err,
        MessengerError::UnsupportedFeature {
            feature: "attachments",
            ..
        }
    ));
}

#[cfg(feature = "discord")]
#[test]
fn strict_mode_rejects_unsupported_location() {
    let dispatch = Dispatch::to(Target::discord_channel("123")).strict();
    let msg = Message::location(0.0, 0.0);
    let caps = CapabilitySet {
        supports_location: false,
        ..CapabilitySet::none()
    };

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(
        err,
        MessengerError::UnsupportedFeature {
            feature: "location",
            ..
        }
    ));
}

#[cfg(feature = "discord")]
#[test]
fn strict_mode_rejects_unsupported_silent() {
    let dispatch = Dispatch::to(Target::discord_channel("123"))
        .strict()
        .silent();
    let msg = Message::text("hi");
    let caps = CapabilitySet {
        supports_silent_delivery: false,
        ..CapabilitySet::none()
    };

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(
        err,
        MessengerError::UnsupportedFeature {
            feature: "silent delivery",
            ..
        }
    ));
}

#[cfg(feature = "discord")]
#[test]
fn best_effort_mode_allows_markdown_downgrade() {
    let dispatch = Dispatch::to(Target::discord_channel("123"));
    let msg = Message::markdown("**bold**");
    let caps = CapabilitySet::none();

    assert_eq!(
        dispatch.options.compatibility,
        CompatibilityMode::BestEffort
    );
    let normalized = normalize_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap();
    assert_eq!(normalized.warnings.len(), 1);
    assert_eq!(normalized.warnings[0].feature, "markdown rendering");
    assert!(matches!(
        normalized.message.body,
        Some(crate::MessageBody::Markdown(_))
    ));
}

#[cfg(feature = "discord")]
#[test]
fn best_effort_mode_drops_unsupported_attachments() {
    let dispatch = Dispatch::to(Target::discord_channel("123"));
    let msg = Message::text("hi").attachment(Attachment {
        kind: crate::AttachmentKind::Document,
        source: AttachmentSource::Bytes {
            filename: "report.txt".into(),
            mime_type: "text/plain".into(),
            data: bytes::Bytes::from_static(b"ok"),
        },
        caption: None,
        alt_text: None,
    });
    let caps = CapabilitySet::none();

    let normalized = normalize_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap();
    assert!(normalized.message.attachments.is_empty());
    assert_eq!(normalized.warnings.len(), 1);
    assert_eq!(normalized.warnings[0].feature, "attachments");
}

#[cfg(feature = "discord")]
#[test]
fn best_effort_mode_drops_unsupported_location() {
    let dispatch = Dispatch::to(Target::discord_channel("123"));
    let msg = Message::text("hi").metadata("k", "v");
    let mut msg = msg;
    msg.location = Some(crate::Location {
        latitude: 0.0,
        longitude: 0.0,
        name: None,
        address: None,
    });
    let caps = CapabilitySet::none();

    let normalized = normalize_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap();
    assert!(normalized.message.location.is_none());
    assert_eq!(normalized.warnings.len(), 1);
    assert_eq!(normalized.warnings[0].feature, "location");
}

#[cfg(feature = "discord")]
#[test]
fn best_effort_mode_drops_reply_silent_and_link_preview() {
    let dispatch = Dispatch::to(Target::discord_channel("123"))
        .reply_to(MessageRef::Discord {
            channel_id: "123".into(),
            message_id: "456".into(),
        })
        .silent()
        .disable_link_preview();
    let msg = Message::text("hi");
    let caps = CapabilitySet {
        supports_reply: false,
        supports_silent_delivery: false,
        supports_link_preview_control: false,
        ..CapabilitySet::all()
    };

    let normalized = normalize_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap();
    assert!(normalized.dispatch.reply_to.is_none());
    assert!(!normalized.dispatch.options.silent);
    assert!(!normalized.dispatch.options.disable_link_preview);
    assert_eq!(normalized.warnings.len(), 3);
    assert_eq!(normalized.warnings[0].feature, "replies");
    assert_eq!(normalized.warnings[1].feature, "silent delivery");
    assert_eq!(normalized.warnings[2].feature, "link preview control");
}

#[cfg(feature = "discord")]
#[test]
fn best_effort_mode_errors_if_everything_is_dropped() {
    let dispatch = Dispatch::to(Target::discord_channel("123"));
    let msg = Message::location(0.0, 0.0);
    let caps = CapabilitySet::none();

    let err = normalize_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(
        matches!(err, MessengerError::InvalidMessage(message) if message.contains("after dropping unsupported features"))
    );
}

#[cfg(feature = "discord")]
#[test]
fn attachment_path_must_exist() {
    let dispatch = Dispatch::to(Target::discord_channel("123"));
    let msg = Message::text("hi").image("/definitely/missing/file.png");
    let caps = CapabilitySet::all();

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(err, MessengerError::InvalidMessage(_)));
}

#[cfg(feature = "discord")]
#[test]
fn attachment_provider_file_id_must_be_non_empty() {
    let dispatch = Dispatch::to(Target::discord_channel("123"));
    let msg = Message::text("hi").attachment(Attachment {
        kind: crate::AttachmentKind::Document,
        source: AttachmentSource::ProviderFileId(" \n".into()),
        caption: None,
        alt_text: None,
    });
    let caps = CapabilitySet::all();

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(err, MessengerError::InvalidMessage(_)));
}

#[test]
fn dispatch_builder_fluency() {
    let d = Dispatch::to(Target::discord_channel("123"))
        .silent()
        .disable_link_preview()
        .strict();

    assert!(d.options.silent);
    assert!(d.options.disable_link_preview);
    assert_eq!(d.options.compatibility, CompatibilityMode::Strict);
}
