//! Subagent watchdog state model.
//!
//! Tracks active subagents across the lifetime of a wrapped provider session.
//! The state is intentionally decoupled from child-process termination so it
//! can be unit-tested in isolation and reused by both the live semantic sink
//! and the exec-layer watchdog ticker.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Identifier for a subagent.
pub(crate) type SubagentId = String;

/// Mutable information stored for an active subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveSubagentInfo {
    /// Optional human-readable name or title.
    pub(crate) name: Option<String>,
    /// When the subagent was first observed starting.
    pub(crate) started_at: Instant,
    /// When the subagent last made progress.
    pub(crate) last_progress_at: Instant,
    /// When a diagnostic line was last emitted for this subagent.
    pub(crate) last_diagnostic_at: Option<Instant>,
}

/// Read-only snapshot of an active subagent suitable for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveSubagentSnapshot {
    pub(crate) id: SubagentId,
    pub(crate) name: Option<String>,
    pub(crate) started_at: Instant,
    pub(crate) last_progress_at: Instant,
    pub(crate) elapsed_since_start: Duration,
    pub(crate) elapsed_since_progress: Duration,
}

impl ActiveSubagentSnapshot {
    /// Convert this snapshot into the lib-side
    /// [`claudine::stream::logs::StuckSubagentInfo`] used to enrich
    /// `EarlyTermination::StepTimeout`.
    pub(crate) fn to_stuck_info(&self) -> claudine::stream::logs::StuckSubagentInfo {
        claudine::stream::logs::StuckSubagentInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            elapsed_since_progress: self.elapsed_since_progress,
        }
    }
}

/// A single diagnostic line for a stuck subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubagentDiagnosticLine {
    pub(crate) id: SubagentId,
    pub(crate) display_name: String,
    pub(crate) elapsed_since_start: Duration,
}

/// Shared watchdog state tracking active subagents.
///
/// All mutation methods accept an explicit `now` so tests can drive time
/// deterministically. Thin convenience wrappers that call `Instant::now()`
/// are provided for production call sites.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct WatchdogState {
    active: HashMap<SubagentId, ActiveSubagentInfo>,
}

impl WatchdogState {
    /// Record that a subagent started.
    ///
    /// If the id is already known, metadata is refreshed without resetting
    /// timestamps that the caller did not provide. This matches the contract
    /// that duplicate starts update metadata without losing timestamps
    /// unexpectedly.
    pub(crate) fn subagent_started(
        &mut self,
        id: SubagentId,
        name: Option<String>,
        now: Instant,
    ) {
        match self.active.get_mut(&id) {
            Some(info) => {
                if name.is_some() {
                    info.name = name;
                }
            }
            None => {
                self.active.insert(
                    id,
                    ActiveSubagentInfo {
                        name,
                        started_at: now,
                        last_progress_at: now,
                        last_diagnostic_at: None,
                    },
                );
            }
        }
    }

    /// Convenience: record a subagent start using the current instant.
    #[allow(dead_code)]
    pub(crate) fn subagent_started_now(&mut self, id: SubagentId, name: Option<String>) {
        self.subagent_started(id, name, Instant::now());
    }

    /// Record that a subagent stopped.
    pub(crate) fn subagent_stopped(&mut self, id: &SubagentId, _now: Instant) {
        self.active.remove(id);
    }

    /// Convenience: record a subagent stop using the current instant.
    #[allow(dead_code)]
    pub(crate) fn subagent_stopped_now(&mut self, id: &SubagentId) {
        self.subagent_stopped(id, Instant::now());
    }

    /// Record progress for an active subagent.
    ///
    /// Silently ignores unknown ids so callers do not need to gate on
    /// `SubagentStart` having already been seen.
    pub(crate) fn observe_subagent_progress(&mut self, id: &SubagentId, now: Instant,
    ) {
        if let Some(info) = self.active.get_mut(id) {
            info.last_progress_at = now;
        }
    }

    /// Convenience: record progress using the current instant.
    #[allow(dead_code)]
    pub(crate) fn observe_subagent_progress_now(&mut self, id: &SubagentId) {
        self.observe_subagent_progress(id, Instant::now());
    }

