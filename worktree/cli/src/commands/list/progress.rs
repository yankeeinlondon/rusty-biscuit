//! The spinner `wt list` shows while the library's wait follows the refresh
//! worker; the wait reports each phase through its `on_phase` callback.

use std::time::Duration;

use biscuit_terminal::components::spinner::{Spinner, SpinnerHandle};
use worktree::remote_head::{FallbackReason, Phase};

/// The spinner stays hidden this long, so a quick answer never flashes it.
const SPINNER_DELAY: Duration = Duration::from_millis(150);

/// The spinner's text for `phase`.
pub fn phase_text(phase: Phase) -> &'static str {
    match phase {
        Phase::CheckingFallback { reason: FallbackReason::NoKey | FallbackReason::NotVisible } => {
            "no API key, using fallback method"
        }
        Phase::CheckingFallback { reason: FallbackReason::RateLimited } => "rate limited, using fallback method",
        Phase::Fetching => "pulling remote updates",
        Phase::Checking | Phase::CheckingFallback { .. } => "updating",
    }
}

/// The spinner shown while waiting; it draws only when its output is a
/// terminal, after [`SPINNER_DELAY`].
pub struct Progress {
    spinner: SpinnerHandle,
}

impl Progress {
    pub fn on_stderr() -> Self {
        Self { spinner: Spinner::new(phase_text(Phase::Checking)).with_delay(SPINNER_DELAY).start_on_stderr() }
    }

    #[cfg(test)]
    pub fn on(writer: impl std::io::Write + Send + 'static, is_terminal: bool) -> Self {
        Self {
            spinner: Spinner::new(phase_text(Phase::Checking)).with_delay(SPINNER_DELAY).start_on(writer, is_terminal),
        }
    }

    pub fn show(&self, phase: Phase) {
        self.spinner.set_text(phase_text(phase));
    }

    /// Clears the spinner's line, if it drew one.
    pub fn finish(self) {
        self.spinner.finish();
    }
}

#[cfg(test)]
mod tests;
