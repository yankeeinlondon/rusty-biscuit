//! The identity of one preparation: resolved text plus playback config.

use biscuit_speaks::TtsConfig;

/// What makes two candidates the same preparation.
///
/// `TtsConfig` has no `Hash`/`Eq`, so the config half is its serialized form.
/// That covers every field, so a field added to `TtsConfig` later separates
/// candidates without this type changing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct PrewarmKey {
    text: String,
    config: String,
}

impl PrewarmKey {
    pub(crate) fn new(text: &str, config: &TtsConfig) -> Self {
        // `TtsConfig` is plain data with string-keyed structs only, so
        // serialization cannot fail in practice; `Debug` is the same full
        // field listing should it ever do so.
        let config = serde_json::to_string(config).unwrap_or_else(|_| format!("{config:?}"));
        Self {
            text: text.to_string(),
            config,
        }
    }
}

#[cfg(test)]
mod tests;