    /// Return a snapshot of every currently active subagent.
    pub(crate) fn active_subagents(&self, now: Instant) -> Vec<ActiveSubagentSnapshot> {
        self.active
            .iter()
            .map(|(id, info)| ActiveSubagentSnapshot {
                id: id.clone(),
                name: info.name.clone(),
                started_at: info.started_at,
                last_progress_at: info.last_progress_at,
                elapsed_since_start: now.saturating_duration_since(info.started_at),
                elapsed_since_progress: now.saturating_duration_since(info.last_progress_at),
            })
            .collect()
    }

    /// Return a snapshot of every active subagent for diagnostic enrichment
    /// at the moment a `step_timeout` watchdog rule fires.
    ///
    /// Equivalent to [`Self::active_subagents`], but named to make the call
    /// site at the breach point self-documenting.
    pub(crate) fn outstanding_at_breach(&self, now: Instant) -> Vec<ActiveSubagentSnapshot> {
        self.active_subagents(now)
    }

    /// Return active subagents whose time since last progress exceeds the
    /// given threshold.
    #[allow(dead_code)]
    pub(crate) fn stuck_subagents(
        &self,
        now: Instant,
        threshold: Duration,
    ) -> Vec<ActiveSubagentSnapshot> {
        self.active
            .iter()
            .filter(|(_, info)| now.saturating_duration_since(info.last_progress_at) >= threshold)
            .map(|(id, info)| ActiveSubagentSnapshot {
                id: id.clone(),
                name: info.name.clone(),
                started_at: info.started_at,
                last_progress_at: info.last_progress_at,
                elapsed_since_start: now.saturating_duration_since(info.started_at),
                elapsed_since_progress: now.saturating_duration_since(info.last_progress_at),
            })
            .collect()
    }

