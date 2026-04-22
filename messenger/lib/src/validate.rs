use std::fmt;
use std::fs::File;

use crate::attachment::AttachmentSource;
use crate::capabilities::CapabilitySet;
use crate::dispatch::{CompatibilityMode, Dispatch};
use crate::error::MessengerError;
use crate::message::Message;
use crate::receipt::ProviderKind;
use crate::target::Target;

/// Validate that a message has content.
///
/// Uses the conservative rule: body, attachments, or location must be
/// present. The `title` field alone is **not** enough for non-desktop
/// providers. Use [`validate_message_for_provider`] in provider-aware
/// flows that allow title-only messages for the desktop provider.
pub fn validate_message(message: &Message) -> Result<(), MessengerError> {
    if message.is_empty() {
        return Err(MessengerError::InvalidMessage(
            "message has no body, attachments, or location".into(),
        ));
    }
    Ok(())
}

/// Provider-aware message validity check.
///
/// Matches [`validate_message`] for every provider except
/// [`ProviderKind::Desktop`], where a title-only message is permitted.
/// Desktop notifications treat `title` as first-class content: a "Deploy
/// finished" toast with no body is valid and should not error out before
/// a backend ever sees it.
pub fn validate_message_for_provider(
    message: &Message,
    provider: ProviderKind,
) -> Result<(), MessengerError> {
    #[cfg(feature = "desktop")]
    if provider == ProviderKind::Desktop {
        if message.is_empty() && message.title.is_none() {
            return Err(MessengerError::InvalidMessage(
                "message has no title, body, attachments, or location".into(),
            ));
        }
        return Ok(());
    }
    let _ = provider;
    validate_message(message)
}

/// A best-effort compatibility warning emitted when a feature is dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityWarning {
    pub provider: ProviderKind,
    pub feature: &'static str,
}

impl fmt::Display for CompatibilityWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "⚠️ the {} feature is not supported on {} and will be dropped",
            self.feature, self.provider
        )
    }
}

/// A normalized send request after applying provider capability rules.
#[derive(Debug, Clone)]
pub struct NormalizedDispatch {
    pub dispatch: Dispatch,
    pub message: Message,
    pub warnings: Vec<CompatibilityWarning>,
}

/// Validate dispatch consistency and capability requirements.
pub fn validate_dispatch(
    dispatch: &Dispatch,
    message: &Message,
    capabilities: &CapabilitySet,
    provider: ProviderKind,
) -> Result<(), MessengerError> {
    normalize_dispatch(dispatch, message, capabilities, provider).map(|_| ())
}

