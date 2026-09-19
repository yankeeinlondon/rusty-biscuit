//! The provider-overlay profile seam.
//!
//! Every fixture states its own home and environment, so nothing here reads the
//! developer's real provider configuration or touches the process environment.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use claudine::invocation_context::{EnvBaseline, HomeBaseline};
use claudine::provider_overlay::{
    OverlayEntryKind, OverlayPlan, OverlayPlanner, OverlayReason, OverlayReasons,
    OverlayResourceClass,
};

use super::super::*;

fn profile(provider: Provider) -> &'static dyn WrapperProfile {
    profile_for_provider(provider).unwrap()
}

/// An absolute path in the host's own spelling.
///
/// Native Windows does not consider a rootless `/x` path absolute, and the
/// Codex state root is required to be — so a fixture that hard-codes a POSIX
/// root passes on Unix and fails on Windows for a reason that has nothing to
/// do with the behavior under test.
fn absolute(relative: &str) -> PathBuf {
    #[cfg(windows)]
    let root = PathBuf::from("C:\\");
    #[cfg(not(windows))]
    let root = PathBuf::from("/");
    join_components(root, relative)
}

fn home() -> PathBuf {
    absolute("home/tester")
}

/// A path under the fixture home. Joined one component at a time because an
/// `OsString` comparison is byte-exact, unlike a `Path` comparison — a
/// `/`-spelled expectation passes on Unix and fails on native Windows.
fn under_home(relative: &str) -> PathBuf {
    join_components(home(), relative)
}

fn join_components(root: PathBuf, relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(root, |path, part| path.join(part))
}

fn plan(provider: Provider, reasons: OverlayReasons, env: &EnvBaseline) -> OverlayPlan {
    let home = HomeBaseline::from_parts(Some(home()), [Some(OsString::from(home())), None, None, None]);
    OverlayPlanner::new(&home, env)
        .with_launch_id("launch")
        .plan(provider, reasons)
        .unwrap_or_else(|refusal| panic!("{provider} plan refused: {refusal:?}"))
}

fn env(entries: &[(&str, &Path)]) -> EnvBaseline {
    EnvBaseline::from_entries(entries.iter().map(|(k, v)| (OsString::from(k), OsString::from(v))))
}

#[test]
fn codex_materializes_its_prompts_and_pins_sqlite_state_outside_the_overlay() {
    let baseline = env(&[]);
    let mut plan = plan(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoPrompt),
        &baseline,
    );

    profile(Provider::Codex)
        .overlay_strategy(&mut plan, &baseline)
        .expect("the default Codex layout plans cleanly");

    let prompts = plan
        .materializations()
        .iter()
        .find(|entry| entry.destination.ends_with("prompts"))
        .expect("the prompt directory is materialized");
    assert_eq!(prompts.source, under_home(".codex/prompts"));
    assert_eq!(prompts.destination, under_home(".claudine/overlays/codex/launch/prompts"));
    assert_eq!(prompts.kind, OverlayEntryKind::Directory);
    assert!(
        prompts.repo_scoped,
        "the repository's own prompts merge over the user's"
    );
    assert!(
        plan.excluded_resources().contains("prompts"),
        "a mirror link would shadow the materialized directory"
    );

    assert_eq!(
        plan.env_patch(),
        vec![
            (
                OsString::from("CODEX_HOME"),
                OsString::from(under_home(".claudine/overlays/codex/launch"))
            ),
            (
                OsString::from("CODEX_SQLITE_HOME"),
                OsString::from(home().join(".codex"))
            ),
        ],
        "live SQLite state stays at its pre-overlay location"
    );
}

/// Audit D4: the state selector resolves from the launch baseline. An explicit
/// `CODEX_SQLITE_HOME` outranks the source root, and neither is ever the
/// overlay.
#[test]
fn codex_prefers_an_explicit_sqlite_home_over_the_source_root() {
    let state_root = absolute("var/codex-state");
    let codex_root = absolute("elsewhere/codex");
    let empty = PathBuf::new();
    for (entries, expected) in [
        (
            vec![("CODEX_SQLITE_HOME", state_root.as_path())],
            state_root.clone(),
        ),
        (
            vec![("CODEX_HOME", codex_root.as_path())],
            codex_root.clone(),
        ),
        (
            vec![
                ("CODEX_HOME", codex_root.as_path()),
                ("CODEX_SQLITE_HOME", state_root.as_path()),
            ],
            state_root.clone(),
        ),
        // A present-but-empty value names nothing, so the source root applies.
        (
            vec![("CODEX_SQLITE_HOME", empty.as_path())],
            home().join(".codex"),
        ),
    ] {
        let baseline = env(&entries);
        let mut plan = plan(
            Provider::Codex,
            OverlayReasons::single(OverlayReason::RepoResources),
            &baseline,
        );
        profile(Provider::Codex)
            .overlay_strategy(&mut plan, &baseline)
            .unwrap_or_else(|error| panic!("{entries:?} failed to plan: {error}"));

        let sqlite = plan
            .env_patch()
            .into_iter()
            .find(|(key, _)| key == "CODEX_SQLITE_HOME")
            .map(|(_, value)| PathBuf::from(value))
            .unwrap_or_else(|| panic!("{entries:?} pinned no SQLite home"));
        assert_eq!(sqlite, expected, "{entries:?}");
        assert_ne!(
            sqlite,
            under_home(".claudine/overlays/codex/launch"),
            "{entries:?} put live state inside the overlay"
        );
    }
}

#[test]
fn codex_refuses_a_relative_sqlite_home() {
    let baseline = env(&[("CODEX_SQLITE_HOME", Path::new("relative/state"))]);
    let mut plan = plan(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoResources),
        &baseline,
    );

    let error = profile(Provider::Codex)
        .overlay_strategy(&mut plan, &baseline)
        .expect_err("a relative state root cannot be handed to Codex");
    assert!(error.to_string().contains("absolute"), "{error}");
}

