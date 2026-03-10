use std::path::PathBuf;

use bytes::Bytes;

/// A file or media attachment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub kind: AttachmentKind,
    pub source: AttachmentSource,
    pub caption: Option<String>,
    pub alt_text: Option<String>,
}

/// The type of attachment content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentKind {
    Image,
    Audio,
    Video,
    Document,
    Binary,
}

/// Where the attachment data comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentSource {
    Path(PathBuf),
    Url(String),
    Bytes {
        filename: String,
        mime_type: String,
        data: Bytes,
    },
    ProviderFileId(String),
}

impl Attachment {
    /// Create an image attachment from a file path.
    pub fn image(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: AttachmentKind::Image,
            source: AttachmentSource::Path(path.into()),
            caption: None,
            alt_text: None,
        }
    }

    /// Create a document attachment from a file path.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: AttachmentKind::Document,
            source: AttachmentSource::Path(path.into()),
            caption: None,
            alt_text: None,
        }
    }

    /// Set a caption on this attachment.
    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Set alt text on this attachment.
    pub fn alt_text(mut self, alt_text: impl Into<String>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}