/// Normalize a message/dispatch pair for a specific provider.
#[tracing::instrument(skip_all, fields(provider = %provider, mode = ?dispatch.options.compatibility))]
pub fn normalize_dispatch(
    dispatch: &Dispatch,
    message: &Message,
    capabilities: &CapabilitySet,
    provider: ProviderKind,
) -> Result<NormalizedDispatch, MessengerError> {
    // Check reply_to provider matches target provider
    if let Some(ref reply_ref) = dispatch.reply_to {
        let target_provider = target_provider_kind(&dispatch.target);
        let reply_provider = reply_ref.provider_kind();
        if target_provider != reply_provider {
            tracing::warn!(
                target_provider = %target_provider,
                reply_provider = %reply_provider,
                "reply reference provider does not match target"
            );
            return Err(MessengerError::InvalidMessage(format!(
                "reply_to provider ({reply_provider}) does not match target provider ({target_provider})"
            )));
        }
    }

    // Discord webhooks cannot carry a reply context. Enforce this before
    // normalization so `BestEffort` mode does not silently drop the field and
    // so `plan_send()` never issues a network call for this misuse.
    if provider == ProviderKind::DiscordWebhook && dispatch.reply_to.is_some() {
        tracing::warn!(
            feature = "replies",
            "Discord webhooks do not support replies; failing plan-time"
        );
        return Err(MessengerError::UnsupportedFeature {
            provider,
            feature: "replies",
        });
    }

    let mut normalized_dispatch = dispatch.clone();
    let mut normalized_message = message.clone();
    let mut warnings = Vec::new();

    let has_markdown = matches!(
        normalized_message.body,
        Some(crate::message::MessageBody::Markdown(_))
    );
    if has_markdown && !capabilities.supports_markdown_rendering {
        if dispatch.options.compatibility == CompatibilityMode::Strict {
            tracing::warn!(
                feature = "markdown rendering",
                "strict mode rejected unsupported feature"
            );
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "markdown rendering",
            });
        }
        tracing::debug!(
            feature = "markdown rendering",
            "provider does not support native markdown rendering"
        );
        warnings.push(CompatibilityWarning {
            provider,
            feature: "markdown rendering",
        });
    }

    if !normalized_message.attachments.is_empty() {
        let has_unsupported_attachment = normalized_message.attachments.iter().any(|attachment| {
            !capabilities
                .supported_attachment_kinds
                .contains(&attachment.kind)
        });

        if has_unsupported_attachment {
            if dispatch.options.compatibility == CompatibilityMode::Strict {
                tracing::warn!(
                    feature = "attachments",
                    "strict mode rejected unsupported feature"
                );
                return Err(MessengerError::UnsupportedFeature {
                    provider,
                    feature: "attachments",
                });
            }
            tracing::debug!(
                feature = "attachments",
                "dropping unsupported attachment kinds"
            );
            normalized_message.attachments.retain(|attachment| {
                capabilities
                    .supported_attachment_kinds
                    .contains(&attachment.kind)
            });
            warnings.push(CompatibilityWarning {
                provider,
                feature: "attachments",
            });
        }
    }

    if normalized_message.location.is_some() && !capabilities.supports_location {
        if dispatch.options.compatibility == CompatibilityMode::Strict {
            tracing::warn!(
                feature = "location",
                "strict mode rejected unsupported feature"
            );
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "location",
            });
        }
        tracing::debug!(feature = "location", "dropping unsupported feature");
        normalized_message.location = None;
        warnings.push(CompatibilityWarning {
            provider,
            feature: "location",
        });
    }

    if normalized_dispatch.reply_to.is_some() && !capabilities.supports_reply {
        if dispatch.options.compatibility == CompatibilityMode::Strict {
            tracing::warn!(
                feature = "replies",
                "strict mode rejected unsupported feature"
            );
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "replies",
            });
        }
        tracing::debug!(feature = "replies", "dropping unsupported feature");
        normalized_dispatch.reply_to = None;
        warnings.push(CompatibilityWarning {
            provider,
            feature: "replies",
        });
    }

    if normalized_dispatch.options.silent && !capabilities.supports_silent_delivery {
        if dispatch.options.compatibility == CompatibilityMode::Strict {
            tracing::warn!(
                feature = "silent delivery",
                "strict mode rejected unsupported feature"
            );
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "silent delivery",
            });
        }
        tracing::debug!(feature = "silent delivery", "dropping unsupported feature");
        normalized_dispatch.options.silent = false;
        warnings.push(CompatibilityWarning {
            provider,
            feature: "silent delivery",
        });
    }

    if normalized_dispatch.options.disable_link_preview
        && !capabilities.supports_link_preview_control
    {
        if dispatch.options.compatibility == CompatibilityMode::Strict {
            tracing::warn!(
                feature = "link preview control",
                "strict mode rejected unsupported feature"
            );
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "link preview control",
            });
        }
        tracing::debug!(
            feature = "link preview control",
            "dropping unsupported feature"
        );
        normalized_dispatch.options.disable_link_preview = false;
        warnings.push(CompatibilityWarning {
            provider,
            feature: "link preview control",
        });
    }

    tracing::trace!(
        attachment_count = normalized_message.attachments.len(),
        "validating attachment sources"
    );
    for (index, attachment) in normalized_message.attachments.iter().enumerate() {
        tracing::trace!(attachment_index = index, kind = ?attachment.kind, "validating attachment");
        validate_attachment_source(attachment, provider)?;
    }

    if is_undeliverable_after_normalization(&normalized_message, provider) {
        tracing::warn!("normalization dropped all deliverable content");
        return Err(MessengerError::InvalidMessage(format!(
            "{provider} cannot deliver this message after dropping unsupported features"
        )));
    }

    Ok(NormalizedDispatch {
        dispatch: normalized_dispatch,
        message: normalized_message,
        warnings,
    })
}

