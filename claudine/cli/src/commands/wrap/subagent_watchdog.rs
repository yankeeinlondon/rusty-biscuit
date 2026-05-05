//! Re-export bridge: the canonical subagent watchdog state now lives in
//! `exec::subagent_watchdog`. This file preserves backward compatibility
//! for callers that still reference `wrap::subagent_watchdog` until the
//! migration is complete.

pub(crate) use crate::commands::wrap::exec::subagent_watchdog::{SubagentId, WatchdogState};
pub(crate) use crate::commands::wrap::exec::timeouts::TimeoutConfig;
