use crate::{Attachment, AttachmentKind, Location, Message, MessageBody, PreparedMessage, ProviderKind};

#[test]
fn text_message_builder() {
    let msg = Message::text("hello");
    assert!(matches!(msg.body, Some(MessageBody::Plain(ref s)) if s == "hello"));
    assert!(msg.attachments.is_empty());
    assert!(msg.location.is_none());
}

#[test]
fn markdown_message_builder() {
    let msg = Message::markdown("**bold**");
    assert!(matches!(msg.body, Some(MessageBody::Markdown(ref s)) if s == "**bold**"));
}

#[test]
fn location_message_builder() {
    let msg = Message::location(34.05, -118.24);
    assert!(msg.body.is_none());
    let loc = msg.location.as_ref().unwrap();
    assert!((loc.latitude - 34.05).abs() < f64::EPSILON);
    assert!((loc.longitude - (-118.24)).abs() < f64::EPSILON);
}

#[test]
fn message_with_attachment() {
    let msg = Message::markdown("**hi**")
        .attachment(Attachment::image("/tmp/chart.png").caption("Chart"));
    assert_eq!(msg.attachments.len(), 1);
    assert_eq!(msg.attachments[0].kind, AttachmentKind::Image);
    assert_eq!(msg.attachments[0].caption.as_deref(), Some("Chart"));
}

#[test]
fn image_shorthand() {
    let msg = Message::text("see this").image("/tmp/photo.jpg");
    assert_eq!(msg.attachments.len(), 1);
    assert_eq!(msg.attachments[0].kind, AttachmentKind::Image);
}

#[test]
fn metadata_builder() {
    let msg = Message::text("deploy")
        .metadata("service", "api")
        .metadata("env", "prod");
    assert_eq!(msg.metadata.get("service").unwrap(), "api");
    assert_eq!(msg.metadata.get("env").unwrap(), "prod");
}

#[test]
fn is_empty_checks() {
    let empty = Message {
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };
    assert!(empty.is_empty());
    assert!(!Message::text("hi").is_empty());
    assert!(!Message::location(0.0, 0.0).is_empty());
}

// Test #1: Location::format_text_line tests (all 4 branches)
#[test]
fn location_format_with_name_and_address() {
    let loc = Location {
        latitude: 34.05,
        longitude: -118.24,
        name: Some("Griffith Observatory".into()),
        address: Some("2800 E Observatory Rd".into()),
    };
    assert_eq!(
        loc.format_text_line(),
        "📍 Griffith Observatory (2800 E Observatory Rd) — 34.0500, -118.2400"
    );
}

#[test]
fn location_format_with_name_only() {
    let loc = Location {
        latitude: 34.05,
        longitude: -118.24,
        name: Some("Griffith Observatory".into()),
        address: None,
    };
    assert_eq!(
        loc.format_text_line(),
        "📍 Griffith Observatory — 34.0500, -118.2400"
    );
}

#[test]
fn location_format_with_address_only() {
    let loc = Location {
        latitude: 34.05,
        longitude: -118.24,
        name: None,
        address: Some("2800 E Observatory Rd".into()),
    };
    assert_eq!(
        loc.format_text_line(),
        "📍 2800 E Observatory Rd — 34.0500, -118.2400"
    );
}

#[test]
fn location_format_with_coords_only() {
    let loc = Location {
        latitude: 34.05,
        longitude: -118.24,
        name: None,
        address: None,
    };
    assert_eq!(loc.format_text_line(), "📍 34.0500, -118.2400");
}

// Test #2: PreparedMessage tests
#[test]
fn prepared_message_plain_text_with_location() {
    let msg = Message::text("Check this out").with_location(34.05, -118.24);
    let prepared = PreparedMessage::new(&msg);
    let result = prepared.render_body_with_location(ProviderKind::Discord);
    assert!(result.starts_with("Check this out\n📍 "));
    assert!(result.contains("34.0500, -118.2400"));
}

#[test]
fn prepared_message_markdown_with_location() {
    let msg = Message::markdown("**Important**").with_location(34.05, -118.24);
    let prepared = PreparedMessage::new(&msg);
    let result = prepared.render_body_with_location(ProviderKind::Discord);
    assert!(result.contains("\n📍 "));
}

#[test]
fn prepared_message_no_body_has_location() {
    let msg = Message::location(34.05, -118.24);
    let prepared = PreparedMessage::new(&msg);
    let result = prepared.render_body_or_location(ProviderKind::Discord);
    assert_eq!(result, "📍 34.0500, -118.2400");
}

#[test]
fn prepared_message_no_body_no_location() {
    let msg = Message {
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };
    let prepared = PreparedMessage::new(&msg);
    let result = prepared.render_body_or_location(ProviderKind::Discord);
    assert_eq!(result, "");
}

#[test]
fn prepared_message_render_body_for_provider_plain() {
    // Test the render_body_for_provider with plain text
    let msg = Message::text("plain text");
    let prepared = PreparedMessage::new(&msg);
    let result = prepared.render_body_for_provider(ProviderKind::Discord);
    assert_eq!(result, "plain text");
}

#[test]
fn prepared_message_render_body_for_provider_markdown() {
    // Test the render_body_for_provider with markdown
    let msg = Message::markdown("**bold**");
    let prepared = PreparedMessage::new(&msg);
    let result = prepared.render_body_for_provider(ProviderKind::Discord);
    // Should render something (exact format depends on markdown renderer)
    assert!(!result.is_empty());
}

#[test]
fn prepared_message_render_body_for_provider_none() {
    // Test the render_body_for_provider with no body
    let msg = Message {
        body: None,
        attachments: vec![],
        location: None,
        metadata: Default::default(),
    };
    let prepared = PreparedMessage::new(&msg);
    let result = prepared.render_body_for_provider(ProviderKind::Discord);
    assert_eq!(result, "");
}