/// The sidecars must land under the `.gemini` child of `GEMINI_CLI_HOME`, not
/// beside it — the shape distinction the audit exists to fix.
#[test]
fn gemini_materializes_its_oauth_sidecars_under_the_provider_visible_root() {
    let baseline = env(&[]);
    let mut plan = plan(
        Provider::Gemini,
        OverlayReasons::single(OverlayReason::Mcp),
        &baseline,
    );

    profile(Provider::Gemini)
        .overlay_strategy(&mut plan, &baseline)
        .expect("the default Gemini layout plans cleanly");

    let entries = plan.materializations();
    assert_eq!(entries.len(), 2);
    for (entry, class) in entries.iter().zip([
        OverlayResourceClass::Config,
        OverlayResourceClass::Auth,
    ]) {
        let name = entry.destination.file_name().expect("a file name");
        assert_eq!(entry.source, home().join(".gemini").join(name));
        assert_eq!(
            entry.destination,
            under_home(".claudine/overlays/gemini/launch/.gemini").join(name),
            "the sidecar must sit beside settings.json, not beside the selector value"
        );
        assert_eq!(entry.kind, OverlayEntryKind::File);
        assert_eq!(entry.class, class);
    }
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.destination.file_name().unwrap().to_string_lossy())
            .collect::<Vec<_>>(),
        vec!["mcp-server-enablement.json", "mcp-oauth-tokens.json"]
    );
}

#[test]
fn gemini_reads_an_explicit_provider_root_as_the_materialization_source() {
    let explicit_parent = absolute("elsewhere/parent");
    let baseline = env(&[("GEMINI_CLI_HOME", explicit_parent.as_path())]);
    let mut plan = plan(
        Provider::Gemini,
        OverlayReasons::single(OverlayReason::Mcp),
        &baseline,
    );

    profile(Provider::Gemini)
        .overlay_strategy(&mut plan, &baseline)
        .expect("an explicit root plans cleanly");

    for entry in plan.materializations() {
        assert!(
            entry.source.starts_with(explicit_parent.join(".gemini")),
            "the user's own root is the source: {}",
            entry.source.display()
        );
        assert!(
            entry.destination.starts_with(under_home(".claudine")),
            "and never the destination: {}",
            entry.destination.display()
        );
    }
}

/// `CLAUDE_CONFIG_DIR` renames Claude's credential-store entry and moves its
/// credentials file, so the overlay pins `CLAUDE_SECURESTORAGE_CONFIG_DIR` to the
/// pre-overlay store: empty for the default entry, the user's own directory when
/// they chose one, and an explicit secure-storage value above both.
#[test]
fn claude_keeps_its_credential_store_at_the_pre_overlay_location() {
    let custom = under_home("custom claude");
    let secure = under_home("secure");
    let cases: [(Vec<(&str, &Path)>, OsString); 4] = [
        (vec![], OsString::new()),
        (vec![("CLAUDE_CONFIG_DIR", Path::new(""))], OsString::new()),
        (vec![("CLAUDE_CONFIG_DIR", &custom)], custom.clone().into_os_string()),
        (
            vec![("CLAUDE_CONFIG_DIR", &custom), ("CLAUDE_SECURESTORAGE_CONFIG_DIR", &secure)],
            secure.clone().into_os_string(),
        ),
    ];
    for (entries, expected) in cases {
        let baseline = env(&entries);
        let mut plan = plan(
            Provider::Claude,
            OverlayReasons::single(OverlayReason::RepoResources),
            &baseline,
        );
        let visible = plan.provider_visible_root().unwrap().to_path_buf();

        profile(Provider::Claude)
            .overlay_strategy(&mut plan, &baseline)
            .unwrap();

        assert!(plan.materializations().is_empty(), "{entries:?}");
        assert_eq!(
            plan.env_patch(),
            vec![
                (OsString::from("CLAUDE_CONFIG_DIR"), visible.into_os_string()),
                (OsString::from("CLAUDE_SECURESTORAGE_CONFIG_DIR"), expected),
            ],
            "{entries:?}"
        );
    }
}

/// Plan → Phase 4 leaves the other seven profiles on the default. A profile
/// that silently gained a strategy would materialize content no audit row
/// covers.
#[test]
fn every_other_profile_keeps_the_no_op_overlay_strategy() {
    let baseline = env(&[]);
    for provider in [
        Provider::KimiCode,
        Provider::Pi,
        Provider::QwenCode,
    ] {
        let mut plan = plan(
            provider,
            OverlayReasons::single(OverlayReason::RepoResources),
            &baseline,
        );
        let before = plan.env_patch();

        profile(provider)
            .overlay_strategy(&mut plan, &baseline)
            .unwrap_or_else(|error| panic!("{provider}: {error}"));

        assert!(plan.materializations().is_empty(), "{provider}");
        assert_eq!(plan.env_patch(), before, "{provider}");
    }
}

/// An inline-injection plan has no storage at all, so a strategy has nothing to
/// attach to and must not invent a root.
#[test]
fn an_inline_plan_gains_no_materialization() {
    let baseline = env(&[]);
    let mut plan = plan(
        Provider::OpenCode,
        OverlayReasons::single(OverlayReason::Mcp),
        &baseline,
    );

    profile(Provider::OpenCode)
        .overlay_strategy(&mut plan, &baseline)
        .expect("an inline plan needs no side effects");

    assert!(plan.materializations().is_empty());
    assert!(plan.env_patch().is_empty());
    assert_eq!(plan.provider_visible_root(), None::<&Path>);
}
