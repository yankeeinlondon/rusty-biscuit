use std::collections::BTreeMap;

use crate::attachment::{Attachment, AttachmentKind, AttachmentSource};

/// Portable message content, independent of any destination.
#[derive(Debug, Clone)]
pub struct Message {
    pub body: Option<MessageBody>,
    pub attachments: Vec<Attachment>,
    pub location: Option<Location>,
    pub metadata: BTreeMap<String, String>,
}

/// The text content of a message.
#[derive(Debug, Clone)]
pub enum MessageBody {
    Plain(String),
    Markdown(String),
}

/// A geographic location.
#[derive(Debug, Clone)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub name: Option<String>,
    pub address: Option<String>,
}

impl Message {
    /// Create a plain-text message.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            body: Some(MessageBody::Plain(text.into())),
            attachments: Vec::new(),
            location: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Create a Markdown message.
    pub fn markdown(md: impl Into<String>) -> Self {
        Self {
            body: Some(MessageBody::Markdown(md.into())),
            attachments: Vec::new(),
            location: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Create a location-only message.
    pub fn location(lat: f64, lon: f64) -> Self {
        Self {
            body: None,
            attachments: Vec::new(),
            location: Some(Location {
                latitude: lat,
                longitude: lon,
                name: None,
                address: None,
            }),
            metadata: BTreeMap::new(),
        }
    }

    /// Add an attachment to the message.
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Shorthand for adding an image attachment from a path.
    pub fn image(self, path: impl Into<std::path::PathBuf>) -> Self {
        self.attachment(Attachment {
            kind: AttachmentKind::Image,
            source: AttachmentSource::Path(path.into()),
            caption: None,
            alt_text: None,
        })
    }

    /// Add a key-value metadata pair.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check whether this message has any content at all.
    pub fn is_empty(&self) -> bool {
        self.body.is_none() && self.attachments.is_empty() && self.location.is_none()
    }
}
