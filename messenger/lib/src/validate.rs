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
pub fn validate_message(message: &Message) -> Result<(), MessengerError> {
    if message.is_empty() {
        return Err(MessengerError::InvalidMessage(
            "message has no body, attachments, or location".into(),
        ));
    }
    Ok(())
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
            return Err(MessengerError::InvalidMessage(format!(
                "reply_to provider ({reply_provider}) does not match target provider ({target_provider})"
            )));
        }
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
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "markdown rendering",
            });
        }
        warnings.push(CompatibilityWarning {
            provider,
            feature: "markdown rendering",
        });
    }

    if !normalized_message.attachments.is_empty() && !capabilities.supports_attachments {
        if dispatch.options.compatibility == CompatibilityMode::Strict {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "attachments",
            });
        }
        normalized_message.attachments.clear();
        warnings.push(CompatibilityWarning {
            provider,
            feature: "attachments",
        });
    }

    if normalized_message.location.is_some() && !capabilities.supports_location {
        if dispatch.options.compatibility == CompatibilityMode::Strict {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "location",
            });
        }
        normalized_message.location = None;
        warnings.push(CompatibilityWarning {
            provider,
            feature: "location",
        });
    }

    if normalized_dispatch.reply_to.is_some() && !capabilities.supports_reply {
        if dispatch.options.compatibility == CompatibilityMode::Strict {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "replies",
            });
        }
        normalized_dispatch.reply_to = None;
        warnings.push(CompatibilityWarning {
            provider,
            feature: "replies",
        });
    }

    if normalized_dispatch.options.silent && !capabilities.supports_silent_delivery {
        if dispatch.options.compatibility == CompatibilityMode::Strict {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "silent delivery",
            });
        }
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
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "link preview control",
            });
        }
        normalized_dispatch.options.disable_link_preview = false;
        warnings.push(CompatibilityWarning {
            provider,
            feature: "link preview control",
        });
    }

    for attachment in &normalized_message.attachments {
        validate_attachment_source(attachment, provider)?;
    }

    if normalized_message.is_empty() {
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
        #[cfg(feature = "slack")]
        Target::Slack(_) => ProviderKind::Slack,
        #[cfg(feature = "signal")]
        Target::Signal(_) => ProviderKind::Signal,
        #[cfg(feature = "whatsapp")]
        Target::WhatsApp(_) => ProviderKind::WhatsApp,
        #[cfg(feature = "telegram")]
        Target::Telegram(_) => ProviderKind::Telegram,
        _ => unreachable!("no provider features enabled"),
    }
}
