//! Tests for wrapper termination policy and the platform wait/escalation loops.
//!
//! Split to match the module tree:
//! - [`projection`] — [`apply_early_termination_to_summary`],
//!   [`early_termination_guard_context`], and message drift.
//! - [`reasons`] — [`early_termination_process_outcome`],
//!   [`trip_to_early_termination`], and [`watchdog_request_to_early_termination`].
//! - [`wait`] — the platform wait-loop escalation, completion, and watchdog
//!   behavior.

use super::*;
use claudine::stream::logs::{EarlyTermination, StalledGenerationContext};
use claudine::stream::summary::StreamExecutionSummary;
use std::time::{Duration, Instant};

mod projection;
mod reasons;
mod wait;
