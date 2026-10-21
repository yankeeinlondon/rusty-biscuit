//! The waiting policy for confirming delivery: a stall limit that progress
//! resets and a ceiling that nothing resets.
//!
//! A drain, a startup check, or an admin-release check waits for markers the
//! platform must acknowledge. Each awaited source (a Windows handle, or the
//! single macOS stream) has its own stall clock; the ceiling spans the whole
//! wait. The delivery side decides what counts as progress and reports it to
//! a [`Delivery`]; [`await_marker`] turns those reports into the clocks'
//! progress and blocks until the markers arrive, a limit expires, coverage is
//! lost, or the caller is interrupted. Time comes from a [`Source`], so the
//! policy and the loop are both tested without sleeping.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// Silence on one awaited source longer than this is [`Check::Stalled`].
pub(crate) const STALL_LIMIT: Duration = Duration::from_secs(30);

/// One wait never lasts longer than this, however much progress it sees.
pub(crate) const WAIT_CEILING: Duration = Duration::from_secs(5 * 60);

/// How often a wait rechecks the interrupt and recorded loss when no report
/// wakes it sooner. Both are polled: the interrupt is a predicate, and loss is
/// recorded in the event queue, which does not wake a wait.
pub(crate) const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// How long confirming delivery may take: [`stall`](Self::stall) without
/// progress on one awaited source, or [`ceiling`](Self::ceiling) in total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitPolicy {
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

    /// Awaits `key` again (a further marker round), its stall clock starting
    /// at `now`; the ceiling keeps running from the wait's start.
    #[cfg_attr(not(any(test, target_os = "macos")), allow(dead_code, reason = "only the FSEvents adapter repeats rounds"))]
    pub fn expect(&mut self, key: K, now: Instant) {
        if !self.awaited.iter().any(|awaited| awaited.key == key) {
            self.awaited.push(Awaited { key, last_progress: now, highest: None });
        }
    }

    /// The keys still awaited, in the order they were added.
    pub fn awaiting(&self) -> impl Iterator<Item = K> + '_ {
        self.awaited.iter().map(|awaited| awaited.key)
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

/// What one awaited source has delivered so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Delivered {
    /// The highest marker acknowledged; `u64::MAX` once the source ended, so
    /// no wait is held on it.
    pub marker: u64,
    /// In-scope events delivered: progress, but not confirmation.
    pub events: u64,
}

/// Where [`await_marker`] reads deliveries and the time, and blocks.
pub(crate) trait Source<K> {
    fn now(&self) -> Instant;
    /// Changes with every report; read before [`delivered`](Self::delivered),
    /// so a report made in between ends the next [`park`](Self::park) at once.
    fn generation(&self) -> u64;
    fn delivered(&self, key: K) -> Delivered;
    /// Blocks for at most `timeout`, or not at all once the generation is no
    /// longer `seen`.
    fn park(&self, seen: u64, timeout: Duration);
}

/// The reports an adapter's delivery thread (or callback) makes to the
/// session's waits, per awaited source.
#[derive(Debug)]
pub(crate) struct Delivery<K> {
    state: Mutex<DeliveryState<K>>,
    changed: Condvar,
}

#[derive(Debug)]
struct DeliveryState<K> {
    generation: u64,
    sources: Vec<(K, Delivered)>,
}

impl<K: Copy + Eq> Delivery<K> {
    pub fn new(keys: impl IntoIterator<Item = K>) -> Self {
        let sources = keys.into_iter().map(|key| (key, Delivered::default())).collect();
        Self { state: Mutex::new(DeliveryState { generation: 0, sources }), changed: Condvar::new() }
    }

    /// Marker `number` arrived on `key`. A lower or repeated one changes
    /// nothing, so a marker's own cleanup event is not progress.
    pub fn acknowledge(&self, key: K, number: u64) {
        self.report(key, |delivered| delivered.marker = delivered.marker.max(number));
    }

    /// An in-scope event arrived on `key`.
    pub fn event(&self, key: K) {
        self.report(key, |delivered| delivered.events += 1);
    }

    /// `key` delivers nothing more (its watch ended or was released).
    #[cfg_attr(not(any(test, windows)), allow(dead_code, reason = "only the Windows adapter's watches end one by one"))]
    pub fn end(&self, key: K) {
        self.report(key, |delivered| delivered.marker = u64::MAX);
    }

    fn report(&self, key: K, change: impl FnOnce(&mut Delivered)) {
        let mut state = self.lock();
        if let Some((_, delivered)) = state.sources.iter_mut().find(|(source, _)| *source == key) {
            change(delivered);
        }
        state.generation += 1;
        drop(state);
        self.changed.notify_all();
    }

