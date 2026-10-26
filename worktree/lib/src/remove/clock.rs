//! Time for `wt remove`'s waits: a [`Clock`] that sleeps until cancelled,
//! and the [`Backoff`] between retries of an entry that is only temporarily
//! undeletable. Tests inject a clock that records each sleep instead of
//! taking it.

use std::time::Duration;

/// How a [`Clock::sleep`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slept {
    Elapsed,
    Cancelled,
}

/// Sleeps for the removal. Every wait goes through one so tests never sleep.
pub trait Clock: Sync {
    /// Waits `duration`, ending early once `cancelled` holds. `cancelled` is
    /// checked before the wait starts, so a cancelled caller never waits.
    fn sleep(&self, duration: Duration, cancelled: &dyn Fn() -> bool) -> Slept;
}

/// [`Clock`] on the host's time.
#[derive(Debug, Clone, Copy, Default)]
pub struct HostClock;

/// How often [`HostClock`] checks for cancellation while it sleeps: the
/// longest a Ctrl-C waits to be noticed.
const CANCEL_CHECK: Duration = Duration::from_millis(50);

impl Clock for HostClock {
    fn sleep(&self, duration: Duration, cancelled: &dyn Fn() -> bool) -> Slept {
        let mut left = duration;
        loop {
            if cancelled() {
                return Slept::Cancelled;
            }
            if left.is_zero() {
                return Slept::Elapsed;
            }
            let slice = left.min(CANCEL_CHECK);
            std::thread::sleep(slice);
            left -= slice;
        }
    }
}

/// The delays between retries of a temporarily blocked entry: 50 ms,
/// doubling to a 2 s cap. There is no overall limit; the wait ends when the
/// entry clears or the user cancels.
#[derive(Debug, Clone)]
pub struct Backoff {
    next: Duration,
}

impl Backoff {
    pub const FIRST: Duration = Duration::from_millis(50);
    pub const CAP: Duration = Duration::from_secs(2);

    pub fn new() -> Self {
        Self { next: Self::FIRST }
    }

    /// Waits the next delay on `clock`, then doubles it up to [`Self::CAP`].
    pub fn wait(&mut self, clock: &dyn Clock, cancelled: &dyn Fn() -> bool) -> Slept {
        let delay = self.next;
        self.next = (delay * 2).min(Self::CAP);
        clock.sleep(delay, cancelled)
    }

    /// Starts again from [`Self::FIRST`], after a retry made progress.
    pub fn reset(&mut self) {
        self.next = Self::FIRST;
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::Instant;

    use super::*;
    use crate::remove::test_support::ManualClock;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn backoff_doubles_from_fifty_milliseconds_to_a_two_second_cap() {
        let clock = ManualClock::new();
        let mut backoff = Backoff::new();
        for _ in 0..8 {
            assert_eq!(backoff.wait(&clock, &|| false), Slept::Elapsed);
        }
        assert_eq!(clock.slept(), [ms(50), ms(100), ms(200), ms(400), ms(800), ms(1600), ms(2000), ms(2000)]);
    }

    #[test]
    fn backoff_starts_over_after_a_reset() {
        let clock = ManualClock::new();
        let mut backoff = Backoff::new();
        backoff.wait(&clock, &|| false);
        backoff.wait(&clock, &|| false);
        backoff.reset();
        backoff.wait(&clock, &|| false);
        assert_eq!(clock.slept(), [ms(50), ms(100), ms(50)]);
    }

    #[test]
    fn a_cancelled_backoff_reports_cancellation() {
        let clock = ManualClock::cancelled_after(2);
        let mut backoff = Backoff::new();
        let ends: Vec<Slept> = (0..3).map(|_| backoff.wait(&clock, &|| false)).collect();
        assert_eq!(ends, [Slept::Elapsed, Slept::Elapsed, Slept::Cancelled]);
        assert_eq!(clock.slept(), [ms(50), ms(100)], "the cancelled wait takes no time");
    }

    #[test]
    fn the_host_clock_never_waits_once_cancelled() {
        let started = Instant::now();
        assert_eq!(HostClock.sleep(Duration::from_secs(60), &|| true), Slept::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    /// Cancellation set during the sleep ends it at the next check, not at
    /// the end of the requested duration.
    #[test]
    fn the_host_clock_notices_cancellation_while_it_sleeps() {
        let checks = Cell::new(0);
        let started = Instant::now();
        let ended = HostClock.sleep(Duration::from_secs(60), &|| {
            checks.set(checks.get() + 1);
            checks.get() > 2
        });
        assert_eq!(ended, Slept::Cancelled);
        assert_eq!(checks.get(), 3);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn the_host_clock_sleeps_the_whole_duration_when_not_cancelled() {
        let started = Instant::now();
        assert_eq!(HostClock.sleep(ms(120), &|| false), Slept::Elapsed);
        assert!(started.elapsed() >= ms(120));
    }
}
