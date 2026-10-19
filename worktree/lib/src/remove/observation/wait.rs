//! The waiting policy for confirming delivery: a stall limit that progress
//! resets and a ceiling that nothing resets.
//!
//! A drain, a startup check, or an admin-release check waits for markers the
//! platform must acknowledge. Each awaited source (a Windows handle, or the
//! single macOS stream) has its own stall clock; the ceiling spans the whole
//! wait. The caller decides what counts as progress and reports it here; this
//! module only keeps the clocks. Time is passed in, never read, so the policy
//! is tested without sleeping.

use std::time::{Duration, Instant};

/// Silence on one awaited source longer than this is [`Check::Stalled`].
pub(crate) const STALL_LIMIT: Duration = Duration::from_secs(30);

/// One wait never lasts longer than this, however much progress it sees.
pub(crate) const WAIT_CEILING: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WaitPolicy {
    pub stall: Duration,
    pub ceiling: Duration,
}

impl Default for WaitPolicy {
    fn default() -> Self {
        Self { stall: STALL_LIMIT, ceiling: WAIT_CEILING }
    }
}

/// What a wait should do after [`WaitState::check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Check<K> {
    Continue,
    /// `key` saw no progress for `waited`, its time since the last progress.
    Stalled { waited: Duration, key: K },
    /// The wait has lasted `waited` since it began.
    Ceiling { waited: Duration },
}

#[derive(Debug)]
struct Awaited<K> {
    key: K,
    last_progress: Instant,
    /// Highest acknowledgment seen; a lower or equal one is not progress.
    highest: Option<u64>,
}

/// The clocks of one wait.
#[derive(Debug)]
pub(crate) struct WaitState<K> {
    policy: WaitPolicy,
    started: Instant,
    awaited: Vec<Awaited<K>>,
}

impl<K: Copy + Eq> WaitState<K> {
    /// Starts the ceiling and every key's stall clock at `now`.
    pub fn new(policy: WaitPolicy, now: Instant, keys: impl IntoIterator<Item = K>) -> Self {
        let awaited = keys.into_iter().map(|key| Awaited { key, last_progress: now, highest: None }).collect();
        Self { policy, started: now, awaited }
    }

    /// Relevant progress on `key` restarts its stall clock. A completed or
    /// unknown key is ignored.
    pub fn progress(&mut self, key: K, now: Instant) {
        if let Some(awaited) = self.awaited.iter_mut().find(|awaited| awaited.key == key) {
            awaited.last_progress = now;
        }
    }

    /// Records acknowledgment `number` on `key`. Only a number above every
    /// earlier one is progress; a duplicate or older one changes nothing.
    /// Returns whether it was progress.
    pub fn acknowledge(&mut self, key: K, number: u64, now: Instant) -> bool {
        let Some(awaited) = self.awaited.iter_mut().find(|awaited| awaited.key == key) else {
            return false;
        };
        if awaited.highest.is_some_and(|highest| number <= highest) {
            return false;
        }
        awaited.highest = Some(number);
        awaited.last_progress = now;
        true
    }

    /// `key` is confirmed and needs no further progress.
    pub fn complete(&mut self, key: K) {
        self.awaited.retain(|awaited| awaited.key != key);
    }

    /// The ceiling wins when it and a stall expire at the same check. With
    /// several stalled keys, the one silent longest is reported.
    pub fn check(&self, now: Instant) -> Check<K> {
        let elapsed = now.saturating_duration_since(self.started);
        if elapsed >= self.policy.ceiling {
            return Check::Ceiling { waited: elapsed };
        }
        self.awaited
            .iter()
            .map(|awaited| (awaited.key, now.saturating_duration_since(awaited.last_progress)))
            .filter(|(_, silent)| *silent >= self.policy.stall)
            .max_by_key(|(_, silent)| *silent)
            .map_or(Check::Continue, |(key, waited)| Check::Stalled { waited, key })
    }

