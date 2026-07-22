//! Which provider identity terminal recovery negotiates under.
//!
//! Review-10 finding 3. `classify_attempt_phase` used to reload the provider and
//! profile from the invocation-fixed `state.run`, so a retry that switched
//! providers ran its recovery under the *opening* provider: the resume admission
//! asked the wrong profile's capability, and an unowned hand-off named the wrong
//! wrapper in its remediation command. Both values now come out of
//! [`ExecutedHarnessAttempt`], which carries what the attempt actually launched.
//!
//! These rows pin what the forwarded profile then decides. The wiring itself is
//! guarded at the source level by
//! `cli/tests/composition_seams.rs::classification_reads_no_invocation_fixed_launch_identity`,
//! and end to end by the `level2_lifecycle_switch_*` rows in
//! `cli/tests/level2_lifecycle_control.rs`.

use super::*;

/// A profile that cannot resume, standing in for a provider whose capability
/// differs from the opening one's.
///
/// A double rather than a real profile because all ten shipped providers
/// currently report `supports_resume() == true` (`ResumeSupport::FirstClass` or
/// `Partial` in each `provider/*/data.rs`). The capability direction is
/// therefore latent today — reachable the moment any provider is added without
/// resume, or any existing one is downgraded — and a double is the only way to
/// hold the seam honest until then.
struct NonResumableProfile(Provider);

impl crate::commands::wrap::profile::WrapperProfile for NonResumableProfile {
    fn provider(&self) -> Provider {
        self.0
    }
    fn supports_resume(&self) -> bool {
        false
    }
    /// Never reached: no row here delivers a prompt. Delegating to a real
    /// profile keeps the double from encoding a second prompt-placement policy
    /// that could drift from the one production uses.
    fn prompt_delivery(
        &self,
        args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<crate::commands::wrap::profile::PromptDelivery> {
        crate::commands::wrap::profile::profile_for_provider(self.0)
            .expect("the double stands in for a real provider")
            .prompt_delivery(args, prompt, non_interactive)
    }
}

struct ResumeAdmission {
    action: TerminalControlAction,
    attempt_number: u32,
}

/// Dispatch a `success`-stack `resume` under `profile`, with a live session.
///
/// Models the live call site after a switched attempt succeeded: the provider
/// launched, a session exists, and the stack asks to resume it.
fn dispatch_resume_under(
    profile: &dyn crate::commands::wrap::profile::WrapperProfile,
) -> ResumeAdmission {
    let fx = fixture(serde_json::json!({}));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    guard.mark_provider_launched();
    let mut state = prompt_state(&fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();

    let outcome = outcome_with(StackControl::Resume {
        message: "keep going".to_string(),
        max_attempts: 1,
    });
    let action = dispatch_terminal_control(
        &outcome,
        1,
        active.iteration_mut(),
        Some("sess-switched"),
        profile,
        profile.provider(),
        &mut state,
        &fx.materialized,
        &mut guard,
        &ledger(&fx.source_path),
        &fx.term,
        false,
    );
    ResumeAdmission {
        attempt_number: active.iteration().attempt().number(),
        action,
    }
}

/// The switched-to provider can resume, so the resume is admitted — even though
/// the opening provider's capability is irrelevant to the live session.
#[test]
fn a_resume_is_admitted_by_the_executed_attempts_profile() {
    let admitted = dispatch_resume_under(
        crate::commands::wrap::profile::profile_for_provider(Provider::Codex)
            .expect("codex profile exists"),
    );

    assert!(
        matches!(admitted.action, TerminalControlAction::Continue),
        "a resume-capable profile admits the resume, got {:?}",
        admitted.action,
    );
    assert_eq!(
        admitted.attempt_number, 2,
        "the admitted resume advanced the provider-attempt slice",
    );
}

/// The reverse direction: the profile the attempt launched under cannot resume,
/// so the resume is refused and the diagnostic names *that* provider.
///
/// Naming is the load-bearing half — a refusal that named the opening provider
/// sends the user to fix a document property that is no longer in play.
#[test]
fn a_resume_is_refused_naming_the_executed_attempts_provider() {
    let refused = dispatch_resume_under(&NonResumableProfile(Provider::Goose));

    let TerminalControlAction::Abort(error) = refused.action else {
        panic!(
            "a profile that cannot resume must refuse, got {:?}",
            refused.action
        );
    };
    let message = error.to_string();
    assert!(
        message.contains("does not support session resume"),
        "the refusal is the resume-support gate's own, got {message}",
    );
    assert!(
        message.contains(&Provider::Goose.to_string()),
        "the refusal names the provider whose session is live, got {message}",
    );
    assert_eq!(
        refused.attempt_number, 1,
        "a refused resume must not advance the provider-attempt slice",
    );
}

/// The unowned-hand-off remediation command is built from the provider it is
/// given, in both directions.
///
/// This is the reachable half of finding 3 today: the direct wrapper
/// passthrough has no owning coordinator, so a `success`/`failure` stack proxy
/// is refused with a "run this instead" command. Sourcing the provider from the
/// invocation told a user whose retry had moved to Codex to re-run
/// `claudine goose`.
#[test]
fn the_unowned_handoff_command_names_the_provider_it_is_given() {
    let source = Path::new("/tmp/CLAUDE.md");
    let target = Path::new("/tmp/target.md");
    let request = claudine::composition::EvaluatedProxyRequest::new(
        target.display().to_string(),
        indexmap::IndexMap::new(),
        claudine::composition::ProxyProvenance::new(
            source.to_path_buf(),
            claudine::composition::ActionLocation::new(LifecycleSignal::Success, 0, 0),
            vec![source.to_path_buf()],
        ),
    );

    for (provider, expected) in [
        (Provider::Codex, "claudine codex"),
        (Provider::Goose, "claudine goose"),
    ] {
        let error = handoff_without_owning_coordinator(&request, provider);
        let claudine::composition::CompositionError::LifecycleProxyWithoutOwningCoordinator {
            command,
            ..
        } = &error
        else {
            panic!("expected LifecycleProxyWithoutOwningCoordinator, got {error:?}");
        };
        assert_eq!(command, expected);
    }
}
