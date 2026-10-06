use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use worktree::remote_head::{FallbackReason, Phase};

use super::*;

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn the_spinner_text_follows_the_phase() {
    assert_eq!(phase_text(Phase::Checking), "updating");
    assert_eq!(
        phase_text(Phase::CheckingFallback { reason: FallbackReason::NoKey }),
        "no API key, using fallback method"
    );
    assert_eq!(
        phase_text(Phase::CheckingFallback { reason: FallbackReason::NotVisible }),
        "no API key, using fallback method"
    );
    assert_eq!(
        phase_text(Phase::CheckingFallback { reason: FallbackReason::RateLimited }),
        "rate limited, using fallback method"
    );
    assert_eq!(phase_text(Phase::CheckingFallback { reason: FallbackReason::Rejected }), "updating");
    assert_eq!(phase_text(Phase::CheckingFallback { reason: FallbackReason::Other }), "updating");
    assert_eq!(phase_text(Phase::Fetching), "pulling remote updates");
}

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn the_spinner_writes_nothing_when_its_output_is_not_a_terminal() {
    let captured = Captured::default();
    let progress = Progress::on(captured.clone(), false);
    for phase in [Phase::Checking, Phase::Fetching] {
        progress.show(phase);
    }
    std::thread::sleep(SPINNER_DELAY + ms(100));
    progress.finish();

    assert!(captured.0.lock().unwrap().is_empty());
}

#[test]
fn the_spinner_draws_the_phase_text_and_clears_its_line_on_a_terminal() {
    let captured = Captured::default();
    let progress = Progress::on(captured.clone(), true);
    progress.show(Phase::Fetching);
    std::thread::sleep(SPINNER_DELAY + ms(200));
    progress.finish();

    let written = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(written.contains("pulling remote updates"), "{written:?}");
    assert!(written.ends_with(biscuit_terminal::components::spinner::CLEAR_LINE), "{written:?}");
}

