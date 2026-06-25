//! Best-effort speech helpers for runtime lifecycle side effects.

use biscuit_speaks::TtsConfig;

/// Speak a short lifecycle message without affecting the main harness flow.
///
/// This uses `biscuit-speaks` and intentionally ignores playback failures.
/// If no Tokio runtime is active, it falls back to a lightweight background
/// thread with its own current-thread runtime.
pub fn speak_when_able(text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }

    let text = text.to_string();

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            biscuit_speaks::speak_when_able(&text, &TtsConfig::default()).await;
        });
        return;
    }

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        match runtime {
            Ok(runtime) => {
                runtime.block_on(async move {
                    biscuit_speaks::speak_when_able(&text, &TtsConfig::default()).await;
                });
            }
            Err(error) => {
                tracing::warn!(%error, "failed to create runtime for harness speech");
            }
        }
    });
}
