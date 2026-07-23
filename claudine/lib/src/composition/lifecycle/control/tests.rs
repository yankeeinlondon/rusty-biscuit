use super::*;
use crate::composition::ActionLocation;
use crate::composition::lifecycle::LifecycleSignal;
use indexmap::IndexMap;

fn retry(max: u32, backoff: RetryBackoff, delay: &str) -> StackControl {
    StackControl::Retry {
        max_attempts: max,
        backoff,
        delay: delay.to_string(),
    }
}

#[test]
fn stop_skip_error_resolve_to_stop() {
    for control in [
        StackControl::Stop,
        StackControl::Skip,
        StackControl::Error { reason: None },
    ] {
        assert_eq!(
            decide_control(&control, 1, 0, true, true),
            ControlDispatch::Stop
        );
    }
}

#[test]
fn retry_post_launch_reinvokes_provider() {
    let control = retry(2, RetryBackoff::Fixed, "0s");
    assert_eq!(
        decide_control(&control, 1, 0, false, true),
        ControlDispatch::Retry {
            delay: Duration::ZERO,
            reenter_preflight: false,
        }
    );
}

#[test]
fn retry_pre_launch_re_enters_preflight() {
    let control = retry(1, RetryBackoff::Fixed, "0s");
    assert_eq!(
        decide_control(&control, 1, 0, false, false),
        ControlDispatch::Retry {
            delay: Duration::ZERO,
            reenter_preflight: true,
        }
    );
}

#[test]
fn retry_honors_max_attempts_budget() {
    let control = retry(2, RetryBackoff::Fixed, "0s");
    // First firing at attempt 1 establishes budget = 1 + 2 = 3.
    let budget = control_budget_for(1, 2);
    assert_eq!(budget, 3);
    // attempt 1 and 2 retry; attempt 3 is exhausted.
    assert!(matches!(
        decide_control(&control, 1, budget, false, true),
        ControlDispatch::Retry { .. }
    ));
    assert!(matches!(
        decide_control(&control, 2, budget, false, true),
        ControlDispatch::Retry { .. }
    ));
    assert_eq!(
        decide_control(&control, 3, budget, false, true),
        ControlDispatch::Exhausted
    );
}

#[test]
fn exponential_backoff_doubles_per_retry() {
    let base = Duration::from_secs(5);
    assert_eq!(
        compute_backoff_delay(base, RetryBackoff::Exponential, 0),
        Duration::from_secs(5)
    );
    assert_eq!(
        compute_backoff_delay(base, RetryBackoff::Exponential, 1),
        Duration::from_secs(10)
    );
    assert_eq!(
        compute_backoff_delay(base, RetryBackoff::Exponential, 2),
        Duration::from_secs(20)
    );
}

#[test]
fn fixed_backoff_is_constant() {
    let base = Duration::from_secs(7);
    for index in 0..4 {
        assert_eq!(
            compute_backoff_delay(base, RetryBackoff::Fixed, index),
            base
        );
    }
}

#[test]
fn exponential_retry_dispatch_applies_doubled_delay() {
    // budget = 3, max_attempts = 2, delay 4s exponential.
    // attempt 1: consumed 0 → 4s; attempt 2: consumed 1 → 8s.
    let control = retry(2, RetryBackoff::Exponential, "4s");
    assert_eq!(
        decide_control(&control, 1, 3, false, true),
        ControlDispatch::Retry {
            delay: Duration::from_secs(4),
            reenter_preflight: false,
        }
    );
    assert_eq!(
        decide_control(&control, 2, 3, false, true),
        ControlDispatch::Retry {
            delay: Duration::from_secs(8),
            reenter_preflight: false,
        }
    );
}

#[test]
fn resume_with_session_resumes() {
    let control = StackControl::Resume {
        message: "fix it".to_string(),
        max_attempts: 1,
    };
    assert_eq!(
        decide_control(&control, 1, 0, true, true),
        ControlDispatch::Resume {
            message: "fix it".to_string(),
        }
    );
}

#[test]
fn resume_without_session_errors() {
    let control = StackControl::Resume {
        message: "fix it".to_string(),
        max_attempts: 1,
    };
    assert_eq!(
        decide_control(&control, 1, 0, false, true),
        ControlDispatch::ResumeWithoutSession
    );
}

