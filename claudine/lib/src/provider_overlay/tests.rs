//! Planner contracts.
//!
//! Every fixture states its own home and environment through
//! [`HomeBaseline::from_parts`] / [`EnvBaseline::from_entries`], so nothing
//! here reads or mutates the process environment and nothing touches the
//! developer's real provider configuration.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::diagnostics::{Diagnostic, code_spec};
use crate::error::ClaudineError;
use crate::invocation_context::{EnvBaseline, HomeBaseline};
use crate::provider::Provider;

use super::*;

const HOME: &str = "/home/tester";

fn home_baseline() -> HomeBaseline {
    HomeBaseline::from_parts(
        Some(PathBuf::from(HOME)),
        [Some(OsString::from(HOME)), None, None, None],
    )
}

fn env(entries: &[(&str, &str)]) -> EnvBaseline {
    EnvBaseline::from_entries(entries.iter().map(|(k, v)| (*k, *v)))
}

fn plan_for(
    provider: Provider,
    reasons: OverlayReasons,
    entries: &[(&str, &str)],
) -> Result<OverlayPlan, OverlayRefusal> {
    let home = home_baseline();
    let env = env(entries);
    OverlayPlanner::new(&home, &env)
        .with_launch_id("launch")
        .plan(provider, reasons)
}

/// A path under the fixture home, joined one component at a time so the value
/// carries the host's own separator. An `OsString` comparison is byte-exact,
/// unlike a `Path` comparison, so a `/`-spelled expectation passes on Unix and
/// fails on native Windows.
fn path(relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(PathBuf::from(HOME), |path, part| path.join(part))
}

// -- shape -----------------------------------------------------------------

#[test]
fn provider_visible_root_applies_the_selector_shape() {
    let storage = Path::new("/overlay/store");

    assert_eq!(
        provider_visible_root(OverlaySelectorShape::ProviderDir, storage),
        storage,
        "a provider-dir selector names the config directory itself"
    );
    assert_eq!(
        provider_visible_root(
            OverlaySelectorShape::ParentOfProviderDir { child: ".gemini" },
            storage
        ),
        storage.join(".gemini"),
        "a parent-of-provider-dir selector names the parent of the config directory"
    );
    assert_eq!(
        provider_visible_root(OverlaySelectorShape::Inline, storage),
        storage,
        "an inline selector carries content, so there is nothing to offset"
    );
}

/// A launch's storage root is the launch root it was given under both shapes;
/// only the directory the provider reads differs.
#[test]
fn overlay_storage_is_the_launch_root_under_both_shapes() {
    let launch_root = path(".claudine/overlays/codex/launch");

    let codex = OverlayStorage::new(&launch_root, OverlaySelectorShape::ProviderDir)
        .expect("a provider-dir selector has storage");
    assert_eq!(codex.storage_root(), launch_root);
    assert_eq!(codex.provider_visible_root(), launch_root);

    let gemini = OverlayStorage::new(
        &launch_root,
        OverlaySelectorShape::ParentOfProviderDir { child: ".gemini" },
    )
    .expect("a parent-shaped selector has storage");
    assert_eq!(gemini.storage_root(), launch_root);
    assert_eq!(gemini.provider_visible_root(), launch_root.join(".gemini"));
}

#[test]
fn an_inline_selector_has_no_storage() {
    assert!(
        OverlayStorage::new(Path::new("/overlay"), OverlaySelectorShape::Inline)
            .is_none()
    );
}

// -- reason sets -----------------------------------------------------------

#[test]
fn overlay_reasons_behave_as_a_set() {
    let mut reasons = OverlayReasons::none();
    assert!(reasons.is_empty());
    assert_eq!(reasons.len(), 0);

    reasons.insert(OverlayReason::Mcp);
    reasons.insert(OverlayReason::RepoResources);
    reasons.insert(OverlayReason::Mcp);

    assert_eq!(reasons.len(), 2, "a repeated insert is not a second member");
    assert!(reasons.contains(OverlayReason::Mcp));
    assert!(reasons.contains(OverlayReason::RepoResources));
    assert!(!reasons.contains(OverlayReason::RepoPrompt));
    assert_eq!(
        reasons.iter().collect::<Vec<_>>(),
        vec![OverlayReason::RepoResources, OverlayReason::Mcp],
        "iteration is in declaration order regardless of insertion order"
    );
    assert_eq!(
        reasons,
        [OverlayReason::Mcp, OverlayReason::RepoResources]
            .into_iter()
            .collect::<OverlayReasons>()
    );
}

