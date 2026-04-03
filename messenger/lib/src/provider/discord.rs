use std::collections::BTreeMap;
use std::fs;

use secrecy::{ExposeSecret, SecretString};
use twilight_http::Client;
use twilight_model::http::attachment::Attachment as DiscordAttachment;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, MessageMarker};

use crate::attachment::AttachmentSource;
use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::message::MessageBody;
use crate::prepared::PreparedMessage;
use crate::receipt::{MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

/// Configuration for the Discord provider.
pub struct DiscordConfig {
    pub bot_token: SecretString,
}

/// Discord provider adapter using the Discord REST API via twilight-http.
pub struct DiscordProvider {
    client: Client,
}

impl DiscordProvider {
    pub fn new(config: DiscordConfig) -> Self {
        let client = Client::new(config.bot_token.expose_secret().to_string());
        Self { client }
    }

    fn build_attachment(
        attachment: &crate::attachment::Attachment,
        id: u64,
    ) -> Result<DiscordAttachment, MessengerError> {
        let (filename, file) = match &attachment.source {
            AttachmentSource::Path(path) => {
                let filename = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        MessengerError::InvalidMessage(format!(
                            "Discord attachment path has no valid filename: {}",
                            path.display()
                        ))
                    })?
                    .to_owned();
                let file = fs::read(path).map_err(|error| {
                    MessengerError::InvalidMessage(format!(
                        "Discord attachment path is not readable ({}): {error}",
                        path.display()
                    ))
                })?;

                (filename, file)
            }
            AttachmentSource::Bytes { filename, data, .. } => (filename.clone(), data.to_vec()),
            AttachmentSource::Url(_) => {
                return Err(MessengerError::InvalidMessage(
                    "Discord attachments must come from a local path or bytes payload".into(),
                ));
            }
            AttachmentSource::ProviderFileId(_) => {
                return Err(MessengerError::InvalidMessage(
                    "Discord does not support provider file ID attachments".into(),
                ));
            }
        };

        let mut discord_attachment = DiscordAttachment::from_bytes(filename, file, id);

        if let Some(description) = attachment
            .alt_text
            .clone()
            .or_else(|| attachment.caption.clone())
        {
            discord_attachment.description(description);
        }

        Ok(discord_attachment)
    }

    fn build_attachments(
        message: &PreparedMessage,
    ) -> Result<Vec<DiscordAttachment>, MessengerError> {
        message
            .attachments()
            .iter()
            .enumerate()
            .map(|(index, attachment)| Self::build_attachment(attachment, index as u64))
            .collect()
    }

    fn parse_channel_id(s: &str) -> Result<Id<ChannelMarker>, MessengerError> {
        s.parse::<u64>()
            .map(Id::new)
            .map_err(|_| MessengerError::InvalidMessage(format!("invalid Discord channel ID: {s}")))
    }

    fn parse_message_id(s: &str) -> Result<Id<MessageMarker>, MessengerError> {
        s.parse::<u64>()
            .map(Id::new)
            .map_err(|_| MessengerError::InvalidMessage(format!("invalid Discord message ID: {s}")))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use bytes::Bytes;
    use tempfile::NamedTempFile;

    use super::DiscordProvider;
    use crate::Attachment;
    use crate::AttachmentSource;
    use crate::Provider;

    #[test]
    fn rejects_invalid_channel_id() {
        let err = DiscordProvider::parse_channel_id("not-a-number").unwrap_err();
        assert!(matches!(
            err,
            crate::MessengerError::InvalidMessage(message)
            if message.contains("invalid Discord channel ID")
        ));
    }

    #[test]
    fn rejects_invalid_message_id() {
        let err = DiscordProvider::parse_message_id("not-a-number").unwrap_err();
        assert!(matches!(
            err,
            crate::MessengerError::InvalidMessage(message)
            if message.contains("invalid Discord message ID")
        ));
    }

    #[tokio::test]
    async fn supports_attachments() {
        let provider = DiscordProvider::new(super::DiscordConfig {
            bot_token: secrecy::SecretString::new("token".into()),
        });

        assert!(provider.capabilities().supports_attachments);
    }

    #[test]
    fn converts_path_attachments_to_discord_uploads() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "hello from disk").unwrap();

        let path = file.path().to_path_buf();
        let filename = path.file_name().unwrap().to_string_lossy().into_owned();
        let attachment = Attachment::image(&path)
            .caption("chart caption")
            .alt_text("chart alt");

        let discord_attachment = DiscordProvider::build_attachment(&attachment, 7).unwrap();

        assert_eq!(discord_attachment.id, 7);
        assert_eq!(discord_attachment.filename, filename);
        assert_eq!(discord_attachment.file, b"hello from disk");
        assert_eq!(discord_attachment.description.as_deref(), Some("chart alt"));
    }

    #[test]
    fn converts_byte_attachments_to_discord_uploads() {
        let attachment = Attachment {
            kind: crate::AttachmentKind::Document,
            source: AttachmentSource::Bytes {
                filename: "report.txt".into(),
                mime_type: "text/plain".into(),
                data: Bytes::from_static(b"ok"),
            },
            caption: Some("report caption".into()),
            alt_text: None,
        };

        let discord_attachment = DiscordProvider::build_attachment(&attachment, 2).unwrap();

        assert_eq!(discord_attachment.id, 2);
        assert_eq!(discord_attachment.filename, "report.txt");
        assert_eq!(discord_attachment.file, b"ok");
        assert_eq!(
            discord_attachment.description.as_deref(),
            Some("report caption")
        );
    }

    #[test]
    fn rejects_url_attachments() {
        let attachment = Attachment {
            kind: crate::AttachmentKind::Image,
            source: AttachmentSource::Url("https://example.com/cat.png".into()),
            caption: None,
            alt_text: None,
        };

        let err = DiscordProvider::build_attachment(&attachment, 0).unwrap_err();
        assert!(matches!(
            err,
            crate::MessengerError::InvalidMessage(message)
            if message.contains("local path or bytes payload")
        ));
    }

    #[test]
    fn rejects_provider_file_id_attachments() {
        let attachment = Attachment {
            kind: crate::AttachmentKind::Document,
            source: AttachmentSource::ProviderFileId("abc123".into()),
            caption: None,
            alt_text: None,
        };

        let err = DiscordProvider::build_attachment(&attachment, 0).unwrap_err();
        assert!(matches!(
            err,
            crate::MessengerError::InvalidMessage(message)
            if message.contains("provider file ID")
        ));
    }
}

