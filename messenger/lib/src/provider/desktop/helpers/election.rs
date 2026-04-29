//! Helper election algorithm shared across platform backends.
//!
//! Given a list of registered helpers and a normalized request, the
//! election produces an ordered slice of helpers to attempt. Helpers that
//! return `score == 0` are filtered out. Caller-supplied
//! `prefer_helpers` ordering breaks score ties before the default tie-break
//! order falls back to insertion order.

use std::sync::Arc;

use super::super::request::DesktopNotificationRequest;
use super::{HelperBackend, HelperError, HelperName};

/// Fallback record kept for `helper_fallbacks` metadata when one or more
/// helpers fail before a successful send.
#[derive(Debug, Clone)]
pub(crate) struct HelperAttempt {
    pub name: HelperName,
    /// Full error message captured for diagnostics. Currently retained for
    /// future debug surfaces; the `summary()` text only emits `name:tag`.
    #[allow(dead_code)]
    pub error: String,
    pub tag: &'static str,
}

impl HelperAttempt {
    /// Produce a short `helper:reason` summary used in receipt metadata.
    pub fn summary(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }

    /// Build an attempt record from a [`HelperError`].
    pub fn from_error(name: HelperName, error: &HelperError) -> Self {
        Self {
            name,
            error: error.to_string(),
            tag: error.summary_tag(),
        }
    }
}

/// Order helpers for election against a single dispatch.
///
/// The returned vector contains references to helpers whose `score()` is
/// non-zero, sorted highest score first. Score ties are broken by
/// `prefer_helpers` (caller-supplied) and then by the order helpers appear
/// in `helpers`.
pub(crate) fn elect_helpers<'a>(
    helpers: &'a [Arc<dyn HelperBackend>],
    request: &DesktopNotificationRequest,
    prefer_helpers: &[HelperName],
) -> Vec<&'a Arc<dyn HelperBackend>> {
    let mut scored: Vec<(usize, u8, &Arc<dyn HelperBackend>)> = helpers
        .iter()
        .enumerate()
        .filter_map(|(idx, helper)| {
            let score = helper.score(request);
            (score > 0).then_some((idx, score, helper))
        })
        .collect();

    scored.sort_by(|left, right| {
        let score_cmp = right.1.cmp(&left.1);
        if score_cmp != std::cmp::Ordering::Equal {
            return score_cmp;
        }

        let left_pref = preference_index(prefer_helpers, left.2.name());
        let right_pref = preference_index(prefer_helpers, right.2.name());
        let pref_cmp = left_pref.cmp(&right_pref);
        if pref_cmp != std::cmp::Ordering::Equal {
            return pref_cmp;
        }

        left.0.cmp(&right.0)
    });

    scored.into_iter().map(|(_, _, helper)| helper).collect()
}

fn preference_index(prefer: &[HelperName], name: HelperName) -> usize {
    prefer
        .iter()
        .position(|candidate| *candidate == name)
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{NotificationAction, NotificationUrgency};
    use async_trait::async_trait;

    use crate::provider::desktop::helpers::{HelperBackend, HelperCapabilities};
    use crate::provider::desktop::request::DesktopNotificationReceipt;

    struct FixedScoreHelper {
        helper_name: HelperName,
        notice_score: u8,
        interactive_score: u8,
    }

    #[async_trait]
    impl HelperBackend for FixedScoreHelper {
        fn name(&self) -> HelperName {
            self.helper_name
        }

        fn capabilities(&self) -> HelperCapabilities {
            HelperCapabilities {
                actions: true,
                reply: false,
                image: true,
                sound: true,
                replace: true,
                group: true,
                blocking: false,
            }
        }

        fn score(&self, request: &DesktopNotificationRequest) -> u8 {
            if request.actions.is_empty() {
                self.notice_score
            } else {
                self.interactive_score
            }
        }

        async fn send(
            &self,
            _request: &DesktopNotificationRequest,
        ) -> Result<DesktopNotificationReceipt, HelperError> {
            Err(HelperError::NotPresent)
        }
    }

    fn helper(name: HelperName, notice: u8, interactive: u8) -> Arc<dyn HelperBackend> {
        Arc::new(FixedScoreHelper {
            helper_name: name,
            notice_score: notice,
            interactive_score: interactive,
        })
    }

    fn request(with_actions: bool) -> DesktopNotificationRequest {
        let mut req = DesktopNotificationRequest {
            title: "T".into(),
            body: None,
            subtitle: None,
            app_name: "App".into(),
            icon: None,
            image: None,
            silent: false,
            category: None,
            urgency: NotificationUrgency::Normal,
            timeout_ms: None,
            replace_id: None,
            group_id: None,
            actions: Vec::new(),
            progress: None,
            badge_count: None,
            replace_helper_hint: None,
        };
        if with_actions {
            req.actions.push(NotificationAction {
                id: "ok".into(),
                label: "OK".into(),
            });
        }
        req
    }

    #[test]
    fn elects_highest_score_first() {
        let helpers = [
            helper(HelperName::NotifySend, 60, 60),
            helper(HelperName::Dunstify, 70, 90),
        ];
        let order = elect_helpers(&helpers, &request(false), &[]);
        let names: Vec<_> = order.iter().map(|helper| helper.name()).collect();
        assert_eq!(names, vec![HelperName::Dunstify, HelperName::NotifySend]);
    }

    #[test]
    fn filters_score_zero_helpers() {
        let helpers = [
            helper(HelperName::Dunstify, 0, 0),
            helper(HelperName::NotifySend, 60, 60),
        ];
        let order = elect_helpers(&helpers, &request(false), &[]);
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].name(), HelperName::NotifySend);
    }

    #[test]
    fn empty_when_all_helpers_score_zero() {
        let helpers = [
            helper(HelperName::Dunstify, 0, 0),
            helper(HelperName::NotifySend, 0, 0),
        ];
        let order = elect_helpers(&helpers, &request(false), &[]);
        assert!(order.is_empty());
    }

    #[test]
    fn prefer_helpers_breaks_ties() {
        let helpers = [
            helper(HelperName::Dunstify, 60, 60),
            helper(HelperName::NotifySend, 60, 60),
        ];
        let prefer = [HelperName::NotifySend];
        let order = elect_helpers(&helpers, &request(false), &prefer);
        let names: Vec<_> = order.iter().map(|helper| helper.name()).collect();
        assert_eq!(names, vec![HelperName::NotifySend, HelperName::Dunstify]);
    }

    #[test]
    fn prefer_helpers_does_not_override_score_zero() {
        let helpers = [
            helper(HelperName::Dunstify, 0, 0),
            helper(HelperName::NotifySend, 60, 60),
        ];
        let prefer = [HelperName::Dunstify];
        let order = elect_helpers(&helpers, &request(false), &prefer);
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].name(), HelperName::NotifySend);
    }

    #[test]
    fn interactive_dispatch_uses_interactive_score() {
        let helpers = [
            helper(HelperName::Dunstify, 70, 90),
            helper(HelperName::NotifySend, 60, 60),
        ];
        let order = elect_helpers(&helpers, &request(true), &[]);
        let names: Vec<_> = order.iter().map(|helper| helper.name()).collect();
        assert_eq!(names, vec![HelperName::Dunstify, HelperName::NotifySend]);
    }
}