// -- planning --------------------------------------------------------------

#[test]
fn a_codex_repo_plan_points_the_selector_at_the_overlay() {
    let plan = plan_for(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoResources),
        &[],
    )
    .expect("Codex repo isolation is supported");

    let selector = plan.selector().expect("a native-root plan has a selector");
    assert_eq!(selector.env_var(), "CODEX_HOME");
    assert_eq!(selector.value(), path(".claudine/overlays/codex/launch"));
    assert_eq!(
        plan.provider_visible_root(),
        Some(path(".claudine/overlays/codex/launch").as_path())
    );
    assert_eq!(plan.source_root(), Some(path(".codex").as_path()));
    assert!(!plan.source_root_is_explicit());
    assert!(plan.requires_materialization());
    assert_eq!(
        plan.capability(OverlayReason::RepoResources),
        Some(OverlayCapability::NativeRoot)
    );
    assert_eq!(
        plan.capability(OverlayReason::Mcp),
        None,
        "a reason that was not requested carries no verdict"
    );
    assert_eq!(
        plan.env_patch(),
        vec![(
            OsString::from("CODEX_HOME"),
            OsString::from(path(".claudine/overlays/codex/launch"))
        )]
    );
    assert_eq!(
        plan.excluded_resources().into_iter().collect::<Vec<_>>(),
        vec![
            "agents".to_string(),
            "prompts".to_string(),
            "skills".to_string()
        ],
        "`--repo` hides the documented resource set for the provider"
    );
}

/// Invariant 1 at the planning layer, across the whole audit matrix: a plan has
/// no way to name a home variable, so no downstream assembly can be handed one.
#[test]
fn no_plan_patches_a_home_variable() {
    let mut patched = 0usize;
    for provider in crate::provider::PROVIDERS_DISPLAY_ORDER {
        for reason in OVERLAY_REASONS {
            let Ok(plan) = plan_for(provider, OverlayReasons::single(reason), &[]) else {
                continue;
            };
            for (key, value) in plan.env_patch() {
                patched += 1;
                for home_variable in crate::invocation_context::HOME_VARIABLES {
                    assert_ne!(
                        key.to_string_lossy().to_ascii_uppercase(),
                        home_variable,
                        "{provider}/{reason:?} patched a home variable with {value:?}"
                    );
                }
            }
        }
    }
    assert!(
        patched > 0,
        "no plan patched anything — the sweep proved nothing"
    );
}

#[test]
fn a_gemini_plan_offsets_the_selector_value_by_the_child_segment() {
    let plan = plan_for(
        Provider::Gemini,
        OverlayReasons::single(OverlayReason::Mcp),
        &[],
    )
    .expect("Gemini MCP is supported");

    let selector = plan.selector().expect("a native-root plan has a selector");
    assert_eq!(selector.env_var(), "GEMINI_CLI_HOME");
    assert_eq!(
        selector.value(),
        path(".claudine/overlays/gemini/launch"),
        "the selector names the parent of the config directory"
    );
    assert_eq!(
        plan.provider_visible_root(),
        Some(path(".claudine/overlays/gemini/launch/.gemini").as_path()),
        "settings.json is read one level below the selector's value"
    );
    assert_eq!(plan.source_root(), Some(path(".gemini").as_path()));
    assert!(
        plan.excluded_resources().is_empty(),
        "an MCP-only plan hides no repo resources"
    );
}

#[test]
fn a_multi_reason_plan_records_every_requested_verdict() {
    let reasons = OverlayReasons::single(OverlayReason::RepoResources)
        .with(OverlayReason::RepoPrompt)
        .with(OverlayReason::Mcp);
    let plan = plan_for(Provider::Codex, reasons, &[]).expect("Codex supports all three reasons");

    for reason in reasons.iter() {
        assert_eq!(
            plan.capability(reason),
            Some(OverlayCapability::NativeRoot),
            "{reason:?}"
        );
    }
    assert_eq!(plan.reasons(), reasons);
}

