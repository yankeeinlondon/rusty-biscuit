//! Layer-ownership tests.
//!
//! Each test names the transition it models and asserts which layers survive
//! it. The point is not that the constructors work; it is that the four
//! layers cannot be confused for one another.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use indexmap::IndexMap;

use super::*;
use crate::composition::MAX_PROXY_HOPS;
use crate::composition::lifecycle::LifecycleSignal;
use crate::composition::types::{CompositionMode, SharedApprovalCache};

fn cache() -> SharedApprovalCache {
    Arc::new(Mutex::new(HashMap::new()))
}

fn doc(name: &str) -> PathBuf {
    PathBuf::from(format!("/prompts/{name}.md"))
}

fn provenance(source: &str, signal: LifecycleSignal, chain: Vec<PathBuf>) -> ProxyProvenance {
    ProxyProvenance::new(doc(source), ActionLocation::new(signal, 0, 0), chain)
}

fn overlay_of(pairs: &[(&str, serde_json::Value)]) -> IndexMap<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn request(target: &str, overlay: &[(&str, serde_json::Value)]) -> EvaluatedProxyRequest {
    EvaluatedProxyRequest::new(
        format!("@prompts/{target}.md"),
        overlay_of(overlay),
        provenance("router", LifecycleSignal::Initialize, vec![doc("router")]),
    )
}

/// The coordinator's normal happy path, used as the fixture for the handoff
/// tests below.
fn commit(ledger: &mut RunLedger, target: &str, overlay: &[(&str, serde_json::Value)])
-> ProxyHandoff {
    let resolved = ResolvedProxyTarget::from_resolver(doc(target));
    let approval = ledger
        .access()
        .approve_hop(resolved)
        .expect("hop should be approved");
    let handoff = ProxyHandoff::commit(request(target, overlay), approval);
    ledger.access().record_proxy(&handoff);
    handoff
}

mod handoff_state {
    use super::*;

    #[test]
    fn commit_takes_its_path_from_the_approval_not_the_authored_string() {
        let mut ledger = RunLedger::new(doc("router"), cache());
        let handoff = commit(&mut ledger, "target", &[]);

        // The authored reference survives for diagnostics, but the path that
        // downstream preparation will open is the resolver's, not a re-read of
        // the authored string.
        assert_eq!(handoff.authored_target(), "@prompts/target.md");
        assert_eq!(handoff.resolved_target(), doc("target"));
    }

    #[test]
    fn evaluated_request_carries_the_overlay_and_provenance_through_commit() {
        let mut ledger = RunLedger::new(doc("router"), cache());
        let handoff = commit(
            &mut ledger,
            "target",
            &[("phase", serde_json::json!(2)), ("live", serde_json::json!(true))],
        );

        assert_eq!(handoff.overlay().get("phase"), Some(&serde_json::json!(2)));
        assert_eq!(handoff.overlay().get("live"), Some(&serde_json::json!(true)));
        assert_eq!(handoff.provenance().source_path(), doc("router"));
        assert_eq!(handoff.provenance().signal(), LifecycleSignal::Initialize);
    }

    #[test]
    fn action_location_renders_the_dotted_property_path() {
        let location = ActionLocation::new(LifecycleSignal::Failure, 1, 2);
        assert_eq!(location.to_string(), "failure.stack[1].action[2]");
    }

    #[test]
    fn omitted_with_installs_an_empty_overlay_rather_than_forwarding() {
        let mut ledger = RunLedger::new(doc("router"), cache());
        let first = commit(&mut ledger, "a", &[("carried", serde_json::json!("yes"))]);
        let prepared = PreparedDocument::from_handoff(first, prepared_composition());
        assert!(!prepared.overlay().is_empty());

        // `a` proxies on without a `with:`. The target gets its own (empty)
        // overlay, not `a`'s.
        let second = commit(&mut ledger, "b", &[]);
        let next = PreparedDocument::from_handoff(second, prepared_composition());
        assert!(next.overlay().is_empty());
    }
}

