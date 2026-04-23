use std::collections::BTreeMap;

use crate::attachment::{Attachment, AttachmentKind, AttachmentSource};

/// Portable message content, independent of any destination.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub title: Option<String>,
    pub body: Option<MessageBody>,
    pub attachments: Vec<Attachment>,
    pub location: Option<Location>,
    pub metadata: BTreeMap<String, String>,
}

/// The text content of a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageBody {
    Plain(String),
    Markdown(String),
}

/// A geographic location.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub name: Option<String>,
    pub address: Option<String>,
}

impl Location {
    /// Format as a text line for providers without native location support.
    ///
    /// ## Examples
    ///
    /// - `📍 Griffith Observatory (2800 E Observatory Rd) — 34.0500, -118.2400`
    /// - `📍 Griffith Observatory — 34.0500, -118.2400`
    /// - `📍 34.0500, -118.2400`
    pub fn format_text_line(&self) -> String {
        let coords = format!("{:.4}, {:.4}", self.latitude, self.longitude);
        match (&self.name, &self.address) {
            (Some(name), Some(addr)) => format!("📍 {name} ({addr}) — {coords}"),
            (Some(name), None) => format!("📍 {name} — {coords}"),
            (None, Some(addr)) => format!("📍 {addr} — {coords}"),
            (None, None) => format!("📍 {coords}"),
        }
    }
}

impl Message {
    /// Create a plain-text message.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            title: None,
            body: Some(MessageBody::Plain(text.into())),
            attachments: Vec::new(),
            location: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Create a Markdown message.
    pub fn markdown(md: impl Into<String>) -> Self {
        Self {
            title: None,
            body: Some(MessageBody::Markdown(md.into())),
            attachments: Vec::new(),
            location: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Create a location-only message.
    pub fn location(lat: f64, lon: f64) -> Self {
        Self {
            title: None,
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

    /// Set the portable title for this message.
    ///
    /// The title is used by providers that distinguish a summary line from the body,
    /// such as desktop notifications. Providers without a native title concept ignore it.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Attach a location to the message.
    pub fn with_location(mut self, lat: f64, lon: f64) -> Self {
        self.location = Some(Location {
            latitude: lat,
            longitude: lon,
            name: None,
            address: None,
        });
        self
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

    /// Shorthand for adding a document attachment from a path.
    pub fn file(self, path: impl Into<std::path::PathBuf>) -> Self {
        self.attachment(Attachment::file(path))
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