#[test]
fn an_empty_reason_set_plans_no_overlay() {
    let plan = plan_for(Provider::Codex, OverlayReasons::none(), &[]).expect("nothing to refuse");
    assert!(!plan.requires_materialization());
    assert!(plan.selector().is_none());
    assert!(plan.env_patch().is_empty());
}

// -- source roots ----------------------------------------------------------

#[test]
fn an_explicit_selector_value_is_the_source_and_never_the_destination() {
    let explicit = "/elsewhere/codex-home";
    let plan = plan_for(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoResources),
        &[("CODEX_HOME", explicit)],
    )
    .expect("an explicit root does not make the reason unsupported");

    assert_eq!(plan.source_root(), Some(Path::new(explicit)));
    assert!(plan.source_root_is_explicit());
    assert_eq!(
        plan.storage_root(),
        Some(path(".claudine/overlays/codex/launch").as_path()),
        "the user's own root is read from, not written to"
    );
    assert_ne!(plan.selector().map(OverlaySelector::value), Some(Path::new(explicit)));
}

#[test]
fn an_explicit_parent_shaped_value_resolves_through_its_child_segment() {
    let plan = plan_for(
        Provider::Gemini,
        OverlayReasons::single(OverlayReason::Mcp),
        &[("GEMINI_CLI_HOME", "/elsewhere/gemini-parent")],
    )
    .expect("Gemini MCP is supported");

    assert_eq!(
        plan.source_root(),
        Some(Path::new("/elsewhere/gemini-parent/.gemini")),
        "the pre-overlay config directory is the child of the explicit value"
    );
    assert!(plan.source_root_is_explicit());
}

/// Representation variant: an empty value is a *present* variable that names
/// nothing, so the documented default still applies.
#[test]
fn an_empty_selector_value_falls_back_to_the_documented_default() {
    let plan = plan_for(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoResources),
        &[("CODEX_HOME", "")],
    )
    .expect("Codex repo isolation is supported");

    assert_eq!(plan.source_root(), Some(path(".codex").as_path()));
    assert!(!plan.source_root_is_explicit());
}

/// The planner reads the supplied baseline, not the ambient process: an
/// unrelated variable of the same shape must not move the source root.
#[test]
fn a_foreign_selector_value_does_not_move_another_providers_source_root() {
    let plan = plan_for(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoResources),
        &[("GEMINI_CLI_HOME", "/elsewhere/gemini")],
    )
    .expect("Codex repo isolation is supported");

    assert_eq!(plan.source_root(), Some(path(".codex").as_path()));
}

#[test]
fn a_host_without_a_resolvable_home_cannot_plan_a_filesystem_overlay() {
    let home = HomeBaseline::from_parts(None, [None, None, None, None]);
    let env = EnvBaseline::default();
    let refusal = OverlayPlanner::new(&home, &env)
        .plan(
            Provider::Codex,
            OverlayReasons::single(OverlayReason::RepoResources),
        )
        .expect_err("there is nowhere to resolve `~/.codex` from");

    assert_eq!(refusal.reason(), OverlayReason::RepoResources);
    assert_eq!(refusal.selector(), Some("CODEX_HOME"));
}

// -- refusals --------------------------------------------------------------

/// The audit's published refusal list, which Phase 6 implements and the L1
/// contract tests assert. All four are reached only through `--repo`.
#[test]
fn every_published_refusal_pair_refuses_before_a_plan_exists() {
    for provider in [
        Provider::Antigravity,
        Provider::OpenCode,
        Provider::Kilo,
        Provider::Goose,
    ] {
        let refusal = plan_for(
            provider,
            OverlayReasons::single(OverlayReason::RepoResources),
            &[],
        )
        .expect_err("{provider} repo isolation is on the published refusal list");

        assert_eq!(refusal.provider(), provider);
        assert_eq!(refusal.reason(), OverlayReason::RepoResources);
        assert!(
            refusal.next_action().contains("--repo"),
            "{provider}: the refusal must name the mode to drop: {}",
            refusal.next_action()
        );
    }
}