    /// How long a wait may block before the next [`check`](Self::check) can
    /// change its answer; zero once a limit has expired.
    pub fn until_next_deadline(&self, now: Instant) -> Duration {
        let ceiling = (self.started + self.policy.ceiling).saturating_duration_since(now);
        self.awaited
            .iter()
            .map(|awaited| (awaited.last_progress + self.policy.stall).saturating_duration_since(now))
            .fold(ceiling, Duration::min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    fn at(start: Instant, seconds: u64) -> Instant {
        start + Duration::from_secs(seconds)
    }

    fn single(start: Instant) -> WaitState<()> {
        WaitState::new(WaitPolicy::default(), start, [()])
    }

    #[test]
    fn default_policy_is_a_thirty_second_stall_and_five_minute_ceiling() {
        assert_eq!(WaitPolicy::default(), WaitPolicy { stall: Duration::from_secs(30), ceiling: Duration::from_secs(300) });
    }

    #[test]
    fn delayed_progress_outlasts_the_old_five_second_total() {
        let start = Instant::now();
        let mut wait = single(start);
        for (number, seconds) in [(1, 8), (2, 20), (3, 45)] {
            assert_eq!(wait.check(at(start, seconds)), Check::Continue, "{seconds}s into the wait");
            assert!(wait.acknowledge((), number, at(start, seconds)));
        }
        assert_eq!(wait.check(at(start, 60)), Check::Continue);
    }

    #[test]
    fn silence_stalls_at_the_stall_limit() {
        let start = Instant::now();
        let wait = single(start);
        assert_eq!(wait.check(start + STALL_LIMIT - SECOND), Check::Continue);
        assert_eq!(wait.check(start + STALL_LIMIT), Check::Stalled { waited: STALL_LIMIT, key: () });
    }

    #[test]
    fn stall_waited_counts_from_the_last_progress_not_the_start() {
        let start = Instant::now();
        let mut wait = single(start);
        wait.progress((), at(start, 100));
        assert_eq!(wait.check(at(start, 129)), Check::Continue);
        assert_eq!(wait.check(at(start, 131)), Check::Stalled { waited: Duration::from_secs(31), key: () });
    }

    #[test]
    fn repeated_progress_cannot_pass_the_ceiling() {
        let start = Instant::now();
        let mut wait = single(start);
        let mut number = 0;
        for seconds in (10..WAIT_CEILING.as_secs()).step_by(10) {
            number += 1;
            wait.acknowledge((), number, at(start, seconds));
            assert_eq!(wait.check(at(start, seconds)), Check::Continue, "{seconds}s into the wait");
        }
        wait.progress((), start + WAIT_CEILING);
        assert_eq!(wait.check(start + WAIT_CEILING), Check::Ceiling { waited: WAIT_CEILING });
    }

    #[test]
    fn duplicate_and_older_acknowledgments_do_not_reset_the_stall_clock() {
        let start = Instant::now();
        let mut wait = single(start);
        assert!(wait.acknowledge((), 5, at(start, 1)));
        assert!(!wait.acknowledge((), 5, at(start, 20)), "a duplicate is not progress");
        assert!(!wait.acknowledge((), 3, at(start, 25)), "an older marker is not progress");
        assert_eq!(wait.check(at(start, 31)), Check::Stalled { waited: Duration::from_secs(30), key: () });
    }

    #[test]
    fn activity_reported_as_nothing_leaves_both_clocks_running() {
        // Unrelated sibling events, marker cleanup, and a returning flush are
        // never reported as progress; only checks happen in between.
        let start = Instant::now();
        let wait = single(start);
        for seconds in 0..30 {
            assert_eq!(wait.check(at(start, seconds)), Check::Continue);
        }
        assert_eq!(wait.check(at(start, 30)), Check::Stalled { waited: STALL_LIMIT, key: () });
        assert_eq!(wait.until_next_deadline(at(start, 30)), Duration::ZERO);
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Handle {
        Parent,
        Admin,
    }

    #[test]
    fn a_busy_handle_does_not_hide_a_stalled_one() {
        let start = Instant::now();
        let mut wait = WaitState::new(WaitPolicy::default(), start, [Handle::Parent, Handle::Admin]);
        for seconds in 1..=31 {
            wait.acknowledge(Handle::Parent, seconds, at(start, seconds));
        }
        assert_eq!(wait.check(at(start, 31)), Check::Stalled { waited: Duration::from_secs(31), key: Handle::Admin });
    }

    #[test]
    fn progress_on_one_handle_is_not_progress_on_the_other() {
        let start = Instant::now();
        let mut wait = WaitState::new(WaitPolicy::default(), start, [Handle::Parent, Handle::Admin]);
        wait.progress(Handle::Admin, at(start, 20));
        assert_eq!(wait.check(at(start, 30)), Check::Stalled { waited: STALL_LIMIT, key: Handle::Parent });
    }

    #[test]
    fn a_completed_handle_needs_no_progress() {
        let start = Instant::now();
        let mut wait = WaitState::new(WaitPolicy::default(), start, [Handle::Parent, Handle::Admin]);
        wait.complete(Handle::Admin);
        wait.acknowledge(Handle::Parent, 1, at(start, 25));
        assert_eq!(wait.check(at(start, 50)), Check::Continue);
        assert!(!wait.acknowledge(Handle::Admin, 1, at(start, 50)), "a completed handle takes no acknowledgment");
    }

    #[test]
    fn the_ceiling_wins_a_tie_with_a_stall() {
        let start = Instant::now();
        let policy = WaitPolicy { stall: Duration::from_secs(30), ceiling: Duration::from_secs(60) };
        let mut wait = WaitState::new(policy, start, [()]);
        wait.progress((), at(start, 30));
        assert_eq!(wait.check(at(start, 60)), Check::Ceiling { waited: Duration::from_secs(60) });
    }

    #[test]
    fn next_deadline_is_the_nearest_of_the_stall_and_ceiling() {
        let start = Instant::now();
        let mut wait = WaitState::new(WaitPolicy::default(), start, [Handle::Parent, Handle::Admin]);
        assert_eq!(wait.until_next_deadline(start), STALL_LIMIT);
        wait.progress(Handle::Parent, at(start, 10));
        assert_eq!(wait.until_next_deadline(at(start, 10)), Duration::from_secs(20), "the admin clock is older");
        wait.complete(Handle::Admin);
        assert_eq!(wait.until_next_deadline(at(start, 10)), STALL_LIMIT);
        wait.progress(Handle::Parent, at(start, 290));
        assert_eq!(wait.until_next_deadline(at(start, 290)), Duration::from_secs(10), "the ceiling is nearer");
    }
}