#[test]
fn resume_honors_max_attempts() {
    let control = StackControl::Resume {
        message: "again".to_string(),
        max_attempts: 1,
    };
    let budget = control_budget_for(1, 1); // 2
    assert!(matches!(
        decide_control(&control, 1, budget, true, true),
        ControlDispatch::Resume { .. }
    ));
    assert_eq!(
        decide_control(&control, 2, budget, true, true),
        ControlDispatch::Exhausted
    );
}

#[test]
fn proxy_dispatches_target() {
    let control = StackControl::Proxy {
        target: "@prompts/other.md".to_string(),
        overlay: IndexMap::new(),
        location: ActionLocation::new(LifecycleSignal::Failure, 0, 0),
    };
    assert_eq!(
        decide_control(&control, 1, 0, false, true),
        ControlDispatch::Proxy {
            target: "@prompts/other.md".to_string(),
        }
    );
}

#[test]
fn recovery_controls_dispatch_event_agnostically() {
    // `decide_control` is event-agnostic: placement (which control is valid
    // in which event) is the parse-time pre-scan's job, so every recovery
    // control dispatches here regardless of the originating event.
    assert!(matches!(
        decide_control(&retry(1, RetryBackoff::Fixed, "0s"), 1, 0, false, true),
        ControlDispatch::Retry {
            reenter_preflight: false,
            ..
        }
    ));

    let resume = StackControl::Resume {
        message: "continue".to_string(),
        max_attempts: 1,
    };
    assert!(matches!(
        decide_control(&resume, 1, 0, true, true),
        ControlDispatch::Resume { .. }
    ));

    let proxy = StackControl::Proxy {
        target: "@x.md".to_string(),
        overlay: IndexMap::new(),
        location: ActionLocation::new(LifecycleSignal::Failure, 0, 0),
    };
    assert!(matches!(
        decide_control(&proxy, 1, 0, false, true),
        ControlDispatch::Proxy { .. }
    ));

    let requeue = StackControl::Defer {
        delay: "5m".to_string(),
        reason: None,
    };
    assert!(matches!(
        decide_control(&requeue, 1, 0, false, true),
        ControlDispatch::Defer { .. }
    ));
}

#[test]
fn requeue_carries_delay_and_reason() {
    let control = StackControl::Defer {
        delay: "5m".to_string(),
        reason: Some("later".to_string()),
    };
    assert_eq!(
        decide_control(&control, 1, 0, false, true),
        ControlDispatch::Defer {
            delay: "5m".to_string(),
            reason: Some("later".to_string()),
        }
    );
}

#[test]
fn parse_delay_handles_units_and_garbage() {
    assert_eq!(parse_delay("5m"), Duration::from_secs(300));
    assert_eq!(parse_delay("30s"), Duration::from_secs(30));
    assert_eq!(parse_delay("0s"), Duration::ZERO);
    assert_eq!(parse_delay(""), Duration::ZERO);
    assert_eq!(parse_delay("not-a-duration"), Duration::ZERO);
}

#[test]
fn resolve_proxy_target_resolves_existing_relative_file() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("prompt.md");
    std::fs::write(&source, "---\n---\n").unwrap();
    let target = dir.path().join("other.md");
    std::fs::write(&target, "---\n---\n").unwrap();

    let resolved = resolve_proxy_target("other.md", &source, None).unwrap();
    assert_eq!(resolved, target);
}

#[test]
fn resolve_proxy_target_resolves_repo_relative() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("sub/prompt.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, "---\n---\n").unwrap();
    let target = dir.path().join("prompts/next.md");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "---\n---\n").unwrap();

    let resolved =
        resolve_proxy_target("@prompts/next.md", &source, Some(dir.path())).unwrap();
    assert_eq!(resolved, target);
}

#[test]
fn proxy_handoff_allowed_rejects_self_and_cycles() {
    let a = std::path::PathBuf::from("/p/a.md");
    let b = std::path::PathBuf::from("/p/b.md");

    // Empty chain: anything is allowed.
    assert!(proxy_handoff_allowed(&[], &a));
    // First hop recorded; re-proxying to the same doc is a self-cycle.
    assert!(!proxy_handoff_allowed(std::slice::from_ref(&a), &a));
    // A -> B is fine; A -> B -> A closes a cycle.
    assert!(proxy_handoff_allowed(std::slice::from_ref(&a), &b));
    assert!(!proxy_handoff_allowed(&[a.clone(), b.clone()], &a));
}

