//! Thread-scoped tokens, first-cause-wins, cleanup depth, and the injected
//! interrupt probe.

use super::*;
use crate::lifecycle::envelope::DiagnosticSnapshot;

fn sibling() -> CancellationCause {
    CancellationCause::SiblingFailure {
        group: "checks".into(),
        leg: "lint".into(),
        error: LifecycleErrorInfo::from_action_failure_with(&NoCodes, "shell", "boom"),
    }
}

struct NoCodes;

impl DiagnosticCatalog for NoCodes {
    fn select(&self, _: &(dyn std::error::Error + 'static)) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_code(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_error_kind(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
}

#[test]
fn the_first_cause_wins_and_a_child_sees_its_parent() {
    let parent = CancellationToken::default();
    let child = parent.child();
    parent.cancel(sibling());
    child.cancel(CancellationCause::Interrupt);

    assert_eq!(child.cause(), Some(sibling()));
    assert_eq!(parent.child().cause(), Some(sibling()));
}

#[test]
fn entering_a_token_binds_this_worker_until_the_scope_drops() {
    let token = CancellationToken::default();
    token.cancel(sibling());
    {
        let _scope = token.enter();
        assert_eq!(cause_with(|| false), Some(sibling()));
        assert!(requested_with(|| false));
        assert_eq!(current().cause(), Some(sibling()));
    }

    assert_eq!(cause_with(|| false), None);
    assert!(!requested_with(|| false));
}

#[test]
fn the_interrupt_probe_answers_when_no_token_has_a_cause() {
    assert_eq!(cause_with(|| true), Some(CancellationCause::Interrupt));
    assert!(requested_with(|| true));
}

#[test]
fn cleanup_scopes_nest_and_suspend_the_stop_check() {
    let outer = CleanupScope::enter();
    let inner = CleanupScope::enter();
    assert!(!requested_with(|| true));
    drop(inner);
    assert!(!requested_with(|| true));
    drop(outer);

    assert!(requested_with(|| true));
}

#[test]
fn a_sibling_failure_reports_the_group_and_leg() {
    let info = sibling().error_with(&NoCodes);

    assert_eq!(info.msg, "group `checks` leg `lint` failed: boom");
    assert!(info.snapshot.is_none());
}
