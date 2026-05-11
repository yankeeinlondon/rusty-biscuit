use std::collections::BTreeMap;

use secrecy::{ExposeSecret, SecretString};
use twilight_http::Client;
use twilight_model::channel::message::Embed;
use twilight_model::http::attachment::Attachment as DiscordAttachment;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, MessageMarker};
use twilight_util::builder::embed::EmbedBuilder;

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::prepared::PreparedMessage;
use crate::receipt::{MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

/// Configuration for the Discord provider.
pub struct DiscordConfig {
    pub bot_token: SecretString,
}

/// Install the `ring` crypto provider as the process-wide rustls default.
///
/// `twilight-http` uses `rustls` internally and rustls 0.23 requires a
/// process-wide `CryptoProvider` to be installed before any TLS handshake.
/// Auto-install only happens when exactly one of `aws-lc-rs` or `ring` is
/// enabled at compile time, which cargo feature unification across transitive
/// crates can break. Installing explicitly is deterministic and idempotent —
/// `install_default()` returns `Err` if a provider is already installed,
/// which we intentionally swallow.
fn install_default_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Discord provider adapter using the Discord REST API via twilight-http.
pub struct DiscordProvider {
    client: Client,
}

/// Result of preparing the Discord request body shape for a message.
///
/// `embed` is `Some` only for `Summarized` bodies; otherwise the rendered
/// content is sent in the top-level `content` field as today.
#[derive(Debug)]
struct DiscordPayload {
    content: Option<String>,
    embed: Option<Embed>,
}

impl DiscordProvider {
    pub fn new(config: DiscordConfig) -> Self {
        install_default_crypto_provider();
        let client = Client::new(config.bot_token.expose_secret().to_string());
        Self { client }
    }

    fn build_payload(message: &PreparedMessage) -> DiscordPayload {
        let summary = message.render_summary();
        let rich = message.render_rich(ProviderKind::Discord);
        let location_line = message.location().map(|l| l.format_text_line());

        match rich {
            Some(mut rich_md) => {
                if let Some(loc) = location_line {
                    if !rich_md.is_empty() {
                        rich_md.push('\n');
                    }
                    rich_md.push_str(&loc);
                }
                let embed = EmbedBuilder::new()
                    .description(rich_md)
                    .build();
                let content = if summary.is_empty() { None } else { Some(summary) };
                DiscordPayload { content, embed: Some(embed) }
            }
            None => {
                let content = message.render_body_with_location(ProviderKind::Discord);
                let content = if content.is_empty() { None } else { Some(content) };
                DiscordPayload { content, embed: None }
            }
        }
    }

    fn build_attachment(
        attachment: &crate::attachment::Attachment,
        id: u64,
    ) -> Result<DiscordAttachment, MessengerError> {
        let (filename, file) = super::attachment_helpers::read_local_or_bytes_attachment(
            &attachment.source,
            "Discord",
        )?;

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

#[async_trait::async_trait]
impl super::Provider for DiscordProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Discord
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_markdown_rendering: true,
            supports_reply: true,
            supported_attachment_kinds: std::collections::BTreeSet::from([
                crate::AttachmentKind::Image,
                crate::AttachmentKind::Audio,
                crate::AttachmentKind::Video,
                crate::AttachmentKind::Document,
                crate::AttachmentKind::Binary,
            ]),
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

        // Build the message body shape (content + optional embed)
        let payload = Self::build_payload(message);
        let attachments = Self::build_attachments(message)?;
        let attachment_kinds: Vec<_> = message
            .attachments()
            .iter()
            .map(|attachment| &attachment.kind)
            .collect();
        tracing::debug!(
            has_reply = dispatch.reply_to.is_some(),
            content_len = payload.content.as_deref().map(str::len).unwrap_or(0),
            has_embed = payload.embed.is_some(),
            attachment_count = attachments.len(),
            "sending Discord message"
        );
        tracing::trace!(attachment_kinds = ?attachment_kinds, "built Discord attachments");

        // Build the message request
        let mut req = self.client.create_message(channel_id);

        if let Some(ref content) = payload.content {
            req = req.content(content);
        }
        let embeds_storage;
        if let Some(embed) = payload.embed {
            embeds_storage = [embed];
            req = req.embeds(&embeds_storage);
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
            ProviderKind::Discord.transport_error(e)
        })?;

        let msg = response.model().await.map_err(|e| {
            tracing::warn!(error = %e, "Discord response decode failed");
            ProviderKind::Discord.transport_error(e)
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

        let caps = provider.capabilities();
        assert!(
            caps.supported_attachment_kinds
                .contains(&crate::AttachmentKind::Image)
        );
        assert!(
            caps.supported_attachment_kinds
                .contains(&crate::AttachmentKind::Document)
        );
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

    use crate::Message;
    use crate::PreparedMessage;

    #[test]
    fn build_payload_plain_body_uses_content_only() {
        let msg = Message::text("hello");
        let prepared = PreparedMessage::new(&msg);
        let p = DiscordProvider::build_payload(&prepared);
        assert_eq!(p.content.as_deref(), Some("hello"));
        assert!(p.embed.is_none());
    }

    #[test]
    fn build_payload_markdown_body_uses_content_only() {
        let msg = Message::markdown("**bold**");
        let prepared = PreparedMessage::new(&msg);
        let p = DiscordProvider::build_payload(&prepared);
        assert_eq!(p.content.as_deref(), Some("**bold**"));
        assert!(p.embed.is_none());
    }

    #[test]
    fn build_payload_summarized_body_splits_into_content_and_embed() {
        let msg = Message::summarized("plain banner", "**rich** body");
        let prepared = PreparedMessage::new(&msg);
        let p = DiscordProvider::build_payload(&prepared);
        assert_eq!(p.content.as_deref(), Some("plain banner"));
        let embed = p.embed.expect("expected embed for Summarized body");
        let desc = embed.description.expect("embed should have description");
        assert!(desc.contains("**rich**"));
        assert!(desc.contains("body"));
    }

    #[test]
    fn build_payload_summarized_with_empty_summary_omits_content() {
        let msg = Message::summarized("", "**rich**");
        let prepared = PreparedMessage::new(&msg);
        let p = DiscordProvider::build_payload(&prepared);
        assert!(p.content.is_none());
        assert!(p.embed.is_some());
    }

    #[test]
    fn build_payload_summarized_appends_location_to_embed_description() {
        let msg = Message::summarized("s", "body").with_location(34.05, -118.24);
        let prepared = PreparedMessage::new(&msg);
        let p = DiscordProvider::build_payload(&prepared);
        let desc = p.embed.unwrap().description.unwrap();
        assert!(desc.contains("body"));
        assert!(desc.contains("📍"));
        assert!(desc.contains("34.0500"));
    }

    #[test]
    fn build_payload_legacy_appends_location_to_content() {
        let msg = Message::markdown("**body**").with_location(34.05, -118.24);
        let prepared = PreparedMessage::new(&msg);
        let p = DiscordProvider::build_payload(&prepared);
        let content = p.content.unwrap();
        assert!(content.contains("**body**"));
        assert!(content.contains("📍"));
    }
}