/// A refusal is per reason. Dropping the refused one leaves the launch
/// available — for OpenCode that is inline MCP injection, which is exactly the
/// combination `claudine opencode --repo --mcp` produces today.
#[test]
fn a_multi_reason_request_refuses_only_the_unsupported_reason() {
    let both =
        OverlayReasons::single(OverlayReason::RepoResources).with(OverlayReason::Mcp);
    let refusal = plan_for(Provider::OpenCode, both, &[])
        .expect_err("OpenCode cannot isolate repo resources");
    assert_eq!(refusal.reason(), OverlayReason::RepoResources);

    let plan = plan_for(
        Provider::OpenCode,
        OverlayReasons::single(OverlayReason::Mcp),
        &[],
    )
    .expect("dropping the refused reason leaves MCP available");
    assert_eq!(
        plan.capability(OverlayReason::Mcp),
        Some(OverlayCapability::ComposableInjection)
    );
}

/// Design is explicit that `OPENCODE_CONFIG_CONTENT` must not acquire a
/// filesystem overlay for uniformity.
#[test]
fn an_inline_injection_plan_acquires_no_storage_root() {
    for provider in [Provider::OpenCode, Provider::Kilo] {
        let plan = plan_for(provider, OverlayReasons::single(OverlayReason::Mcp), &[])
            .unwrap_or_else(|refusal| panic!("{provider} MCP refused: {refusal:?}"));

        assert!(!plan.requires_materialization(), "{provider}");
        assert_eq!(plan.storage_root(), None, "{provider}");
        assert_eq!(plan.provider_visible_root(), None, "{provider}");
        assert_eq!(plan.source_root(), None, "{provider}");
        assert!(plan.selector().is_none(), "{provider}");
        assert!(plan.env_patch().is_empty(), "{provider}");
    }
}

// -- profile seam ----------------------------------------------------------

#[test]
fn a_materialized_entry_is_excluded_from_the_source_mirror() {
    let mut plan = plan_for(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoPrompt),
        &[],
    )
    .expect("Codex repo prompts are supported");
    assert!(
        plan.excluded_resources().is_empty(),
        "a prompt-only plan hides nothing until something is materialized"
    );

    let source = plan.source_root().expect("a native-root plan has a source");
    let visible = plan
        .provider_visible_root()
        .expect("a native-root plan has a visible root");
    plan.materialize(
        OverlayMaterialization::directory(
            source.join("prompts"),
            visible.join("prompts"),
            OverlayResourceClass::Config,
        )
        .repo_scoped(),
    );

    assert_eq!(
        plan.excluded_resources().into_iter().collect::<Vec<_>>(),
        vec!["prompts".to_string()],
        "a mirror link would shadow the real content the plan asks for"
    );
    let entry = &plan.materializations()[0];
    assert_eq!(entry.kind, OverlayEntryKind::Directory);
    assert!(entry.repo_scoped);
}

/// A nested destination is content *inside* an entry the mirror already
/// handles, so it must not silently hide that entry's whole top level.
#[test]
fn a_nested_materialization_does_not_exclude_its_ancestor() {
    let mut plan = plan_for(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::Mcp),
        &[],
    )
    .expect("Codex MCP is supported");
    let visible = plan.provider_visible_root().expect("a visible root").to_path_buf();
    plan.materialize(OverlayMaterialization::file(
        path(".codex/prompts/repo.md"),
        visible.join("prompts").join("repo.md"),
        OverlayResourceClass::Config,
    ));

    assert!(plan.excluded_resources().is_empty());
}

#[test]
fn pinned_external_state_joins_the_env_patch_after_the_selector() {
    let mut plan = plan_for(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoResources),
        &[],
    )
    .expect("Codex repo isolation is supported");
    plan.pin_external_state("CODEX_SQLITE_HOME", path(".codex"));

    assert_eq!(
        plan.env_patch(),
        vec![
            (
                OsString::from("CODEX_HOME"),
                OsString::from(path(".claudine/overlays/codex/launch"))
            ),
            (
                OsString::from("CODEX_SQLITE_HOME"),
                OsString::from(path(".codex"))
            ),
        ],
        "live state stays at its pre-overlay location through the provider's own selector"
    );
}

