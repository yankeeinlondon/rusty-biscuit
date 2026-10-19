use std::future::Future;
use std::pin::Pin;

use biscuit_speaks::{Speak, TtsConfig};

/// The future one preparation runs as.
///
/// Dropping it cancels the preparation: biscuit-speaks publishes a cache file
/// only after synthesis completes, so a dropped preparation leaves no entry.
pub(crate) type PrepareFuture = Pin<Box<dyn Future<Output = Result<(), PrewarmError>> + Send>>;

/// A failed speech preparation.
///
/// Carries no provider error text: a pre-warm warning may never include the
/// phrase, a secret, or a raw provider response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PrewarmError {
    #[error("speech preparation failed")]
    Prepare,
}

/// Prepares one phrase's audio ahead of the event that speaks it.
///
/// A successful preparation includes the no-op case (a provider that produces
/// no file, or a default that cannot safely be resolved); it never publishes
/// playback.
pub(crate) trait Preparer: Send + Sync {
    fn prepare(&self, text: String, config: TtsConfig) -> PrepareFuture;
}

/// The production [`Preparer`], backed by [`Speak::prepare`].
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SpeakPreparer;

impl Preparer for SpeakPreparer {
    fn prepare(&self, text: String, config: TtsConfig) -> PrepareFuture {
        Box::pin(async move {
            // The returned builder would publish playback; the event that later
            // speaks this phrase resolves and publishes it on its own.
            Speak::new(text)
                .with_config(config)
                .prepare()
                .await
                .map(drop)
                .map_err(|_| PrewarmError::Prepare)
        })
    }
}
