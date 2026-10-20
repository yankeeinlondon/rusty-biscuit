//! The shared deadline for an operation spanning several sends.
//!
//! A [`Deadline`] is created once and passed to every send of the operation,
//! through [`Call::deadline`](crate::Call::deadline) or, for a raw send,
//! [`CallOptions`](crate::CallOptions). Its clones share one
//! expiration on the monotonic clock and one count of completed logical
//! sends. Each send under a deadline is bounded by the smaller of the time
//! remaining and the client's per-request timeout; once it has expired, no
//! further request is sent.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::time::Instant;

/// A monotonic deadline shared by every clone.
#[derive(Clone)]
pub struct Deadline {
    shared: Arc<Shared>,
}

struct Shared {
    created: Instant,
    expires_at: Instant,
    completed: AtomicU64,
}

impl Deadline {
    /// A deadline `budget` from now. A budget too large to represent never
    /// expires in practice.
    pub fn after(budget: Duration) -> Self {
        let created = Instant::now();
        let expires_at = created
            .checked_add(budget)
            .unwrap_or_else(|| created + Duration::from_secs(100 * 365 * 24 * 60 * 60));
        Self {
            shared: Arc::new(Shared {
                created,
                expires_at,
                completed: AtomicU64::new(0),
            }),
        }
    }

    /// When the deadline expires.
    pub fn expires_at(&self) -> Instant {
        self.shared.expires_at
    }

    /// Time left before expiry; zero once expired.
    pub fn remaining(&self) -> Duration {
        self.shared.expires_at.saturating_duration_since(Instant::now())
    }

    /// Whether the deadline has expired.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.shared.expires_at
    }

    /// Time since the deadline was created.
    pub fn elapsed(&self) -> Duration {
        self.shared.created.elapsed()
    }

    /// Logical sends completed under this deadline, by any clone.
    ///
    /// A send counts once its outcome is classified, an HTTP error status
    /// included; redirect hops, sends that failed before a response head,
    /// and dropped sends do not count.
    pub fn completed_requests(&self) -> u64 {
        self.shared.completed.load(Ordering::Acquire)
    }

    pub(crate) fn record_completed(&self) {
        self.shared.completed.fetch_add(1, Ordering::AcqRel);
    }
}

/// The single expiration of one logical send.
///
/// It is computed once when the send starts, as the earlier of the client's
/// per-request timeout and the shared deadline, and every later wait of the
/// send (the response head, a redirect hop, a buffered body read) uses the
/// same instant.
#[derive(Debug, Clone)]
pub(crate) struct Expiration {
    pub(crate) at: Instant,
    pub(crate) limit: Limit,
    /// The per-request timeout, used as the idle bound between stream reads.
    pub(crate) timeout: Duration,
    /// The shared deadline's expiry, which still bounds a whole stream.
    pub(crate) deadline_at: Option<Instant>,
}

/// Which bound an [`Expiration`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Limit {
    RequestTimeout,
    SharedDeadline,
}

impl Expiration {
    /// The expiration of a send starting at `start`.
    pub(crate) fn start(start: Instant, timeout: Duration, deadline: Option<&Deadline>) -> Self {
        let timeout_at = start.checked_add(timeout).unwrap_or_else(|| start + Duration::from_secs(100 * 365 * 24 * 60 * 60));
        let deadline_at = deadline.map(Deadline::expires_at);
        match deadline_at {
            Some(deadline_at) if deadline_at < timeout_at => Self {
                at: deadline_at,
                limit: Limit::SharedDeadline,
                timeout,
                deadline_at: Some(deadline_at),
            },
            _ => Self {
                at: timeout_at,
                limit: Limit::RequestTimeout,
                timeout,
                deadline_at,
            },
        }
    }
}

impl fmt::Debug for Deadline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Deadline")
            .field("remaining", &self.remaining())
            .field("completed_requests", &self.completed_requests())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_clone_counts_into_one_total() {
        let deadline = Deadline::after(Duration::from_secs(60));
        let clone = deadline.clone();
        deadline.record_completed();
        clone.record_completed();
        assert_eq!(deadline.completed_requests(), 2);
        assert_eq!(clone.completed_requests(), 2);
        assert_eq!(Deadline::after(Duration::from_secs(60)).completed_requests(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn the_earlier_bound_limits_a_send() {
        let start = Instant::now();
        let timeout = Duration::from_secs(30);

        let alone = Expiration::start(start, timeout, None);
        assert_eq!((alone.at, alone.limit, alone.deadline_at), (start + timeout, Limit::RequestTimeout, None));

        let short = Deadline::after(Duration::from_secs(5));
        let bounded = Expiration::start(start, timeout, Some(&short));
        assert_eq!((bounded.at, bounded.limit), (short.expires_at(), Limit::SharedDeadline));

        let long = Deadline::after(Duration::from_secs(120));
        let timed = Expiration::start(start, timeout, Some(&long));
        assert_eq!((timed.at, timed.limit), (start + timeout, Limit::RequestTimeout));
        assert_eq!(timed.deadline_at, Some(long.expires_at()), "the deadline still bounds a stream");

        let huge = Expiration::start(start, Duration::MAX, None);
        assert!(huge.at > start, "an unrepresentable timeout saturates instead of panicking");
    }
}
