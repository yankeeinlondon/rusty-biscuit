#[cfg(feature = "discord")]
pub mod discord;
#[cfg(feature = "signal")]
pub mod signal;
#[cfg(feature = "slack")]
pub mod slack;
#[cfg(feature = "telegram")]
pub mod telegram;
#[cfg(feature = "whatsapp")]
pub mod whatsapp;

use std::collections::HashMap;

use futures::future::join_all;

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::message::Message;
use crate::prepared::PreparedMessage;
use crate::receipt::{ProviderKind, SendReceipt};
use crate::validate::{target_provider_kind, validate_dispatch, validate_message};

/// A messaging provider that can send messages to a specific platform.
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn capabilities(&self) -> CapabilitySet;
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError>;

    async fn send(
        &self,
        dispatch: &Dispatch,
        message: &Message,
    ) -> Result<SendReceipt, MessengerError> {
        let prepared = PreparedMessage::new(message);
        self.send_prepared(dispatch, &prepared).await
    }
}

/// The main coordinator that routes dispatches to registered providers.
pub struct Messenger {
    providers: HashMap<ProviderKind, Box<dyn Provider>>,
}

impl Messenger {
    /// Create a new messenger with no registered providers.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider. Replaces any existing provider of the same kind.
    pub fn register(&mut self, provider: Box<dyn Provider>) {
        self.providers.insert(provider.kind(), provider);
    }

    /// Send a message to a single target.
    pub async fn send(
        &self,
        dispatch: Dispatch,
        message: &Message,
    ) -> Result<SendReceipt, MessengerError> {
        let prepared = PreparedMessage::new(message);
        self.send_prepared(dispatch, &prepared).await
    }

    async fn send_prepared(
        &self,
        dispatch: Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        validate_message(message.original())?;

        let provider_kind = target_provider_kind(&dispatch.target);
        let provider = self
            .providers
            .get(&provider_kind)
            .ok_or(MessengerError::MissingConfiguration {
                provider: provider_kind,
                field: "provider registration",
            })?;

        validate_dispatch(
            &dispatch,
            message.original(),
            &provider.capabilities(),
            provider_kind,
        )?;

        provider.send_prepared(&dispatch, message).await
    }

    /// Send a message to multiple targets, collecting all results.
    pub async fn send_many(
        &self,
        dispatches: Vec<Dispatch>,
        message: &Message,
    ) -> Vec<Result<SendReceipt, MessengerError>> {
        let prepared = PreparedMessage::new(message);
        join_all(
            dispatches
                .into_iter()
                .map(|dispatch| self.send_prepared(dispatch, &prepared)),
        )
        .await
    }
}

impl Default for Messenger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipt::MessageRef;
    use crate::target::Target;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct MockProvider {
        send_count: AtomicU32,
        should_fail: bool,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                send_count: AtomicU32::new(0),
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                send_count: AtomicU32::new(0),
                should_fail: true,
            }
        }
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Discord
        }

        fn capabilities(&self) -> CapabilitySet {
            CapabilitySet::all()
        }

        async fn send_prepared(
            &self,
            _dispatch: &Dispatch,
            _message: &PreparedMessage,
        ) -> Result<SendReceipt, MessengerError> {
            self.send_count.fetch_add(1, Ordering::Relaxed);
            if self.should_fail {
                return Err(MessengerError::Transport {
                    provider: ProviderKind::Discord,
                    message: "mock failure".into(),
                });
            }
            Ok(SendReceipt {
                provider: ProviderKind::Discord,
                message_ref: MessageRef::Discord {
                    channel_id: "123".into(),
                    message_id: "456".into(),
                },
                raw_id: "456".into(),
                metadata: BTreeMap::new(),
            })
        }
    }

    #[cfg(feature = "discord")]
    #[tokio::test]
    async fn register_and_send() {
        let mut messenger = Messenger::new();
        messenger.register(Box::new(MockProvider::new()));

        let dispatch = Dispatch::to(Target::discord_channel("123"));
        let message = Message::text("hello");

        let receipt = messenger.send(dispatch, &message).await.unwrap();
        assert_eq!(receipt.provider, ProviderKind::Discord);
        assert_eq!(receipt.raw_id, "456");
    }

    #[cfg(feature = "slack")]
    #[tokio::test]
    async fn unregistered_provider_returns_error() {
        let messenger = Messenger::new();
        let dispatch = Dispatch::to(Target::slack_channel("C123"));
        let message = Message::text("hello");

        let err = messenger.send(dispatch, &message).await.unwrap_err();
        assert!(matches!(err, MessengerError::MissingConfiguration { .. }));
    }

    #[cfg(feature = "discord")]
    #[tokio::test]
    async fn send_many_mixed_results() {
        let mut messenger = Messenger::new();
        messenger.register(Box::new(MockProvider::failing()));

        let dispatches = vec![
            Dispatch::to(Target::discord_channel("123")),
            Dispatch::to(Target::discord_channel("456")),
        ];
        let message = Message::text("hello");

        let results = messenger.send_many(dispatches, &message).await;
        assert_eq!(results.len(), 2);
        assert!(results[0].is_err());
        assert!(results[1].is_err());
    }
}