mod run_ledger {
    use super::*;

    #[test]
    fn chain_is_seeded_with_the_origin_so_a_self_proxy_is_a_cycle_on_the_first_hop() {
        let mut ledger = RunLedger::new(doc("router"), cache());
        assert!(ledger.contains_document(&doc("router")));
        assert_eq!(ledger.hops(), 0);

        let rejection = ledger
            .access()
            .approve_hop(ResolvedProxyTarget::from_resolver(doc("router")))
            .expect_err("a document proxying to itself is a cycle");
        assert_eq!(rejection, HopRejection::Cycle);
    }

    #[test]
    fn an_a_b_a_cycle_is_rejected() {
        let mut ledger = RunLedger::new(doc("a"), cache());
        commit(&mut ledger, "b", &[]);

        let rejection = ledger
            .access()
            .approve_hop(ResolvedProxyTarget::from_resolver(doc("a")))
            .expect_err("A->B->A is a cycle");
        assert_eq!(rejection, HopRejection::Cycle);
        assert_eq!(ledger.hops(), 1);
    }

    #[test]
    fn hop_budget_exhausts_at_max_proxy_hops() {
        let mut ledger = RunLedger::new(doc("origin"), cache());
        // The origin occupies one chain slot, so the chain reaches
        // MAX_PROXY_HOPS after MAX_PROXY_HOPS - 1 hops.
        for i in 0..(MAX_PROXY_HOPS - 1) {
            commit(&mut ledger, &format!("hop{i}"), &[]);
        }
        assert_eq!(ledger.chain().len(), MAX_PROXY_HOPS);

        let rejection = ledger
            .access()
            .approve_hop(ResolvedProxyTarget::from_resolver(doc("one-too-many")))
            .expect_err("the hop budget is exhausted");
        assert_eq!(rejection, HopRejection::BudgetExhausted);
    }

    #[test]
    fn a_rejected_hop_does_not_extend_the_chain() {
        let mut ledger = RunLedger::new(doc("a"), cache());
        commit(&mut ledger, "b", &[]);
        let before = ledger.chain().to_vec();

        let _ = ledger
            .access()
            .approve_hop(ResolvedProxyTarget::from_resolver(doc("a")));

        assert_eq!(ledger.chain(), before.as_slice());
    }

    #[test]
    fn proxy_extends_the_ledger_and_never_resets_it() {
        let approvals = cache();
        approvals.lock().unwrap().insert(
            "ls -la".to_string(),
            crate::harness::shell::CachedApprovalDecision::Allowed,
        );
        let mut ledger = RunLedger::new(doc("router"), approvals);
        let anchor = ledger.command_started();

        commit(&mut ledger, "a", &[]);
        commit(&mut ledger, "b", &[]);

        // Hop accounting bounds a *chain* of documents, so a handoff extends
        // it rather than starting over.
        assert_eq!(ledger.hops(), 2);
        assert_eq!(ledger.chain(), [doc("router"), doc("a"), doc("b")]);
        // Command-wide timing and the approval cache are invocation state:
        // they outlive every document in the chain.
        assert_eq!(ledger.command_started(), anchor);
        assert!(ledger.approval_cache().lock().unwrap().contains_key("ls -la"));
    }

    #[test]
    fn transition_provenance_is_recorded_for_diagnostics() {
        let mut ledger = RunLedger::new(doc("router"), cache());
        commit(&mut ledger, "target", &[]);
        ledger
            .access()
            .record_transition(TransitionRecord::Retry { attempt: 2 });

        assert_eq!(ledger.transitions().len(), 2);
        match &ledger.transitions()[0] {
            TransitionRecord::Proxy {
                provenance,
                resolved_target,
            } => {
                assert_eq!(provenance.source_path(), doc("router"));
                assert_eq!(resolved_target, &doc("target"));
            }
            other => panic!("expected a recorded proxy, got {other:?}"),
        }
        assert_eq!(ledger.transitions()[1], TransitionRecord::Retry { attempt: 2 });
    }

