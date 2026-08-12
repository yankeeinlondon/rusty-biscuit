//! Consumer-side projections of observed signals (design §Sink).
//!
//! Policy facts that are universal per taxonomy kind live here (e.g.
//! `rate_limited` ⇒ throttled); situational interpretation stays in the
//! consumer. Existing consumers migrate by swapping their input type from
//! parser-computed structs to these projections, not by changing logic.

use claudine_catalog_types::{Quantity, SignalEvent, Unit};

use super::sink::ObservedSignal;
use crate::stream::summary::RateLimitInfo;

/// Project the run's throttle posture from rate-limit-class observations
/// (`rate_limited`, `usage_cap_approaching`, `usage_capped`).
///
/// The LAST relevant observation wins, mirroring the Claude parser's
/// last-`rate_limit_event`-wins behavior (`signals` is in first-seen
/// order). `None` when no rate-limit-class signal was observed — unlike
/// the parser, which records `is_throttled: Some(false)` events; consumers
/// keep the parser value as fallback during the migration bridge.
///
/// Field semantics follow the parser's `resolved_*` accessors
/// (`stream/protocol/claude.rs`): `rate_limited` and `usage_capped` are
/// throttled states, `usage_cap_approaching` is an advisory
/// (`is_throttled: Some(false)`); `message` carries only the raw provider
/// message — rendering synthesized messages is a consumer concern.
pub fn rate_limit_info(signals: &[ObservedSignal]) -> Option<RateLimitInfo> {
    let mut info: Option<RateLimitInfo> = None;
    for signal in signals {
        match &signal.event {
            SignalEvent::RateLimited {
                reset_at,
                retry_after,
                message,
                ..
            } => {
                info = Some(RateLimitInfo {
                    is_throttled: Some(true),
                    retry_after_ms: retry_after.as_ref().and_then(quantity_to_millis),
                    message: message.clone(),
                    reset_at: *reset_at,
                });
            }
            SignalEvent::UsageCapApproaching {
                resets_at, message, ..
            } => {
                info = Some(RateLimitInfo {
                    is_throttled: Some(false),
                    retry_after_ms: None,
                    message: message.clone(),
                    reset_at: *resets_at,
                });
            }
            SignalEvent::UsageCapped {
                lifts_at, message, ..
            } => {
                info = Some(RateLimitInfo {
                    is_throttled: Some(true),
                    retry_after_ms: None,
                    message: message.clone(),
                    reset_at: *lifts_at,
                });
            }
            _ => {}
        }
    }
    info
}

/// Duration quantity → whole milliseconds. Non-duration units carry no
/// wait semantics and project to `None`.
fn quantity_to_millis(quantity: &Quantity) -> Option<u64> {
    let millis = match quantity.unit {
        Unit::DurationMillis => quantity.value,
        Unit::DurationSecs => quantity.value * 1_000.0,
        _ => return None,
    };
    (millis.is_finite() && millis >= 0.0).then_some(millis.round() as u64)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use claudine_catalog_types::{CapScope, SignalSource};

    use super::*;

    fn observed(event: SignalEvent) -> ObservedSignal {
        ObservedSignal {
            event,
            source: SignalSource::Stream,
            first_seen: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            occurrences: 1,
            context: Default::default(),
        }
    }

    #[test]
    fn empty_signals_project_to_none() {
        assert_eq!(rate_limit_info(&[]), None);
    }

    #[test]
    fn unrelated_signals_project_to_none() {
        let signals = vec![observed(SignalEvent::NoFunds {
            message: Some("no credits".into()),
            top_up_url: None,
        })];
        assert_eq!(rate_limit_info(&signals), None);
    }

    #[test]
    fn rate_limited_projects_throttled_with_retry_after_millis() {
        let signals = vec![observed(SignalEvent::RateLimited {
            status_code: Some(429),
            reset_at: None,
            retry_after: Some(Quantity {
                value: 5_000.0,
                unit: Unit::DurationMillis,
            }),
            message: Some("slow".into()),
        })];
        assert_eq!(
            rate_limit_info(&signals),
            Some(RateLimitInfo {
                is_throttled: Some(true),
                retry_after_ms: Some(5_000),
                message: Some("slow".into()),
                reset_at: None,
            })
        );
    }

    #[test]
    fn retry_after_in_seconds_converts_to_millis() {
        let signals = vec![observed(SignalEvent::RateLimited {
            status_code: None,
            reset_at: None,
            retry_after: Some(Quantity {
                value: 2.5,
                unit: Unit::DurationSecs,
            }),
            message: None,
        })];
        assert_eq!(
            rate_limit_info(&signals).unwrap().retry_after_ms,
            Some(2_500)
        );
    }

    #[test]
    fn usage_capped_projects_throttled_with_lifts_at_as_reset() {
        let lifts_at: DateTime<Utc> = Utc.timestamp_opt(1_712_000_000, 0).unwrap();
        let signals = vec![observed(SignalEvent::UsageCapped {
            model: CapScope::All,
            timeframe: None,
            remaining: None,
            lifts_at: Some(lifts_at),
            message: Some("limit reached".into()),
        })];
        assert_eq!(
            rate_limit_info(&signals),
            Some(RateLimitInfo {
                is_throttled: Some(true),
                retry_after_ms: None,
                message: Some("limit reached".into()),
                reset_at: Some(lifts_at),
            })
        );
    }

    #[test]
    fn last_relevant_observation_wins() {
        let signals = vec![
            observed(SignalEvent::UsageCapApproaching {
                model: CapScope::All,
                timeframe: None,
                remaining: None,
                resets_at: None,
                message: Some("approaching".into()),
            }),
            observed(SignalEvent::RateLimited {
                status_code: None,
                reset_at: None,
                retry_after: None,
                message: Some("throttled now".into()),
            }),
        ];
        let info = rate_limit_info(&signals).unwrap();
        assert_eq!(info.is_throttled, Some(true));
        assert_eq!(info.message.as_deref(), Some("throttled now"));
    }
}