    /// Produce diagnostic lines for active subagents.
    ///
    /// Each subagent emits at most one diagnostic per `silence_window`.
    /// On emission, `last_diagnostic_at` is updated so subsequent calls
    /// within the same window are suppressed.
    pub(crate) fn diagnostic_lines(
        &mut self,
        now: Instant,
        silence_window: Duration,
    ) -> Vec<SubagentDiagnosticLine> {
        let mut lines = Vec::new();
        for (id, info) in &mut self.active {
            let elapsed = now.saturating_duration_since(info.started_at);
            if let Some(last) = info.last_diagnostic_at
                && now.saturating_duration_since(last) < silence_window
            {
                continue;
            }
            let display_name = info
                .name
                .as_deref()
                .filter(|n| !n.is_empty())
                .unwrap_or(id)
                .to_string();
            lines.push(SubagentDiagnosticLine {
                id: id.clone(),
                display_name,
                elapsed_since_start: elapsed,
            });
            info.last_diagnostic_at = Some(now);
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Instant {
        Instant::now()
    }

    #[test]
    fn starts_and_stops() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("Alpha".into()), t0);
        state.subagent_started("b".into(), Some("Beta".into()), t0);

        let active = state.active_subagents(t0);
        assert_eq!(active.len(), 2);
        assert!(active.iter().any(|s| s.id == "a" && s.name.as_deref() == Some("Alpha")));
        assert!(active.iter().any(|s| s.id == "b" && s.name.as_deref() == Some("Beta")));

        state.subagent_stopped(&"a".into(), t0);
        let active = state.active_subagents(t0);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "b");
    }

    #[test]
    fn duplicate_start_updates_metadata_without_losing_timestamps() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("Alpha".into()), t0);

        // Progress after 1s
        let t1 = t0 + Duration::from_secs(1);
        state.observe_subagent_progress(&"a".into(), t1);

        // Duplicate start with a new name
        let t2 = t0 + Duration::from_secs(2);
        state.subagent_started("a".into(), Some("Alpha-2".into()), t2);

        let active = state.active_subagents(t2);
        assert_eq!(active.len(), 1);
        let snap = &active[0];
        // Name updated
        assert_eq!(snap.name.as_deref(), Some("Alpha-2"));
        // Started_at preserved from first start
        assert_eq!(snap.started_at, t0);
        // Last progress preserved
        assert_eq!(snap.last_progress_at, t1);
    }

    #[test]
    fn progress_resets_only_matching_subagent() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);
        state.subagent_started("b".into(), None, t0);

        let t1 = t0 + Duration::from_secs(5);
        state.observe_subagent_progress(&"a".into(), t1);

        let active = state.active_subagents(t1);
        let a = active.iter().find(|s| s.id == "a").unwrap();
        let b = active.iter().find(|s| s.id == "b").unwrap();
        assert_eq!(a.last_progress_at, t1);
        assert_eq!(b.last_progress_at, t0);
    }

    #[test]
    fn stuck_subagents_respects_threshold() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);
        state.subagent_started("b".into(), None, t0);

        // a makes progress at t=2, b stays silent
        let t2 = t0 + Duration::from_secs(2);
        state.observe_subagent_progress(&"a".into(), t2);

        let t5 = t0 + Duration::from_secs(5);
        // a has been silent for 3s (5-2), b for 5s. threshold = 4s → only b.
        let stuck = state.stuck_subagents(t5, Duration::from_secs(4));
        assert_eq!(stuck.len(), 1);
        assert_eq!(stuck[0].id, "b");
    }

    #[test]
    fn diagnostic_lines_emitted_at_most_once_per_silence_window() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("Alpha".into()), t0);

        let window = Duration::from_secs(10);

        // First call emits
        let lines = state.diagnostic_lines(t0, window);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].id, "a");
        assert_eq!(lines[0].display_name, "Alpha");

        // Second call within window suppresses
        let t5 = t0 + Duration::from_secs(5);
        let lines = state.diagnostic_lines(t5, window);
        assert!(lines.is_empty());

        // After window passes, emits again
        let t15 = t0 + Duration::from_secs(15);
        let lines = state.diagnostic_lines(t15, window);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].id, "a");
    }

    #[test]
    fn diagnostic_lines_falls_back_to_id_when_name_missing() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("x".into(), None, t0);

        let lines = state.diagnostic_lines(t0, Duration::from_secs(10));
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].display_name, "x");
    }

    #[test]
    fn diagnostic_lines_includes_all_active_subagents() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("A".into()), t0);
        state.subagent_started("b".into(), Some("B".into()), t0);

        let lines = state.diagnostic_lines(t0, Duration::from_secs(10));
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().any(|l| l.id == "a"));
        assert!(lines.iter().any(|l| l.id == "b"));
    }

    #[test]
    fn diagnostic_lines_skips_stopped_subagents() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);
        state.subagent_stopped(&"a".into(), t0);

        let lines = state.diagnostic_lines(t0, Duration::from_secs(10));
        assert!(lines.is_empty());
    }

    #[test]
    fn stuck_subagents_empty_when_all_active_below_threshold() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);

        let stuck = state.stuck_subagents(t0, Duration::from_secs(10));
        assert!(stuck.is_empty());
    }

    #[test]
    fn active_subagents_reflects_elapsed_times() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);

        let t7 = t0 + Duration::from_secs(7);
        let active = state.active_subagents(t7);
        assert_eq!(active[0].elapsed_since_start, Duration::from_secs(7));
        assert_eq!(active[0].elapsed_since_progress, Duration::from_secs(7));
    }

    // --- outstanding_at_breach tests ---

    #[test]
    fn outstanding_at_breach_returns_all_active_subagents() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("Alpha".into()), t0);
        state.subagent_started("b".into(), None, t0);

        let t5 = t0 + Duration::from_secs(5);
        let outstanding = state.outstanding_at_breach(t5);
        assert_eq!(outstanding.len(), 2);
        let alpha = outstanding.iter().find(|s| s.id == "a").unwrap();
        assert_eq!(alpha.name.as_deref(), Some("Alpha"));
        assert_eq!(alpha.elapsed_since_start, Duration::from_secs(5));
    }

    #[test]
    fn outstanding_at_breach_skips_stopped_subagents() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);
        state.subagent_started("b".into(), None, t0);
        state.subagent_stopped(&"a".into(), t0);

        let outstanding = state.outstanding_at_breach(t0);
        assert_eq!(outstanding.len(), 1);
        assert_eq!(outstanding[0].id, "b");
    }

    #[test]
    fn outstanding_at_breach_empty_when_no_active() {
        let state = WatchdogState::default();
        let outstanding = state.outstanding_at_breach(now());
        assert!(outstanding.is_empty());
    }
}