// -- diagnostics -----------------------------------------------------------

#[test]
fn a_refusal_becomes_a_typed_pre_spawn_error() {
    let refusal = plan_for(
        Provider::Antigravity,
        OverlayReasons::single(OverlayReason::RepoResources),
        &[],
    )
    .expect_err("Antigravity has no selector at all");
    let error = ClaudineError::from(refusal);

    assert_eq!(error.code(), "provider.overlay_unsupported");
    let spec = code_spec(error.code()).expect("the code is in the locked catalog");
    let detail = error.detail();
    let object = detail.as_object().expect("a catalog-shaped detail object");
    assert_eq!(object.len(), spec.detail.len());
    assert_eq!(detail["provider"], serde_json::json!("Antigravity"));
    assert_eq!(detail["reason"], serde_json::json!("repo_resources"));
    assert_eq!(
        detail["selector"],
        Value::Null,
        "Antigravity exposes no provider-scoped selector"
    );
    assert!(
        detail["next_action"]
            .as_str()
            .is_some_and(|action| action.contains("--repo"))
    );
}

/// Invariant 8: a diagnostic may name a variable and a path, never a
/// credential value or file content. The selector's *value* is what an
/// explicit provider root makes secret-adjacent, and it is never projected.
#[test]
fn an_overlay_diagnostic_names_the_selector_but_never_its_value() {
    let secret_root = "/private/tokens/codex-home";
    let refusal = plan_for(
        Provider::Goose,
        OverlayReasons::single(OverlayReason::RepoResources),
        &[("CODEX_HOME", secret_root), ("OPENAI_API_KEY", "sk-secret")],
    )
    .expect_err("Goose repo isolation is refused");
    let error = ClaudineError::from(refusal);

    assert_eq!(
        error.detail()["selector"],
        serde_json::json!("GOOSE_PATH_ROOT"),
        "the variable name is safe to print"
    );
    let rendered = format!("{}{}", error, error.detail());
    for secret in [secret_root, "sk-secret"] {
        assert!(
            !rendered.contains(secret),
            "an ambient value leaked into a diagnostic: {rendered}"
        );
    }
}