#[async_trait::async_trait]
impl super::Provider for DiscordProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Discord
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_markdown_rendering: true,
            supports_reply: true,
            supports_attachments: true,
            supports_location: true,
            supports_silent_delivery: false,
            supports_link_preview_control: false,
        }
    }

    #[tracing::instrument(skip_all, fields(provider = "discord", channel = tracing::field::Empty))]
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        let channel_id = match &dispatch.target {
            Target::Discord(t) => Self::parse_channel_id(&t.channel_id)?,
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected Discord target".into(),
                ));
            }
        };
        tracing::Span::current().record("channel", tracing::field::display(channel_id));

        // Render the message body (with location text fallback)
        let content = match message.body() {
            Some(MessageBody::Plain(_)) | Some(MessageBody::Markdown(_)) => {
                message.render_body_with_location(ProviderKind::Discord)
            }
            None if message.location().is_some() => {
                message.render_body_with_location(ProviderKind::Discord)
            }
            None => String::new(),
        };
        let attachments = Self::build_attachments(message)?;
        let attachment_kinds: Vec<_> = message
            .attachments()
            .iter()
            .map(|attachment| &attachment.kind)
            .collect();
        tracing::debug!(
            has_reply = dispatch.reply_to.is_some(),
            content_len = content.len(),
            attachment_count = attachments.len(),
            "sending Discord message"
        );
        tracing::trace!(attachment_kinds = ?attachment_kinds, "built Discord attachments");

        // Build the message request
        let mut req = self.client.create_message(channel_id);

        if !content.is_empty() {
            req = req.content(&content);
        }
        if !attachments.is_empty() {
            req = req.attachments(&attachments);
        }

        // Handle reply_to
        if let Some(MessageRef::Discord { message_id, .. }) = &dispatch.reply_to {
            let msg_id = Self::parse_message_id(message_id)?;
            req = req.reply(msg_id);
        }
        // Execute the request
        let response = req.await.map_err(|e| {
            tracing::warn!(error = %e, "Discord request failed");
            MessengerError::Transport {
                provider: ProviderKind::Discord,
                message: e.to_string(),
            }
        })?;

        let msg = response.model().await.map_err(|e| {
            tracing::warn!(error = %e, "Discord response decode failed");
            MessengerError::Transport {
                provider: ProviderKind::Discord,
                message: e.to_string(),
            }
        })?;
        tracing::debug!(raw_id = %msg.id, "Discord message sent");

        Ok(SendReceipt {
            provider: ProviderKind::Discord,
            message_ref: MessageRef::Discord {
                channel_id: channel_id.to_string(),
                message_id: msg.id.to_string(),
            },
            raw_id: msg.id.to_string(),
            metadata: BTreeMap::new(),
        })
    }
}