fn is_undeliverable_after_normalization(message: &Message, provider: ProviderKind) -> bool {
    #[cfg(feature = "desktop")]
    if provider == ProviderKind::Desktop {
        return message.is_empty() && message.title.is_none();
    }
    let _ = provider;
    message.is_empty()
}

fn validate_attachment_source(
    attachment: &crate::attachment::Attachment,
    provider: ProviderKind,
) -> Result<(), MessengerError> {
    match &attachment.source {
        AttachmentSource::Path(path) => {
            if !path.exists() {
                return Err(MessengerError::InvalidMessage(format!(
                    "{provider} attachment path does not exist: {}",
                    path.display()
                )));
            }

            if !path.is_file() {
                return Err(MessengerError::InvalidMessage(format!(
                    "{provider} attachment path is not a file: {}",
                    path.display()
                )));
            }

            File::open(path).map_err(|error| {
                MessengerError::InvalidMessage(format!(
                    "{provider} attachment path is not readable ({}): {error}",
                    path.display()
                ))
            })?;
        }
        AttachmentSource::Url(url) => {
            if url.trim().is_empty() {
                return Err(MessengerError::InvalidMessage(format!(
                    "{provider} attachment URL is empty"
                )));
            }
        }
        AttachmentSource::Bytes {
            filename,
            mime_type,
            ..
        } => {
            if filename.trim().is_empty() {
                return Err(MessengerError::InvalidMessage(format!(
                    "{provider} attachment filename is empty"
                )));
            }
            if mime_type.trim().is_empty() {
                return Err(MessengerError::InvalidMessage(format!(
                    "{provider} attachment mime type is empty"
                )));
            }
        }
        AttachmentSource::ProviderFileId(file_id) => {
            if file_id.trim().is_empty() || file_id.contains('\n') || file_id.contains('\r') {
                return Err(MessengerError::InvalidMessage(format!(
                    "{provider} provider file id is malformed"
                )));
            }
        }
    }

    Ok(())
}

/// Resolve which provider a target maps to.
#[allow(unreachable_patterns)]
pub fn target_provider_kind(target: &Target) -> ProviderKind {
    match target {
        #[cfg(feature = "discord")]
        Target::Discord(_) => ProviderKind::Discord,
        #[cfg(feature = "discord")]
        Target::DiscordWebhook(_) => ProviderKind::DiscordWebhook,
        #[cfg(feature = "slack")]
        Target::Slack(_) => ProviderKind::Slack,
        #[cfg(feature = "slack")]
        Target::SlackWebhook(_) => ProviderKind::SlackWebhook,
        #[cfg(feature = "signal")]
        Target::Signal(_) => ProviderKind::Signal,
        #[cfg(feature = "whatsapp")]
        Target::WhatsApp(_) => ProviderKind::WhatsApp,
        #[cfg(feature = "telegram")]
        Target::Telegram(_) => ProviderKind::Telegram,
        #[cfg(feature = "desktop")]
        Target::Desktop(_) => ProviderKind::Desktop,
        #[cfg(feature = "apns")]
        Target::Apns(_) => ProviderKind::Apns,
        #[cfg(feature = "fcm")]
        Target::Fcm(_) => ProviderKind::Fcm,
        _ => unreachable!("no provider features enabled"),
    }
}
