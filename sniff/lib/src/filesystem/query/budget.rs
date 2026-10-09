use super::PathUsageError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The single monotonic budget shared by every phase of one query.
///
/// Phases call [`expired`](Self::expired) before scheduling each unit of work;
/// nothing interrupts a call already running, so a slow native call can
/// overrun the deadline.
#[derive(Debug)]
pub(crate) struct Budget {
    started: Instant,
    deadline: Instant,
    limit: Duration,
    exhausted: AtomicBool,
    #[cfg(test)]
    checks_before_expiry: std::sync::atomic::AtomicI64,
}

impl Budget {
    /// Starts the budget now.
    pub(crate) fn start(limit: Duration) -> Result<Self, PathUsageError> {
        if limit.is_zero() {
            return Err(PathUsageError::InvalidOption {
                message: "the deadline must be greater than zero".to_string(),
            });
        }
        let started = Instant::now();
        let deadline = started
            .checked_add(limit)
            .ok_or_else(|| PathUsageError::InvalidOption {
                message: format!("a deadline of {limit:?} cannot be represented"),
            })?;
        Ok(Self {
            started,
            deadline,
            limit,
            exhausted: AtomicBool::new(false),
            #[cfg(test)]
            checks_before_expiry: std::sync::atomic::AtomicI64::new(i64::MAX),
        })
    }

    /// A budget that reports expiry from the `checks`-th call to
    /// [`expired`](Self::expired) onward, independent of the clock.
    #[cfg(test)]
    pub(crate) fn expiring_after_checks(checks: i64) -> Self {
        let budget = Self::start(Duration::from_secs(3600)).expect("valid test budget");
        budget
            .checks_before_expiry
            .store(checks, Ordering::SeqCst);
        budget
    }

    /// Calls to [`expired`](Self::expired) made so far on a budget from
    /// [`expiring_after_checks`](Self::expiring_after_checks)`(i64::MAX)`.
    #[cfg(test)]
    pub(crate) fn checks_made(&self) -> i64 {
        i64::MAX - self.checks_before_expiry.load(Ordering::SeqCst)
    }

    /// Exhausts the budget now, as if the deadline had passed.
    #[cfg(test)]
    pub(crate) fn expire(&self) {
        self.exhausted.store(true, Ordering::Relaxed);
    }

    /// Whether the budget has run out. Once true it stays true, and the report
    /// records that the budget was exhausted.
    pub(crate) fn expired(&self) -> bool {
        if self.exhausted.load(Ordering::Relaxed) {
            return true;
        }
        #[cfg(test)]
        let forced = self.checks_before_expiry.fetch_sub(1, Ordering::SeqCst) <= 0;
        #[cfg(not(test))]
        let forced = false;
        let expired = forced || Instant::now() >= self.deadline;
        if expired {
            self.exhausted.store(true, Ordering::Relaxed);
        }
        expired
    }

    /// Whether any check observed expiry.
    pub(crate) fn exhausted(&self) -> bool {
        self.exhausted.load(Ordering::Relaxed)
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    pub(crate) fn limit(&self) -> Duration {
        self.limit
    }
}