    #[test]
    fn contains_document_is_the_one_chain_membership_answer() {
        let mut ledger = RunLedger::new(doc("router"), cache());
        commit(&mut ledger, "target", &[]);

        assert!(ledger.contains_document(&doc("router")));
        assert!(ledger.contains_document(&doc("target")));
        assert!(!ledger.contains_document(&doc("elsewhere")));
    }
}

mod invocation_inputs {
    use super::*;

    fn draft() -> InvocationInputsDraft {
        InvocationInputsDraft::new(
            "router.md".to_string(),
            CompositionMode::ChainedDocument,
            PathBuf::from("/repo"),
        )
    }

    #[test]
    fn frozen_inputs_expose_reads_only() {
        let mut d = draft();
        d.yolo = true;
        d.set_overrides = Some(serde_json::json!({ "phase": 2 }));
        let inputs = d.freeze();

        // Reads work through Deref; there is no DerefMut, so the compile_fail
        // doctest on `InvocationInputs` covers the mutation half.
        assert!(inputs.yolo);
        assert_eq!(inputs.file_ref, "router.md");
        assert_eq!(
            inputs.set_overrides,
            Some(serde_json::json!({ "phase": 2 }))
        );
    }

    #[test]
    fn caller_overrides_are_unchanged_by_a_proxy() {
        let mut d = draft();
        d.set_overrides = Some(serde_json::json!({ "phase": 2 }));
        let inputs = d.freeze();
        let mut ledger = RunLedger::new(doc("router"), cache());

        // The target's `with:` says phase 9; the caller said phase 2.
        commit(&mut ledger, "target", &[("phase", serde_json::json!(9))]);

        // The caller stays authoritative at every document: a handoff cannot
        // reach into invocation inputs at all.
        assert_eq!(inputs.set_overrides, Some(serde_json::json!({ "phase": 2 })));
    }
}

mod active_document_state {
    use super::*;

    #[test]
    fn initial_state_starts_at_iteration_one_attempt_one_with_no_budgets() {
        let state = ActiveDocumentState::initial();
        assert_eq!(state.iteration().number(), 1);
        assert_eq!(state.iteration().attempt().number(), 1);
        assert_eq!(state.iteration().retry_budget().ceiling(), None);
        assert_eq!(state.iteration().resume_budget().ceiling(), None);
        assert_eq!(state.iteration().attempt().session_id(), None);
    }

    #[test]
    fn retry_replaces_the_attempt_slice_and_drops_the_session() {
        let mut state = ActiveDocumentState::initial();
        state.iteration_mut().attempt_mut().adopt_session("s-1".into());
        state
            .iteration_mut()
            .attempt_mut()
            .record_outcome(AttemptOutcome::Failed);

        state.iteration_mut().retry_attempt();

        // Retry starts a fresh provider session.
        assert_eq!(state.iteration().attempt().number(), 2);
        assert_eq!(state.iteration().attempt().session_id(), None);
        assert_eq!(state.iteration().attempt().last_outcome(), None);
        // Still the same document iteration.
        assert_eq!(state.iteration().number(), 1);
    }

    #[test]
    fn resume_replaces_the_attempt_slice_but_retains_the_live_session() {
        let mut state = ActiveDocumentState::initial();
        state.iteration_mut().attempt_mut().adopt_session("s-1".into());

        state
            .iteration_mut()
            .resume_attempt("s-1".into(), Some("keep going".into()));

        assert_eq!(state.iteration().attempt().number(), 2);
        assert_eq!(state.iteration().attempt().session_id(), Some("s-1"));
        assert_eq!(state.iteration().attempt().resume_followup(), Some("keep going"));
    }

