use crate::{Attachment, AttachmentKind, Message, MessageBody};

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
