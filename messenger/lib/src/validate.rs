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

/// Validate dispatch consistency and capability requirements.
pub fn validate_dispatch(
    dispatch: &Dispatch,
    message: &Message,
    capabilities: &CapabilitySet,
    provider: ProviderKind,
) -> Result<(), MessengerError> {
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

    // In strict mode, check capabilities
    if dispatch.options.compatibility == CompatibilityMode::Strict {
        if message.body.is_some()
            && matches!(message.body, Some(crate::message::MessageBody::Markdown(_)))
            && !capabilities.supports_markdown_rendering
        {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "markdown rendering",
            });
        }
        if !message.attachments.is_empty() && !capabilities.supports_attachments {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "attachments",
            });
        }
        if message.location.is_some() && !capabilities.supports_location {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "location",
            });
        }
        if dispatch.options.silent && !capabilities.supports_silent_delivery {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "silent delivery",
            });
        }
        if dispatch.options.disable_link_preview && !capabilities.supports_link_preview_control {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "link preview control",
            });
        }
        if dispatch.reply_to.is_some() && !capabilities.supports_reply {
            return Err(MessengerError::UnsupportedFeature {
                provider,
                feature: "replies",
            });
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