#[test]
fn request_scoped_proxy_resolution_rejects_lexical_a_b_a_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let prompts = dir.path().join("prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    let a = prompts.join("a.md");
    let b = prompts.join("b.md");
    std::fs::write(&a, "---\n---\n").unwrap();
    std::fs::write(&b, "---\n---\n").unwrap();

    let context = biscuit_file::FileResolutionContext::new(&prompts)
        .with_source_path(&a)
        .with_repository_root(dir.path());
    let resolved_b = resolve_proxy_target_in_context("./b.md", &a, &context).unwrap();
    let resolved_a = resolve_proxy_target_in_context("././a.md", &b, &context).unwrap();
    let authored_a_identity = prompts.join("nested/../a.md");
    let chain = vec![authored_a_identity, resolved_b];

    assert!(!proxy_handoff_allowed(&chain, &resolved_a));
}

#[test]
fn proxy_handoff_allowed_enforces_hop_limit() {
    // A chain at the hop limit rejects any further hand-off, even to a
    // never-seen document.
    let chain: Vec<std::path::PathBuf> = (0..MAX_PROXY_HOPS)
        .map(|i| std::path::PathBuf::from(format!("/p/{i}.md")))
        .collect();
    let fresh = std::path::PathBuf::from("/p/fresh.md");
    assert!(!proxy_handoff_allowed(&chain, &fresh));
    // One below the limit still allows a fresh target.
    assert!(proxy_handoff_allowed(&chain[..MAX_PROXY_HOPS - 1], &fresh));
}

#[test]
fn resolve_proxy_target_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("prompt.md");
    std::fs::write(&source, "---\n---\n").unwrap();

    let err = resolve_proxy_target("nope.md", &source, None).unwrap_err();
    assert!(
        matches!(err, crate::harness::HarnessError::PathResolutionFailed { .. }),
        "unexpected variant: {err:?}"
    );
    assert!(
        err.to_string().contains("does not exist"),
        "unexpected: {err}"
    );
}