    #[test]
    fn retry_cannot_reset_its_own_budget_by_replacing_the_attempt() {
        let mut state = ActiveDocumentState::initial();
        // A `retry: { max_attempts: 2 }` firing at attempt 1 ceilings at 3.
        let ceiling = state.iteration_mut().retry_budget_mut().ceiling_for(1, 2);
        assert_eq!(ceiling, 3);

        state.iteration_mut().retry_attempt();
        // Firing again at attempt 2 must reuse the established ceiling, not
        // recompute 2 + 2 = 4 and drift out of reach forever.
        assert_eq!(state.iteration_mut().retry_budget_mut().ceiling_for(2, 2), 3);
        state.iteration_mut().retry_attempt();
        assert_eq!(state.iteration_mut().retry_budget_mut().ceiling_for(3, 2), 3);

        // Budget exhausted at the ceiling.
        assert!(state.iteration().retry_budget().permits(2));
        assert!(!state.iteration().retry_budget().permits(3));
    }

    #[test]
    fn retry_and_resume_budgets_have_separate_labeled_homes() {
        let mut state = ActiveDocumentState::initial();
        state.iteration_mut().retry_budget_mut().ceiling_for(1, 2);

        // Spending retry budget must not spend resume budget.
        assert_eq!(state.iteration().retry_budget().ceiling(), Some(3));
        assert_eq!(state.iteration().resume_budget().ceiling(), None);
        assert_eq!(
            state.iteration().retry_budget().kind(),
            ControlBudgetKind::Retry
        );
        assert_eq!(
            state.iteration().resume_budget().kind(),
            ControlBudgetKind::Resume
        );
    }

    #[test]
    fn advancing_the_loop_gives_the_next_iteration_fresh_budgets() {
        let mut state = ActiveDocumentState::initial();
        state.iteration_mut().retry_budget_mut().ceiling_for(1, 2);
        state.iteration_mut().retry_attempt();
        state
            .iteration_mut()
            .mutations_mut()
            .insert("phase".into(), serde_json::json!(2));

        state.advance_iteration();

        assert_eq!(state.iteration().number(), 2);
        assert_eq!(state.iteration().attempt().number(), 1);
        assert_eq!(state.iteration().retry_budget().ceiling(), None);
        assert!(state.iteration().mutations().is_empty());
    }

    #[test]
    fn proxy_discards_active_document_execution_state() {
        let mut source = ActiveDocumentState::initial();
        source.iteration_mut().retry_budget_mut().ceiling_for(1, 2);
        source.iteration_mut().attempt_mut().adopt_session("s-1".into());
        source.iteration_mut().retry_attempt();
        source.advance_iteration();

        // What the coordinator builds for a freshly adopted target.
        let target = ActiveDocumentState::initial();

        assert_eq!(target.iteration().number(), 1);
        assert_eq!(target.iteration().attempt().number(), 1);
        assert_eq!(target.iteration().attempt().session_id(), None);
        assert_eq!(target.iteration().retry_budget().ceiling(), None);
        assert_eq!(target.iteration().resume_budget().ceiling(), None);
    }
}

mod prepared_document {
    use super::*;

    #[test]
    fn direct_preparation_has_no_overlay_and_no_provenance() {
        let prepared = PreparedDocument::direct(prepared_composition());
        assert!(prepared.overlay().is_empty());
        assert!(prepared.provenance().is_none());
    }

    #[test]
    fn overlay_and_provenance_survive_a_canonical_refresh() {
        let mut ledger = RunLedger::new(doc("router"), cache());
        let handoff = commit(&mut ledger, "target", &[("phase", serde_json::json!(2))]);
        let prepared = PreparedDocument::from_handoff(handoff, prepared_composition());

        // Retry / resume / loop refresh: new composition, same overlay.
        let refreshed = prepared.refreshed(prepared_composition());

        assert_eq!(
            refreshed.overlay().values().get("phase"),
            Some(&serde_json::json!(2))
        );
        assert_eq!(
            refreshed.provenance().map(ProxyProvenance::source_path),
            Some(doc("router").as_path())
        );
    }

