use crate::attachment::Attachment;
use crate::markdown::{ast::RichNode, parse::parse_markdown, render_nodes_for_provider};
use crate::message::{Location, Message, MessageBody};
use crate::receipt::ProviderKind;

/// A message with lazily reusable derived state for provider sends.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct PreparedMessage {
    message: Message,
    markdown_nodes: Option<Vec<RichNode>>,
}

impl PreparedMessage {
    /// Create a prepared message from a portable message payload.
    pub fn new(message: &Message) -> Self {
        let markdown_nodes = match &message.body {
            Some(MessageBody::Markdown(markdown)) => Some(parse_markdown(markdown)),
            _ => None,
        };

        Self {
            message: message.clone(),
            markdown_nodes,
        }
    }

    /// Return the original message payload.
    pub fn original(&self) -> &Message {
        &self.message
    }

    /// Return the message body, if any.
    pub fn body(&self) -> Option<&MessageBody> {
        self.message.body.as_ref()
    }

    /// Return the attachments.
    pub fn attachments(&self) -> &[Attachment] {
        &self.message.attachments
    }

    /// Return the location payload, if any.
    pub fn location(&self) -> Option<&Location> {
        self.message.location.as_ref()
    }

    /// Render the body for a specific provider.
    pub fn render_body_for_provider(&self, provider: ProviderKind) -> String {
        match (&self.message.body, &self.markdown_nodes) {
            (Some(MessageBody::Plain(text)), _) => text.clone(),
            (Some(MessageBody::Markdown(_)), Some(nodes)) => {
                render_nodes_for_provider(nodes, provider)
            }
            (Some(MessageBody::Markdown(markdown)), None) => {
                crate::markdown::render_for_provider(markdown, provider)
            }
            (None, _) => String::new(),
        }
    }
}
