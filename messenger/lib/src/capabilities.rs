/// Describes what a provider supports.
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    pub supports_markdown_rendering: bool,
    pub supports_reply: bool,
    pub supports_attachments: bool,
    pub supports_location: bool,
    pub supports_silent_delivery: bool,
    pub supports_link_preview_control: bool,
}

impl CapabilitySet {
    /// All capabilities enabled.
    pub fn all() -> Self {
        Self {
            supports_markdown_rendering: true,
            supports_reply: true,
            supports_attachments: true,
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
            supports_attachments: false,
            supports_location: false,
            supports_silent_delivery: false,
            supports_link_preview_control: false,
        }
    }
}