    // A reporter that panicked leaves the counts it managed to write.
    fn lock(&self) -> MutexGuard<'_, DeliveryState<K>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<K: Copy + Eq> Source<K> for Delivery<K> {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn generation(&self) -> u64 {
        self.lock().generation
    }

    fn delivered(&self, key: K) -> Delivered {
        self.lock().sources.iter().find(|(source, _)| *source == key).map_or_else(Delivered::default, |(_, delivered)| *delivered)
    }

    fn park(&self, seen: u64, timeout: Duration) {
        let state = self.lock();
        if state.generation == seen {
            drop(self.changed.wait_timeout(state, timeout).unwrap_or_else(PoisonError::into_inner));
        }
    }
}

/// How [`await_marker`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome<K> {
    /// Every awaited key delivered the marker, `round_trip` after the wait
    /// began.
    Confirmed { round_trip: Duration },
    /// Coverage was lost; the loss is already recorded.
    Lost,
    /// The caller was interrupted; nothing is recorded.
    Interrupted,
    Stalled { waited: Duration, key: K },
    /// `key` is the first one still awaited, if any.
    Ceiling { waited: Duration, key: Option<K> },
}

/// Waits until every key `wait` awaits has delivered marker `marker` (or
/// ended). A new event or a higher marker on a key is that key's progress;
/// nothing else is. Confirmation outranks loss and the interrupt, which are
/// rechecked at least every [`POLL_INTERVAL`].
pub(crate) fn await_marker<K: Copy + Eq>(
    source: &impl Source<K>,
    wait: &mut WaitState<K>,
    marker: u64,
    lost: &dyn Fn() -> bool,
    interrupted: &dyn Fn() -> bool,
) -> Outcome<K> {
    let started = source.now();
    let mut seen: Vec<(K, Delivered)> = wait.awaiting().map(|key| (key, source.delivered(key))).collect();
    loop {
        let generation = source.generation();
        let now = source.now();
        for (key, last) in &mut seen {
            let current = source.delivered(*key);
            if current.events > last.events {
                wait.progress(*key, now);
            }
            if current.marker > last.marker {
                wait.acknowledge(*key, current.marker, now);
            }
            *last = current;
            if current.marker >= marker {
                wait.complete(*key);
            }
        }
        seen.retain(|(_, delivered)| delivered.marker < marker);
        if seen.is_empty() {
            return Outcome::Confirmed { round_trip: now.saturating_duration_since(started) };
        }
        if lost() {
            return Outcome::Lost;
        }
        if interrupted() {
            return Outcome::Interrupted;
        }
        match wait.check(now) {
            Check::Continue => {}
            Check::Stalled { waited, key } => return Outcome::Stalled { waited, key },
            Check::Ceiling { waited } => return Outcome::Ceiling { waited, key: seen.first().map(|(key, _)| *key) },
        }
        source.park(generation, wait.until_next_deadline(now).min(POLL_INTERVAL));
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

    /// A further marker round restarts its key's stall clock, never the
    /// ceiling, which still counts from the wait's start.
    #[test]
    fn a_repeated_round_restarts_its_stall_clock_but_not_the_ceiling() {
        let start = Instant::now();
        let mut wait = single(start);
        wait.complete(());
        assert_eq!(wait.check(at(start, 200)), Check::Continue, "nothing is awaited between rounds");
        wait.expect((), at(start, 20));
        assert_eq!(wait.check(at(start, 49)), Check::Continue);
        assert_eq!(wait.check(at(start, 50)), Check::Stalled { waited: STALL_LIMIT, key: () });
        wait.progress((), at(start, 290));
        wait.expect((), at(start, 295));
        assert_eq!(wait.awaiting().count(), 1, "expecting an awaited key adds nothing");
        assert_eq!(wait.check(start + WAIT_CEILING), Check::Ceiling { waited: WAIT_CEILING });
    }

    /// Reports change the generation; a marker only ever rises (a lower or
    /// repeated one, such as a marker's own cleanup event, is no change), and
    /// an ended source is never waited for.
    #[test]
    fn delivery_keeps_the_highest_marker_and_counts_events() {
        let delivery = Delivery::new([Handle::Parent, Handle::Admin]);
        let before = delivery.generation();
        delivery.acknowledge(Handle::Parent, 5);
        delivery.acknowledge(Handle::Parent, 3);
        delivery.acknowledge(Handle::Parent, 5);
        delivery.event(Handle::Parent);
        delivery.event(Handle::Parent);
        assert_eq!(delivery.delivered(Handle::Parent), Delivered { marker: 5, events: 2 });
        assert_eq!(delivery.delivered(Handle::Admin), Delivered::default(), "reports on one source leave the other");
        assert!(delivery.generation() > before);
        delivery.end(Handle::Admin);
        assert_eq!(delivery.delivered(Handle::Admin).marker, u64::MAX);
    }

    /// A report made after the generation was read ends the next park at
    /// once, so a wait never sleeps through it.
    #[test]
    fn a_report_after_reading_the_generation_ends_the_park_at_once() {
        let delivery = Delivery::new([()]);
        let seen = delivery.generation();
        delivery.event(());
        let started = Instant::now();
        delivery.park(seen, Duration::from_secs(30));
        assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
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