/// D4: a bare implicit proxy target is repository-first. With the same basename
/// present at both the repository root and the authoring document's directory,
/// the proxy resolves to the repository-root copy — the exact behavior the
/// private harness grammar could never reach (`./foo` and `foo` were identical).
#[test]
fn resolve_proxy_target_prefers_repository_root_for_implicit() {
    let repo = tempfile::tempdir().unwrap();
    let source = repo.path().join("prompts/run.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, "---\n---\n").unwrap();
    let repo_copy = repo.path().join("plan.md");
    std::fs::write(&repo_copy, "---\n---\n").unwrap();
    let source_copy = repo.path().join("prompts/plan.md");
    std::fs::write(&source_copy, "---\n---\n").unwrap();

    let resolved = resolve_proxy_target("plan.md", &source, Some(repo.path())).unwrap();
    assert_eq!(
        resolved, repo_copy,
        "implicit proxy target must resolve repository-first (D4)"
    );
}

#[test]
fn resolve_proxy_target_package_reference_prefers_authoring_package_area() {
    let repo = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(repo.path()).unwrap();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"claudine/lib\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    let package_area = root.join("claudine");
    let member = package_area.join("lib");
    std::fs::create_dir_all(member.join("src")).unwrap();
    std::fs::create_dir_all(member.join("prompts")).unwrap();
    std::fs::write(
        member.join("Cargo.toml"),
        "[package]\nname = \"fixture-claudine\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(member.join("src/lib.rs"), "").unwrap();
    let source = member.join("prompts/run.md");
    std::fs::write(&source, "---\n---\n").unwrap();
    std::fs::write(root.join("shared.md"), "repository decoy").unwrap();
    let package_target = package_area.join("shared.md");
    std::fs::write(&package_target, "package").unwrap();

    assert_eq!(
        resolve_proxy_target("!shared.md", &source, Some(&root)).unwrap(),
        package_target,
    );
}

/// An explicit `./` proxy target stays pinned to the authoring document even
/// when a repository-root sibling of the same name exists.
#[test]
fn resolve_proxy_target_explicit_stays_source_relative() {
    let repo = tempfile::tempdir().unwrap();
    let source = repo.path().join("prompts/run.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, "---\n---\n").unwrap();
    std::fs::write(repo.path().join("plan.md"), "---\n---\n").unwrap();
    let source_copy = repo.path().join("prompts/plan.md");
    std::fs::write(&source_copy, "---\n---\n").unwrap();

    let resolved = resolve_proxy_target("./plan.md", &source, Some(repo.path())).unwrap();
    assert_eq!(resolved, source_copy);
}

/// D6: a nested proxy anchors on the *proxied* document. Resolving a bare
/// sibling reference against document B finds B's own sibling, never A's —
/// proving provenance follows the `source_path` argument the swap installs.
#[test]
fn resolve_proxy_target_nested_provenance_follows_the_target_document() {
    let repo = tempfile::tempdir().unwrap();
    // Document A lives in `a/`; document B (the proxied target) lives in `b/`.
    let doc_a = repo.path().join("a/run.md");
    std::fs::create_dir_all(doc_a.parent().unwrap()).unwrap();
    std::fs::write(&doc_a, "---\n---\n").unwrap();
    let doc_b = repo.path().join("b/target.md");
    std::fs::create_dir_all(doc_b.parent().unwrap()).unwrap();
    std::fs::write(&doc_b, "---\n---\n").unwrap();
    // `sibling.md` exists only next to B.
    let b_sibling = repo.path().join("b/sibling.md");
    std::fs::write(&b_sibling, "---\n---\n").unwrap();

    // A nested proxy authored *by B* resolves against B's directory.
    let resolved = resolve_proxy_target("./sibling.md", &doc_b, Some(repo.path())).unwrap();
    assert_eq!(resolved, b_sibling);

    // The same reference anchored on A cannot see B's sibling — confirming the
    // source argument, not a leaked original, drives resolution.
    let err = resolve_proxy_target("./sibling.md", &doc_a, Some(repo.path())).unwrap_err();
    assert!(matches!(
        err,
        crate::harness::HarnessError::PathResolutionFailed { .. }
    ));
}

#[test]
#[serial_test::serial]
fn lifecycle_proxy_reuses_request_snapshot_after_environment_mutation() {
    let request = tempfile::tempdir().unwrap();
    let source_dir = request.path().join("source");
    let captured = request.path().join("captured");
    std::fs::create_dir_all(&source_dir).unwrap();
    std::fs::create_dir_all(&captured).unwrap();
    let source = source_dir.join("run.md");
    let target = captured.join("target.md");
    std::fs::write(&source, "---\n---\n").unwrap();
    std::fs::write(&target, "---\n---\n").unwrap();
    let ambient = tempfile::tempdir().unwrap();
    let prior = std::env::var_os("LIFECYCLE_SNAPSHOT_ROOT");
    let mut env = std::collections::HashMap::new();
    env.insert(
        "LIFECYCLE_SNAPSHOT_ROOT".to_string(),
        captured.display().to_string(),
    );
    let snapshot = biscuit_file::FileResolutionContext::new(request.path()).with_env(env);

    // SAFETY: this test is serialized while mutating process-global state.
    unsafe { std::env::set_var("LIFECYCLE_SNAPSHOT_ROOT", ambient.path()) };
    let resolved = resolve_proxy_target_in_context(
        "{{LIFECYCLE_SNAPSHOT_ROOT}}/target.md",
        &source,
        &snapshot,
    );
    match prior {
        Some(value) => unsafe { std::env::set_var("LIFECYCLE_SNAPSHOT_ROOT", value) },
        None => unsafe { std::env::remove_var("LIFECYCLE_SNAPSHOT_ROOT") },
    }

    assert_eq!(resolved.unwrap(), target);
}

/// A `{{VAR}}`-interpolated target with an unset variable surfaces the typed
/// `FileReferenceUnresolvable` diagnostic rather than a missing-target no-match.
#[test]
fn resolve_proxy_target_unset_interpolation_variable_is_typed() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("run.md");
    std::fs::write(&source, "---\n---\n").unwrap();

    let err = resolve_proxy_target(
        "{{DEFINITELY_UNSET_PROXY_VAR}}/x.md",
        &source,
        Some(dir.path()),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            crate::harness::HarnessError::FileReferenceUnresolvable { .. }
        ),
        "unexpected variant: {err:?}"
    );
}
