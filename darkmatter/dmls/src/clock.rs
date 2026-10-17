//! An injectable monotonic clock.
//!
//! Timing that decides behavior (a slow-request threshold, an elapsed figure in
//! a log line) reads a [`Clock`] instead of calling [`Instant::now`] directly,
//! so a test can drive time with a [`ManualClock`] and assert exact boundaries
//! without sleeping.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A source of monotonic instants.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

/// The process's monotonic clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A clock that moves only when [`ManualClock::advance`] is called.
#[derive(Debug)]
pub struct ManualClock {
    origin: Instant,
    offset: Mutex<Duration>,
}

impl ManualClock {
    pub fn new() -> Self {
        Self { origin: Instant::now(), offset: Mutex::new(Duration::ZERO) }
    }

    pub fn advance(&self, by: Duration) {
        *self.offset.lock().expect("manual clock poisoned") += by;
    }
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        self.origin + *self.offset.lock().expect("manual clock poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manual_clock_moves_only_when_advanced() {
        let clock = ManualClock::new();
        let start = clock.now();
        assert_eq!(clock.now(), start);
        clock.advance(Duration::from_millis(500));
        assert_eq!(clock.now().duration_since(start), Duration::from_millis(500));
    }

    #[test]
    fn the_system_clock_is_monotonic() {
        let first = SystemClock.now();
        assert!(SystemClock.now() >= first);
    }
}
