//! An injectable monotonic clock.
//!
//! Timing that decides behavior (a slow-request threshold, an elapsed figure in
//! a log line) reads a [`Clock`] instead of calling [`Instant::now`] directly,
//! so a test can drive time with a [`ManualClock`] and assert exact boundaries
//! without sleeping.
//!
//! A clock also answers wall time, sampled together with its monotonic time
//! ([`Clock::pair`]): a remote cache entry expires at a wall-clock instant,
//! and the slot worker turns that into a monotonic deadline from one paired
//! sample, so a test advancing a [`ManualClock`] moves both.

use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

/// A source of monotonic instants, and of wall time beside them.
pub trait Clock: Send + Sync + std::fmt::Debug {
    fn now(&self) -> Instant;

    /// The wall-clock time now.
    fn wall(&self) -> SystemTime {
        SystemTime::now()
    }

    /// Wall and monotonic time sampled together.
    fn pair(&self) -> (SystemTime, Instant) {
        (self.wall(), self.now())
    }
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
    wall_origin: SystemTime,
    offset: Mutex<Duration>,
}

impl ManualClock {
    pub fn new() -> Self {
        Self { origin: Instant::now(), wall_origin: SystemTime::now(), offset: Mutex::new(Duration::ZERO) }
    }

    /// A clock whose wall time starts at `wall`.
    pub fn starting_at(wall: SystemTime) -> Self {
        Self { wall_origin: wall, ..Self::new() }
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

    fn wall(&self) -> SystemTime {
        self.wall_origin + *self.offset.lock().expect("manual clock poisoned")
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
    fn a_manual_clock_advances_wall_and_monotonic_time_together() {
        let clock = ManualClock::new();
        let (wall, mono) = clock.pair();
        clock.advance(Duration::from_secs(3));
        let (later_wall, later_mono) = clock.pair();
        assert_eq!(later_wall.duration_since(wall).unwrap(), Duration::from_secs(3));
        assert_eq!(later_mono.duration_since(mono), Duration::from_secs(3));
    }

    #[test]
    fn the_system_clock_is_monotonic() {
        let first = SystemClock.now();
        assert!(SystemClock.now() >= first);
    }
}
