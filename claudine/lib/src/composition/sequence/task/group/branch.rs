//! A group leg's private state, built when the leg is admitted and released
//! when it settles.
//!
//! A [`LegBranch`] is the leg's mutable `RuntimeState` and live-frontmatter
//! cell, both copied from the group-start seed. The scheduler builds one only
//! for an admitted leg and drops it as soon as the leg's outcome (its result
//! and mutation delta) is extracted, before its worker admits the next leg, so
//! at most `max_parallel` branches are alive at once.
//!
//! Lifetime counts are test-only. With `work-counters`, each branch is one
//! `Work::RuntimeStateBranch` and one live `Gauge::RuntimeStateBranch` unit.
//! With `test-fixtures`, each group attempt keeps its own [`BranchLedger`]
//! and, when [`TEST_GROUP_BRANCHES_ENV`] names a file, appends one JSON line
//! to it when the attempt settles, so CLI tests can read the counts of a
//! child process. Without either feature neither type holds any state.

use std::sync::{Arc, Mutex};

use darkmatter::markdown::compose::SharedFrontmatter;

use crate::composition::runtime_state::{RuntimeSnapshot, RuntimeState};

/// The file a `test-fixtures` build appends one JSON line to per settled
/// group attempt: `group`, `legs` (declared), `cap` (worker threads),
/// `built` (branches constructed), `peak` (most alive at once), and `live`
/// (still alive when the attempt settled; always 0).
#[cfg(feature = "test-fixtures")]
const TEST_GROUP_BRANCHES_ENV: &str = "CLAUDINE_TEST_GROUP_BRANCHES";

/// One admitted leg's private runtime and live-frontmatter cell.
pub(super) struct LegBranch<'a> {
    pub(super) runtime: Arc<RuntimeState>,
    /// `None` when the group runs without a live document cell.
    pub(super) live_frontmatter: Option<Mutex<SharedFrontmatter>>,
    ledger: &'a BranchLedger,
}

impl<'a> LegBranch<'a> {
    /// Build an admitted leg's branch from the group-start seed.
    pub(super) fn admit(
        seed: &RuntimeSnapshot,
        live_seed: Option<&SharedFrontmatter>,
        ledger: &'a BranchLedger,
    ) -> Self {
        crate::note_work!(RuntimeStateBranch);
        #[cfg(feature = "work-counters")]
        crate::work_counters::enter(crate::work_counters::Gauge::RuntimeStateBranch);
        ledger.enter();
        let live_frontmatter = live_seed.map(|seed| {
            // The cell shares the seed's base and copies only the values the
            // group-start view owns.
            crate::note_work!(LiveFrontmatterView);
            crate::note_work!(
                LiveFrontmatterCopy,
                crate::work_counters::value_nodes(seed.owned())
            );
            Mutex::new(seed.clone())
        });
        Self {
            runtime: Arc::new(RuntimeState::from_snapshot(seed.clone())),
            live_frontmatter,
            ledger,
        }
    }
}

impl Drop for LegBranch<'_> {
    fn drop(&mut self) {
        #[cfg(feature = "work-counters")]
        crate::work_counters::leave(crate::work_counters::Gauge::RuntimeStateBranch);
        self.ledger.leave();
    }
}

/// One group attempt's branch counts (`test-fixtures` only).
#[derive(Default)]
pub(super) struct BranchLedger {
    /// `(built, live, peak)`.
    #[cfg(feature = "test-fixtures")]
    counts: Mutex<(usize, usize, usize)>,
}

impl BranchLedger {
    fn enter(&self) {
        #[cfg(feature = "test-fixtures")]
        {
            let mut counts = self.counts.lock().expect("branch ledger poisoned");
            counts.0 += 1;
            counts.1 += 1;
            counts.2 = counts.2.max(counts.1);
        }
    }

    fn leave(&self) {
        #[cfg(feature = "test-fixtures")]
        {
            self.counts.lock().expect("branch ledger poisoned").1 -= 1;
        }
    }

    /// Append this settled attempt's counts to [`TEST_GROUP_BRANCHES_ENV`]'s
    /// file, when a test set it.
    pub(super) fn record(&self, group: &str, legs: usize, cap: usize) {
        #[cfg(feature = "test-fixtures")]
        if let Ok(path) = std::env::var(TEST_GROUP_BRANCHES_ENV) {
            use std::io::Write;
            let (built, live, peak) = *self.counts.lock().expect("branch ledger poisoned");
            let line = serde_json::json!({
                "group": group, "legs": legs, "cap": cap, "built": built, "peak": peak, "live": live,
            });
            // A test reads the file after the process exits; a failed append
            // shows up there as a missing line.
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| writeln!(file, "{line}"));
        }
        #[cfg(not(feature = "test-fixtures"))]
        let _ = (group, legs, cap);
    }
}