#[test]
fn a_materialization_failure_projects_its_stage_and_publishes_its_cause() {
    let error = ClaudineError::ProviderOverlayFailed {
        provider: Provider::Codex,
        reason: OverlayReason::RepoResources,
        stage: OverlayStage::Materialization,
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
    };

    assert_eq!(error.code(), "provider.overlay_failed");
    assert_eq!(error.detail()["stage"], serde_json::json!("materialization"));
    assert_eq!(error.detail()["reason"], serde_json::json!("repo_resources"));
    let cause = (&error as &(dyn std::error::Error + 'static))
        .source()
        .expect("the io failure is published, not flattened");
    assert_eq!(
        cause
            .downcast_ref::<std::io::Error>()
            .expect("the concrete io::Error survives")
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

// -- the isolation table ---------------------------------------------------

/// The documented per-provider `--repo` exclusion table
/// (`docs/topics/repo-isolation.md`). One copy, read by both the planner and
/// the wrapper's overlay materialization.
#[test]
fn the_repo_isolation_table_matches_the_documented_set() {
    for (offset, expected) in [
        (".claude", &["skills", "commands", "agents", "hooks"][..]),
        (".codex", &["skills", "agents", "prompts"][..]),
        (".gemini", &["skills", "agents"][..]),
        (".goose", &["skills", "agents"][..]),
        (".kimi", &["skills", "agents"][..]),
        (".opencode", &["skills"][..]),
        (".pi", &["skills", "commands", "agents", "hooks"][..]),
        (".qwen", &["skills", "commands"][..]),
        (".kilo", &["skills", "commands", "agents", "hooks"][..]),
    ] {
        assert_eq!(repo_isolated_resources(offset), expected, "{offset}");
    }
}

// -- launch roots ----------------------------------------------------------

/// Review 1, finding 2: every launch owns its overlay root. Two plans for the
/// same provider and reasons never share a directory, and neither lands on
/// the compatible legacy storage `~/.claudine/<agent_offset>`.
#[test]
fn every_plan_gets_its_own_launch_root_outside_legacy_storage() {
    let home = home_baseline();
    let env = env(&[]);
    let planner = OverlayPlanner::new(&home, &env);
    let reasons = OverlayReasons::single(OverlayReason::RepoResources);

    let first = planner.plan(Provider::Codex, reasons).unwrap();
    let second = planner.plan(Provider::Codex, reasons).unwrap();

    let (first, second) = (first.storage_root().unwrap(), second.storage_root().unwrap());
    assert_ne!(first, second, "two launches share one overlay root");
    for root in [first, second] {
        assert_eq!(root.parent(), Some(path(".claudine/overlays/codex").as_path()));
        assert!(!root.starts_with(path(".claudine/.codex")), "{}", root.display());
    }
}

mod lease {
    use std::fs;

    use tempfile::TempDir;

    use super::super::{OverlayLease, sweep_abandoned_overlays};

    #[test]
    fn a_lease_creates_its_root_and_dropping_it_removes_root_and_lock() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("overlays").join("codex").join("launch");

        let lease = OverlayLease::acquire(&root).unwrap();
        assert!(root.is_dir());
        fs::write(root.join("config.toml"), "x").unwrap();
        drop(lease);

        let left: Vec<_> = fs::read_dir(root.parent().unwrap()).unwrap().collect();
        assert!(left.is_empty(), "the root or its lock survived the lease: {left:?}");
    }

    /// Removal unlinks the Unix mirror's links; it never follows them into the
    /// user's provider root.
    #[cfg(unix)]
    #[test]
    fn dropping_a_lease_does_not_follow_links_into_the_source() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source");
        fs::create_dir_all(source.join("skills")).unwrap();
        fs::write(source.join("skills").join("user.md"), "user").unwrap();
        let root = tmp.path().join("overlays").join("codex").join("launch");

        let lease = OverlayLease::acquire(&root).unwrap();
        std::os::unix::fs::symlink(source.join("skills"), root.join("skills")).unwrap();
        drop(lease);

        assert!(!root.exists());
        assert_eq!(fs::read_to_string(source.join("skills").join("user.md")).unwrap(), "user");
    }

    #[test]
    fn a_name_already_in_use_is_refused_and_left_alone() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("codex").join("launch");
        let live = OverlayLease::acquire(&root).unwrap();
        fs::write(root.join("config.toml"), "live").unwrap();

        assert!(OverlayLease::acquire(&root).is_err());
        assert_eq!(fs::read_to_string(root.join("config.toml")).unwrap(), "live");
        drop(live);
    }

    /// A root whose owner ended without dropping its lease (a crash, `_exit`)
    /// is reclaimed; a root a live lease holds is not, even in this process.
    #[test]
    fn a_sweep_removes_only_roots_without_a_live_owner() {
        let tmp = TempDir::new().unwrap();
        let launches = tmp.path().join("overlays");
        let live_root = launches.join("codex").join("live");
        let live = OverlayLease::acquire(&live_root).unwrap();
        let abandoned = launches.join("gemini").join("abandoned");
        fs::create_dir_all(abandoned.join(".gemini")).unwrap();
        fs::write(launches.join("gemini").join("abandoned.lock"), "").unwrap();
        let lockless = launches.join("codex").join("lockless");
        fs::create_dir_all(&lockless).unwrap();

        assert_eq!(sweep_abandoned_overlays(&launches), 2);

        assert!(live_root.is_dir(), "a live launch's root was swept");
        assert!(!abandoned.exists() && !lockless.exists());
        assert!(!launches.join("gemini").join("abandoned.lock").exists());
        drop(live);
        assert_eq!(sweep_abandoned_overlays(&launches), 0);
    }

    #[test]
    fn a_sweep_of_a_missing_directory_removes_nothing() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(sweep_abandoned_overlays(&tmp.path().join("absent")), 0);
    }
}
