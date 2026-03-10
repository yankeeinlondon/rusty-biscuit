use crate::{
    Attachment, AttachmentSource, CapabilitySet, CompatibilityMode, Dispatch, Message,
    MessengerError, ProviderKind, Target,
    validate_dispatch, validate_message,
};
use crate::receipt::MessageRef;

#[test]
fn empty_message_is_invalid() {
    let msg = Message {
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

#[cfg(all(feature = "discord", feature = "slack"))]
#[test]
fn mismatched_reply_to_target() {
    let dispatch = Dispatch::to(Target::discord_channel("123"))
        .reply_to(MessageRef::Slack {
            channel_id: "C123".into(),
            thread_ts: "123.456".into(),
        });
    let msg = Message::text("hi");
    let caps = CapabilitySet::all();

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(err, MessengerError::InvalidMessage(_)));
}

#[cfg(feature = "discord")]
#[test]
fn matched_reply_to_target_ok() {
    let dispatch = Dispatch::to(Target::discord_channel("123"))
        .reply_to(MessageRef::Discord {
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
        MessengerError::UnsupportedFeature { feature: "markdown rendering", .. }
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
        supports_attachments: false,
        ..CapabilitySet::none()
    };

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(
        err,
        MessengerError::UnsupportedFeature { feature: "attachments", .. }
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
        MessengerError::UnsupportedFeature { feature: "location", .. }
    ));
}

#[cfg(feature = "discord")]
#[test]
fn strict_mode_rejects_unsupported_silent() {
    let dispatch = Dispatch::to(Target::discord_channel("123")).strict().silent();
    let msg = Message::text("hi");
    let caps = CapabilitySet {
        supports_silent_delivery: false,
        ..CapabilitySet::none()
    };

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(
        err,
        MessengerError::UnsupportedFeature { feature: "silent delivery", .. }
    ));
}

#[cfg(feature = "discord")]
#[test]
fn best_effort_mode_allows_markdown_downgrade() {
    let dispatch = Dispatch::to(Target::discord_channel("123"));
    let msg = Message::markdown("**bold**");
    let caps = CapabilitySet::none();

    assert_eq!(dispatch.options.compatibility, CompatibilityMode::BestEffort);
    assert!(validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).is_ok());
}

#[cfg(feature = "discord")]
#[test]
fn best_effort_mode_rejects_unsupported_attachments() {
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

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(
        err,
        MessengerError::UnsupportedFeature { feature: "attachments", .. }
    ));
}

#[cfg(feature = "discord")]
#[test]
fn best_effort_mode_rejects_unsupported_location() {
    let dispatch = Dispatch::to(Target::discord_channel("123"));
    let msg = Message::location(0.0, 0.0);
    let caps = CapabilitySet::none();

    let err = validate_dispatch(&dispatch, &msg, &caps, ProviderKind::Discord).unwrap_err();
    assert!(matches!(
        err,
        MessengerError::UnsupportedFeature { feature: "location", .. }
    ));
}

#[cfg(feature = "discord")]
#[test]
fn attachment_path_must_exist() {
    let dispatch = Dispatch::to(Target::discord_channel("123"));
    let msg = Message::text("hi").image("/definitely/missing/file.png");
    let caps = CapabilitySet {
        supports_attachments: true,
        ..CapabilitySet::all()
    };

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
    let caps = CapabilitySet {
        supports_attachments: true,
        ..CapabilitySet::all()
    };

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