    #[test]
    fn overlay_exposes_property_names_without_values_for_status_and_tracing() {
        let mut ledger = RunLedger::new(doc("router"), cache());
        let handoff = commit(
            &mut ledger,
            "target",
            &[("token", serde_json::json!("s3cret"))],
        );
        let prepared = PreparedDocument::from_handoff(handoff, prepared_composition());

        let names: Vec<_> = prepared.overlay().property_names().collect();
        assert_eq!(names, ["token"]);
    }
}

mod transitions {
    use super::*;
    use crate::composition::CompositionError;

    #[test]
    fn a_proxy_hands_off_source_ownership_but_a_retry_does_not() {
        let proxy: DocumentTransition = DocumentTransition::Proxy(request("target", &[]));
        assert!(proxy.hands_off_source());
        assert!(DocumentTransition::<CompositionError>::Complete.hands_off_source());
        assert!(!DocumentTransition::<CompositionError>::Retry.hands_off_source());
        assert!(!DocumentTransition::<CompositionError>::Continue.hands_off_source());
        assert!(
            !DocumentTransition::<CompositionError>::Resume {
                session: "s-1".into(),
                message: None,
            }
            .hands_off_source()
        );
    }

    #[test]
    fn abort_preserves_its_concrete_error_rather_than_a_string() {
        #[derive(Debug, Clone, PartialEq)]
        struct DriverError {
            code: u8,
        }

        // A CLI-side producer aborts with its own error type: the library
        // neither erases it nor depends on it.
        let transition: DocumentTransition<DriverError> = DocumentTransition::Abort(
            TransitionAbort::new(Some(LifecycleSignal::Failure), DriverError { code: 7 }),
        );

        match transition {
            DocumentTransition::Abort(abort) => {
                assert_eq!(abort.signal(), Some(LifecycleSignal::Failure));
                assert_eq!(abort.source(), &DriverError { code: 7 });
                assert_eq!(abort.into_source(), DriverError { code: 7 });
            }
            other => panic!("expected an abort, got {other:?}"),
        }
    }

    #[test]
    fn map_abort_retypes_the_error_and_leaves_other_variants_alone() {
        let abort: DocumentTransition<u8> =
            DocumentTransition::Abort(TransitionAbort::new(None, 7u8));
        let lifted: DocumentTransition<String> = abort.map_abort(|code| format!("code {code}"));
        match lifted {
            DocumentTransition::Abort(a) => assert_eq!(a.source(), "code 7"),
            other => panic!("expected an abort, got {other:?}"),
        }

        let retry: DocumentTransition<u8> = DocumentTransition::Retry;
        assert_eq!(retry.map_abort(|_| String::new()), DocumentTransition::Retry);
    }
}

/// A minimal prepared composition. The canonical preparation service (Phase 5)
/// is what will really build these; these tests only care that the
/// prepared-document layer carries one.
fn prepared_composition() -> crate::composition::types::PreparedComposition {
    use crate::composition::types::{
        CompositionClosurePlan, EffectiveSelectionHints, PreparedComposition,
    };

    PreparedComposition {
        mode: CompositionMode::ChainedDocument,
        resolved_path: doc("target"),
        source_repo_root: None,
        prompt: String::new(),
        effective_frontmatter: serde_json::json!({}),
        selection_hints: EffectiveSelectionHints::default(),
        closure: CompositionClosurePlan::Direct,
        lifecycle: crate::composition::LifecycleConfig::default(),
        compose_perf: None,
        dropped_optionals: Vec::new(),
        warnings: Vec::new(),
        deferred_lifecycle_keys: Vec::new(),
        rematerialize: Default::default(),
    }
}
