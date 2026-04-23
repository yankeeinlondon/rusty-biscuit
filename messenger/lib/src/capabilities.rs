use std::collections::BTreeSet;

use crate::attachment::AttachmentKind;

/// Describes what a provider supports.
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    pub supports_markdown_rendering: bool,
    pub supports_reply: bool,
    pub supported_attachment_kinds: BTreeSet<AttachmentKind>,
    pub supports_location: bool,
    pub supports_silent_delivery: bool,
    pub supports_link_preview_control: bool,
}

impl CapabilitySet {
    /// All capabilities enabled, including every known attachment kind.
    pub fn all() -> Self {
        Self {
            supports_markdown_rendering: true,
            supports_reply: true,
            supported_attachment_kinds: BTreeSet::from([
                AttachmentKind::Image,
                AttachmentKind::Audio,
                AttachmentKind::Video,
                AttachmentKind::Document,
                AttachmentKind::Binary,
            ]),
            supports_location: true,
            supports_silent_delivery: true,
            supports_link_preview_control: true,
        }
    }

    /// No capabilities enabled.
    pub fn none() -> Self {
        Self {
            supports_markdown_rendering: false,
            supports_reply: false,
            supported_attachment_kinds: BTreeSet::new(),
            supports_location: false,
            supports_silent_delivery: false,
            supports_link_preview_control: false,
        }
    }
}
