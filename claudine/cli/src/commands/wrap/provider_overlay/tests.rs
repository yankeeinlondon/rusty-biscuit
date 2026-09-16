//! Every fixture states its own home and environment baseline, so nothing here
//! reads the developer's real provider configuration or mutates the process
//! environment. Paths are joined one component at a time: an `OsString`
//! comparison is byte-exact on native Windows, unlike a `Path` comparison.

#[cfg(unix)]
use std::process::Command;

use super::*;
use claudine::provider::PROVIDERS_DISPLAY_ORDER;
use claudine::provider_overlay::repo_isolated_resources;
use tempfile::TempDir;

fn join(root: &Path, parts: &[&str]) -> PathBuf {
    parts.iter().fold(root.to_path_buf(), |path, part| path.join(part))
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn home_baseline(home: &Path) -> HomeBaseline {
    HomeBaseline::from_parts(
        Some(home.to_path_buf()),
        [Some(home.as_os_str().to_owned()), None, None, None],
    )
}

fn env_baseline(entries: &[(&str, &Path)]) -> EnvBaseline {
    EnvBaseline::from_entries(
        entries
            .iter()
            .map(|(name, value)| (OsString::from(name), value.as_os_str().to_owned())),
    )
}

/// The launch id every [`planned`] fixture pins, so its root is predictable.
const LAUNCH: &str = "launch";

/// The root a [`planned`] fixture's overlay is built in.
fn launch_root(home: &Path, provider: Provider) -> PathBuf {
    join(home, &[".claudine", "overlays", provider.as_slug(), LAUNCH])
}

/// A plan as the wrapper builds it: planned, then shaped by the profile.
fn planned(
    provider: Provider,
    reasons: OverlayReasons,
    home: &Path,
    env: &EnvBaseline,
) -> Result<OverlayPlan, claudine::provider_overlay::OverlayRefusal> {
    let mut plan = OverlayPlanner::new(&home_baseline(home), env)
        .with_launch_id(LAUNCH)
        .plan(provider, reasons)?;
    profile_for_provider(provider)
        .unwrap()
        .overlay_strategy(&mut plan, env)
        .unwrap();
    Ok(plan)
}

/// Build an overlay through the launch entry point and apply its patch to an
/// empty child environment, as `build_child_env_with_launch` does. The plan is
/// returned too: it owns the overlay root, which is removed once it drops.
fn launch_env(
    provider: Provider,
    reasons: OverlayReasons,
    cwd: &Path,
    effective_root: Option<&Path>,
    home: &Path,
    env: &EnvBaseline,
) -> Result<(HashMap<OsString, OsString>, Option<PathBuf>, OverlayPlan)> {
    let (plan, _) = build_overlay(
        provider,
        reasons,
        cwd,
        false,
        effective_root,
        &home_baseline(home),
        env,
    )?;
    let mut child_env = HashMap::new();
    apply_overlay_env(&mut child_env, &plan);
    let visible = plan.provider_visible_root().map(Path::to_path_buf);
    Ok((child_env, visible, plan))
}

fn assert_no_home_variable(env: &HashMap<OsString, OsString>) {
    for name in HOME_VARIABLES {
        assert!(!env.contains_key(OsStr::new(name)), "overlay patch wrote {name}");
    }
}

fn mirror_modes() -> Vec<MirrorMode> {
    if cfg!(unix) {
        vec![MirrorMode::Link, MirrorMode::Copy]
    } else {
        vec![MirrorMode::Copy]
    }
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

fn is_absent(path: &Path) -> bool {
    fs::symlink_metadata(path).is_err()
}

/// Every entry beneath `root` — relative path, whether it is a link, and file
/// bytes — without following links.
fn snapshot(root: &Path) -> Vec<(PathBuf, bool, Option<Vec<u8>>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, bool, Option<Vec<u8>>)>) {
        let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap()).collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type().unwrap();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if file_type.is_dir() {
                out.push((relative, false, None));
                walk(root, &path, out);
            } else {
                let bytes = file_type.is_file().then(|| fs::read(&path).unwrap());
                out.push((relative, file_type.is_symlink(), bytes));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

fn overlay_error(report: &color_eyre::eyre::Report) -> &ClaudineError {
    report
        .downcast_ref::<ClaudineError>()
        .unwrap_or_else(|| panic!("expected a typed overlay error, got {report:?}"))
}

// -- exclusion policy ---------------------------------------------------------

/// The per-provider `--repo` exclusion table has one owner, in the library, so
/// the overlay planner and this mirror cannot disagree about what `--repo`
/// hides. Both spellings the mirror accepts — bare and dot-prefixed — stay
/// excluded, and a launch without `--repo` hides only what a profile
/// materializes itself.
#[test]
fn repo_only_exclusions_come_from_the_shared_isolation_table() {
    let tmp = TempDir::new().unwrap();
    let env = EnvBaseline::default();
    let mut checked = 0;
    for provider in PROVIDERS_DISPLAY_ORDER {
        let Ok(plan) = planned(
            provider,
            OverlayReasons::single(OverlayReason::RepoResources),
            tmp.path(),
            &env,
        ) else {
            continue;
        };
        checked += 1;
        let expected = repo_isolated_resources(provider.agent_offset());
        assert!(!expected.is_empty(), "{provider} excludes nothing");

        let excluded = plan.excluded_resources();
        let owned: BTreeSet<String> = expected.iter().map(|name| (*name).to_string()).collect();
        let materialized: BTreeSet<String> = excluded.difference(&owned).cloned().collect();
        assert!(
            materialized
                .iter()
                .all(|name| plan.materializations().iter().any(|entry| entry
                    .destination
                    .file_name()
                    .is_some_and(|file| file == name.as_str()))),
            "{provider} excludes names that are neither isolated nor materialized: {materialized:?}"
        );

        for name in expected {
            assert!(
                is_excluded(&excluded, OsStr::new(name)),
                "{provider} does not hide `{name}` under --repo"
            );
            assert!(
                is_excluded(&excluded, OsStr::new(&format!(".{name}"))),
                "{provider} does not hide `.{name}` under --repo"
            );
        }
        assert!(
            !is_excluded(&excluded, OsStr::new("config.toml")),
            "{provider} hides its settings under --repo"
        );
    }
    assert!(checked > 0, "no provider could plan a --repo overlay");

    let codex = planned(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoPrompt),
        tmp.path(),
        &env,
    )
    .unwrap();
    assert_eq!(
        codex.excluded_resources(),
        BTreeSet::from(["prompts".to_string()]),
        "a Codex prompt overlay without --repo hides only the prompts it rebuilds"
    );

    let gemini = planned(
        Provider::Gemini,
        OverlayReasons::single(OverlayReason::Mcp),
        tmp.path(),
        &env,
    )
    .unwrap();
    assert!(!is_excluded(&gemini.excluded_resources(), OsStr::new("skills")));
}

/// Through the filesystem: a Claude `--repo` overlay omits every isolated
/// resource class in both spellings and still carries settings and history.
#[test]
fn a_repo_overlay_omits_isolated_resources_and_keeps_settings() {
    for mode in mirror_modes() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let source = home.join(".claude");
        write(&source.join("settings.json"), "{}");
        write(&join(&source, &["projects", "repo", "session.jsonl"]), "{}");
        write(&join(&source, &["skills", "review", "SKILL.md"]), "skill");
        write(&join(&source, &["commands", "plan.md"]), "command");
        write(&join(&source, &[".agents", "helper.md"]), "agent");
        write(&join(&source, &["hooks", "hook.sh"]), "hook");

        let plan = planned(
            Provider::Claude,
            OverlayReasons::single(OverlayReason::RepoResources),
            &home,
            &EnvBaseline::default(),
        )
        .unwrap();
        let _lease = materialize(&plan, tmp.path(), mode).unwrap();

        let overlay = launch_root(&home, Provider::Claude);
        assert!(overlay.join("settings.json").exists(), "{mode:?}");
        assert!(join(&overlay, &["projects", "repo", "session.jsonl"]).exists(), "{mode:?}");
        for hidden in ["skills", "commands", ".agents", "hooks"] {
            assert!(is_absent(&overlay.join(hidden)), "{mode:?} mirrored `{hidden}`");
        }
    }
}

// -- live state -----------------------------------------------------------------

#[test]
fn volatile_state_files_match_live_dbs_only() {
    // Live DBs + sidecars must be detected (never shared via symlink).
    assert!(is_volatile_state_file(OsStr::new("state_5.sqlite")));
    assert!(is_volatile_state_file(OsStr::new("logs_2.sqlite-wal")));
    assert!(is_volatile_state_file(OsStr::new("memories_1.sqlite-shm")));
    assert!(is_volatile_state_file(OsStr::new("goals_1.sqlite-journal")));

    // Shared config and codex's own repair backups must NOT match.
    assert!(!is_volatile_state_file(OsStr::new("config.toml")));
    assert!(!is_volatile_state_file(OsStr::new("auth.json")));
    assert!(!is_volatile_state_file(OsStr::new(
        "state_5.sqlite.codex-repair-1780436523.0.bak"
    )));
}

/// Databases, their sidecars, lock files, and sockets are live state: neither
/// mirror strategy may link or copy them, at the top level or — for the copy,
/// which walks the tree — at any depth.
#[test]
fn a_mutable_state_file_is_neither_linked_nor_copied() {
    for mode in mirror_modes() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let source = home.join(".codex");
        write(&source.join("config.toml"), "model = \"x\"");
        write(&source.join("state_5.sqlite"), "db");
        write(&source.join("logs_2.sqlite-wal"), "wal");
        write(&source.join("session.lock"), "pid");
        write(&join(&source, &["sessions", "deep", "history.sqlite-shm"]), "shm");
        write(&join(&source, &["sessions", "deep", "run.lock"]), "pid");
        write(&join(&source, &["sessions", "deep", "rollout.jsonl"]), "{}");
        #[cfg(unix)]
        let _socket = std::os::unix::net::UnixListener::bind(source.join("agent.sock")).unwrap();

        let plan = planned(
            Provider::Codex,
            OverlayReasons::single(OverlayReason::RepoPrompt),
            &home,
            &EnvBaseline::default(),
        )
        .unwrap();
        let _lease = materialize(&plan, tmp.path(), mode).unwrap();

        let overlay = launch_root(&home, Provider::Codex);
        assert!(overlay.join("config.toml").exists(), "{mode:?}");
        for live in ["state_5.sqlite", "logs_2.sqlite-wal", "session.lock", "agent.sock"] {
            assert!(is_absent(&overlay.join(live)), "{mode:?} placed `{live}` in the overlay");
        }
        if mode == MirrorMode::Copy {
            let deep = join(&overlay, &["sessions", "deep"]);
            assert!(deep.join("rollout.jsonl").is_file());
            assert!(is_absent(&deep.join("history.sqlite-shm")), "nested sidecar was copied");
            assert!(is_absent(&deep.join("run.lock")), "nested lock file was copied");
        }
    }
}

// -- native Windows copy strategy ---------------------------------------------------

/// The copy strategy walks the whole tree. The Windows mirror it replaces
/// descended one level and hard-linked what it found there, so a directory two
/// levels down either failed the launch or never reached the overlay.
#[test]
fn copy_mode_materializes_nested_directories_to_arbitrary_depth() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let source = home.join(".codex");
    let deep_source = join(&source, &["sessions", "2026", "09", "16", "run", "rollout.jsonl"]);
    write(&source.join("config.toml"), "model = \"x\"");
    write(&deep_source, "original");

    let plan = planned(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoPrompt),
        &home,
        &EnvBaseline::default(),
    )
    .unwrap();
    let _lease = materialize(&plan, tmp.path(), MirrorMode::Copy).unwrap();

    let overlay = launch_root(&home, Provider::Codex);
    let deep_dest = join(&overlay, &["sessions", "2026", "09", "16", "run", "rollout.jsonl"]);
    assert_eq!(fs::read_to_string(&deep_dest).unwrap(), "original");
    for copied in [&deep_dest, &overlay.join("config.toml"), &overlay.join("sessions")] {
        assert!(!is_symlink(copied), "{} is a link, not a copy", copied.display());
    }

    fs::write(&deep_dest, "overlay write").unwrap();
    assert_eq!(fs::read_to_string(&deep_source).unwrap(), "original");
}

// -- per-launch roots (review 1, finding 2) ------------------------------------------

/// Two live launches of one provider — the first without `--repo`, the second
/// with it after the user deleted a source entry — each see exactly their own
/// plan. The second cannot inherit the `skills` the first mirrored or the entry
/// the source no longer has, and ending it leaves the first launch's view whole.
#[test]
fn concurrent_launches_of_one_provider_each_see_only_their_own_plan() {
    for mode in mirror_modes() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let source = home.join(".codex");
        write(&source.join("config.toml"), "model = \"x\"");
        write(&join(&source, &["skills", "review", "SKILL.md"]), "user skill");
        write(&join(&source, &["rules", "default.rules"]), "rule");
        let env = EnvBaseline::default();
        let open = |reasons: OverlayReasons| {
            let mut plan = OverlayPlanner::new(&home_baseline(&home), &env)
                .plan(Provider::Codex, reasons)
                .unwrap();
            profile_for_provider(Provider::Codex)
                .unwrap()
                .overlay_strategy(&mut plan, &env)
                .unwrap();
            let lease = materialize(&plan, tmp.path(), mode).unwrap().expect("a native root");
            (plan, lease)
        };

        let mcp = OverlayReasons::single(OverlayReason::Mcp);
        let (first, _first_lease) = open(mcp);
        let first_root = first.provider_visible_root().unwrap().to_path_buf();
        assert!(first_root.join("skills").exists(), "{mode:?}: fixture check");

        fs::remove_dir_all(source.join("rules")).unwrap();
        let (second, second_lease) = open(mcp.with(OverlayReason::RepoResources));
        let second_root = second.provider_visible_root().unwrap().to_path_buf();

        assert_ne!(first_root, second_root, "{mode:?}: two launches share one root");
        assert!(second_root.join("config.toml").exists(), "{mode:?}");
        assert!(is_absent(&second_root.join("skills")), "{mode:?}: --repo kept an earlier launch's skills");
        assert!(is_absent(&second_root.join("rules")), "{mode:?}: a deleted source entry survived");

        drop(second_lease);
        assert!(is_absent(&second_root), "{mode:?}: the ended launch's root remains");
        assert!(first_root.join("config.toml").exists(), "{mode:?}: ending one launch broke the other");
        assert!(join(&first_root, &["skills", "review", "SKILL.md"]).exists(), "{mode:?}");
    }
}

/// The root lives exactly as long as the plan: a retry's recorded clone keeps
/// it, and dropping the last clone removes it without touching the source.
#[test]
fn dropping_the_last_plan_clone_removes_the_launch_root() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    write(&join(&home, &[".codex", "skills", "user.md"]), "user");

    let (_env, _visible, plan) = launch_env(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::Mcp),
        tmp.path(),
        Some(tmp.path()),
        &home,
        &EnvBaseline::default(),
    )
    .unwrap();
    let root = plan.storage_root().unwrap().to_path_buf();
    let recorded = plan.clone();

    drop(plan);
    assert!(root.join("skills").exists(), "a live clone lost its root");
    drop(recorded);
    assert!(is_absent(&root));
    assert_eq!(fs::read_to_string(join(&home, &[".codex", "skills", "user.md"])).unwrap(), "user");
}

/// A root whose launch ended without dropping its plan (a crash, a second
/// Ctrl+C) is reclaimed by the next launch, whatever provider it belonged to.
#[test]
fn a_launch_reclaims_roots_abandoned_by_ended_launches() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let abandoned = join(&home, &[".claudine", "overlays", "gemini", "ended"]);
    write(&join(&abandoned, &[".gemini", "settings.json"]), "{\"mcpServers\":{}}");
    write(&abandoned.with_file_name("ended.lock"), "");

    let (_env, _visible, _plan) = launch_env(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::Mcp),
        tmp.path(),
        Some(tmp.path()),
        &home,
        &EnvBaseline::default(),
    )
    .unwrap();

    assert!(is_absent(&abandoned));
}

// -- guarded write-back (review 1, finding 2 follow-up) -------------------------------

/// Replace `path` the way a provider refreshing a credential does: write a
/// sibling, then rename it over the entry.
fn atomic_replace(path: &Path, content: &str) {
    let staged = path.with_extension("staged");
    fs::write(&staged, content).unwrap();
    fs::rename(&staged, path).unwrap();
}

/// A Codex `--repo` overlay over a source holding `auth.json`, materialized in
/// `mode`; returns the home, the provider-visible root, and the lease.
fn codex_auth_overlay(tmp: &TempDir, mode: MirrorMode) -> (PathBuf, PathBuf, OverlayLease) {
    let home = tmp.path().join("home");
    let auth = join(&home, &[".codex", "auth.json"]);
    if !auth.exists() {
        write(&auth, "{\"token\":\"old\"}");
    }
    let plan = planned(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoResources),
        &home,
        &EnvBaseline::default(),
    )
    .unwrap();
    let lease = materialize(&plan, tmp.path(), mode).unwrap().expect("a native root");
    let visible = plan.provider_visible_root().unwrap().to_path_buf();
    (home, visible, lease)
}

/// Unix: a provider that refreshes a token by renaming over the mirror's link
/// leaves a real file in the overlay. Releasing the launch carries it back,
/// keeping the source's own permissions.
#[cfg(unix)]
#[test]
fn a_file_renamed_over_its_link_is_written_back_when_the_launch_ends() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    let source = join(tmp.path(), &["home", ".codex", "auth.json"]);
    write(&source, "{\"token\":\"old\"}");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o640)).unwrap();
    let (_home, visible, lease) = codex_auth_overlay(&tmp, MirrorMode::Link);
    assert!(is_symlink(&visible.join("auth.json")), "fixture check: a mirrored link");

    atomic_replace(&visible.join("auth.json"), "{\"token\":\"rotated\"}");
    drop(lease);

    assert_eq!(fs::read_to_string(&source).unwrap(), "{\"token\":\"rotated\"}");
    assert_eq!(fs::metadata(&source).unwrap().permissions().mode() & 0o777, 0o640);
    assert!(is_absent(&visible));
}

/// Copy mode (native Windows): a provider rewriting its private copy in place
/// is carried back to the source when the launch ends.
#[test]
fn a_copy_modified_in_place_is_written_back_when_the_launch_ends() {
    let tmp = TempDir::new().unwrap();
    let (home, visible, lease) = codex_auth_overlay(&tmp, MirrorMode::Copy);
    assert!(!is_symlink(&visible.join("auth.json")), "fixture check: a copy");

    fs::write(visible.join("auth.json"), "{\"token\":\"rotated in place\"}").unwrap();
    drop(lease);

    assert_eq!(
        fs::read_to_string(join(&home, &[".codex", "auth.json"])).unwrap(),
        "{\"token\":\"rotated in place\"}"
    );
}

/// A source someone else wrote during the launch wins: the overlay's change is
/// refused rather than overwriting it, under either mirror strategy.
#[test]
fn a_source_changed_during_the_launch_is_not_overwritten() {
    for mode in mirror_modes() {
        let tmp = TempDir::new().unwrap();
        let (home, visible, lease) = codex_auth_overlay(&tmp, mode);
        let source = join(&home, &[".codex", "auth.json"]);

        atomic_replace(&visible.join("auth.json"), "{\"token\":\"from the launch\"}");
        fs::write(&source, "{\"token\":\"from another session\"}").unwrap();
        drop(lease);

        assert_eq!(
            fs::read_to_string(&source).unwrap(),
            "{\"token\":\"from another session\"}",
            "{mode:?}"
        );
    }
}

/// Nothing but a mirrored top-level source file is ever written back:
/// `--repo`-excluded entries, profile materializations (Gemini's private OAuth
/// sidecar), Claudine's injected MCP configuration, and live state keep their
/// source bytes however the launch changed them.
#[test]
fn excluded_materialized_injected_and_live_files_are_never_written_back() {
    for mode in mirror_modes() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let codex = home.join(".codex");
        for (name, content) in [("config.toml", "model = \"user\""), ("agents", "excluded file"), ("state_5.sqlite", "db")] {
            write(&codex.join(name), content);
        }
        let plan = planned(
            Provider::Codex,
            OverlayReasons::single(OverlayReason::RepoResources).with(OverlayReason::Mcp),
            &home,
            &EnvBaseline::default(),
        )
        .unwrap();
        let lease = materialize(&plan, tmp.path(), mode).unwrap().unwrap();
        let visible = plan.provider_visible_root().unwrap().to_path_buf();
        atomic_replace(&visible.join("config.toml"), "[mcp_servers.injected]");
        claudine::provider_overlay::record_claudine_write(&visible.join("config.toml"));
        write(&visible.join("agents"), "provider wrote an excluded name");
        write(&visible.join("state_5.sqlite"), "overlay db");
        drop(lease);

        for (name, content) in [("config.toml", "model = \"user\""), ("agents", "excluded file"), ("state_5.sqlite", "db")] {
            assert_eq!(fs::read_to_string(codex.join(name)).unwrap(), content, "{mode:?} wrote back `{name}`");
        }

        let gemini = home.join(".gemini");
        write(&gemini.join("mcp-oauth-tokens.json"), "{\"token\":\"user\"}");
        let plan = planned(Provider::Gemini, OverlayReasons::single(OverlayReason::Mcp), &home, &EnvBaseline::default())
            .unwrap();
        let lease = materialize(&plan, tmp.path(), mode).unwrap().unwrap();
        let visible = plan.provider_visible_root().unwrap().to_path_buf();
        fs::write(visible.join("mcp-oauth-tokens.json"), "{\"token\":\"overlay\"}").unwrap();
        drop(lease);
        assert_eq!(
            fs::read_to_string(gemini.join("mcp-oauth-tokens.json")).unwrap(),
            "{\"token\":\"user\"}",
            "{mode:?} wrote back a materialized sidecar"
        );
    }
}

/// An entry the provider created, or removed, inside the overlay does not
/// reach the source: write-back only updates files the source already had.
#[test]
fn entries_the_provider_created_or_removed_are_never_written_back() {
    for mode in mirror_modes() {
        let tmp = TempDir::new().unwrap();
        let (home, visible, lease) = codex_auth_overlay(&tmp, mode);
        write(&visible.join("created.json"), "{}");
        write(&join(&visible, &["sessions", "new.jsonl"]), "{}");
        remove_existing_path(&visible.join("auth.json")).unwrap();
        drop(lease);

        let source = home.join(".codex");
        assert!(is_absent(&source.join("created.json")), "{mode:?}");
        assert!(is_absent(&source.join("sessions")), "{mode:?}");
        assert_eq!(fs::read_to_string(source.join("auth.json")).unwrap(), "{\"token\":\"old\"}", "{mode:?}");
    }
}

// -- source root ------------------------------------------------------------------

/// An explicit `CODEX_HOME` is the source the overlay is built from. The
/// overlay lands under Claudine's storage root, and not one byte beneath the
/// user's directory changes — under either mirror strategy.
#[test]
fn an_explicit_source_root_is_read_from_not_written_to() {
    for mode in mirror_modes() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let custom = tmp.path().join("custom codex");
        let repo = tmp.path().join("repo");
        write(&custom.join("config.toml"), "model = \"x\"");
        write(&join(&custom, &["prompts", "user.md"]), "user");
        write(&join(&custom, &["skills", "review", "SKILL.md"]), "skill");
        write(&custom.join("state_5.sqlite"), "db");
        write(&join(&repo, &[".codex", "prompts", "repo.md"]), "repo");
        fs::create_dir_all(&home).unwrap();
        let before = snapshot(&custom);

        let env = env_baseline(&[("CODEX_HOME", &custom)]);
        let plan = planned(
            Provider::Codex,
            OverlayReasons::single(OverlayReason::RepoPrompt),
            &home,
            &env,
        )
        .unwrap();
        assert!(plan.source_root_is_explicit());
        let _lease = materialize(&plan, &repo, mode).unwrap();

        assert_eq!(snapshot(&custom), before, "{mode:?} wrote into the source root");
        let overlay = launch_root(&home, Provider::Codex);
        assert!(overlay.join("config.toml").exists(), "{mode:?}");
        assert!(join(&overlay, &["prompts", "user.md"]).exists(), "{mode:?}");
        assert!(join(&overlay, &["prompts", "repo.md"]).exists(), "{mode:?}");
    }
}

/// Through the launch entry point: the explicit root still names SQLite state,
/// and the child's `CODEX_HOME` names the overlay built from it.
#[test]
fn an_explicit_codex_home_pins_sqlite_state_to_the_source_root() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let custom = tmp.path().join("custom-codex");
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&custom).unwrap();
    fs::create_dir_all(&home).unwrap();
    write(&join(&repo, &[".codex", "prompts", "repo.md"]), "repo");

    let env = env_baseline(&[("CODEX_HOME", &custom)]);
    let (child_env, visible, plan) = launch_env(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoPrompt),
        &repo,
        Some(&repo),
        &home,
        &env,
    )
    .unwrap();

    let overlay = plan.storage_root().unwrap().to_path_buf();
    assert!(overlay.starts_with(join(&home, &[".claudine", "overlays", "codex"])));
    assert_eq!(visible, Some(overlay.clone()));
    assert_eq!(
        child_env.get(OsStr::new("CODEX_SQLITE_HOME")),
        Some(&custom.as_os_str().to_owned())
    );
    assert_eq!(
        child_env.get(OsStr::new("CODEX_HOME")),
        Some(&overlay.into_os_string()),
        "the explicit root is the source, never the destination"
    );
    assert_no_home_variable(&child_env);
}

/// A root the user named must exist: building an empty overlay in its place
/// would drop their configuration without a word.
#[test]
fn a_missing_explicit_source_root_fails_with_a_typed_error_before_building_storage() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let named = tmp.path().join("named claude");

    let report = launch_env(
        Provider::Claude,
        OverlayReasons::single(OverlayReason::RepoResources),
        tmp.path(),
        Some(tmp.path()),
        &home,
        &env_baseline(&[("CLAUDE_CONFIG_DIR", &named)]),
    )
    .unwrap_err();

    match overlay_error(&report) {
        ClaudineError::ProviderOverlayFailed {
            provider,
            reason,
            stage,
            source,
        } => {
            assert_eq!(*provider, Provider::Claude);
            assert_eq!(*reason, OverlayReason::RepoResources);
            assert_eq!(*stage, OverlayStage::SourceRoot);
            assert_eq!(source.kind(), io::ErrorKind::NotFound);
        }
        other => panic!("expected ProviderOverlayFailed, got {other:?}"),
    }
    assert!(is_absent(&home.join(".claudine")), "storage was created for a failed overlay");
    assert!(is_absent(&named), "the named source root was created");
}

/// Phase 11 regression: a provider that has never run has no default root.
/// `codex --mcp` and `claude --repo` launched with an empty overlay before the
/// overlay rework, and must still do so, without creating the user's root.
#[test]
fn a_missing_default_source_root_builds_an_empty_overlay() {
    for (provider, reason, root, selector) in [
        (Provider::Codex, OverlayReason::Mcp, ".codex", "CODEX_HOME"),
        (Provider::Claude, OverlayReason::RepoResources, ".claude", "CLAUDE_CONFIG_DIR"),
    ] {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        fs::create_dir_all(&home).unwrap();

        let (child_env, visible, plan) = launch_env(
            provider,
            OverlayReasons::single(reason),
            tmp.path(),
            Some(tmp.path()),
            &home,
            &EnvBaseline::default(),
        )
        .unwrap_or_else(|report| panic!("{provider:?} {reason:?} refused a missing default root: {report:?}"));

        let overlay = plan.storage_root().unwrap().to_path_buf();
        assert_eq!(visible.as_deref(), Some(overlay.as_path()), "{provider:?}");
        assert!(overlay.is_dir(), "{provider:?}: the overlay was not created");
        assert_eq!(
            child_env.get(OsStr::new(selector)),
            Some(&overlay.clone().into_os_string()),
            "{provider:?}"
        );
        assert!(is_absent(&home.join(root)), "{provider:?}: the user's root was created");
        assert_no_home_variable(&child_env);
    }
}

/// A source root that is a file is not a missing root to start empty from.
#[test]
fn a_source_root_that_is_a_file_fails_with_a_typed_error() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    write(&home.join(".codex"), "not a directory");

    let report = launch_env(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::Mcp),
        tmp.path(),
        Some(tmp.path()),
        &home,
        &EnvBaseline::default(),
    )
    .unwrap_err();

    match overlay_error(&report) {
        ClaudineError::ProviderOverlayFailed { stage, .. } => assert_eq!(*stage, OverlayStage::SourceRoot),
        other => panic!("expected ProviderOverlayFailed, got {other:?}"),
    }
    assert!(is_absent(&home.join(".claudine")), "storage was created for a failed overlay");
}

#[test]
fn an_unsupported_reason_is_refused_before_anything_is_written() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    write(&join(&home, &[".config", "opencode", "opencode.json"]), "{}");

    let report = launch_env(
        Provider::OpenCode,
        OverlayReasons::single(OverlayReason::RepoResources),
        tmp.path(),
        Some(tmp.path()),
        &home,
        &EnvBaseline::default(),
    )
    .unwrap_err();

    assert!(
        matches!(
            overlay_error(&report),
            ClaudineError::ProviderOverlayUnsupported {
                provider: Provider::OpenCode,
                reason: OverlayReason::RepoResources,
                ..
            }
        ),
        "{report:?}"
    );
    assert!(is_absent(&home.join(".claudine")));
}

// -- materialized entries -------------------------------------------------------------

/// Gemini's OAuth tokens and server enablement are materialized as real files
/// under the provider-visible root.
#[test]
fn gemini_sidecars_are_copied_rather_than_linked() {
    for mode in mirror_modes() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let source = home.join(".gemini");
        let overlay = launch_root(&home, Provider::Gemini).join(".gemini");
        write(&source.join("settings.json"), "{}");
        write(&source.join("mcp-oauth-tokens.json"), "{\"token\":\"user\"}");
        write(&source.join("mcp-server-enablement.json"), "{}");

        let plan = planned(
            Provider::Gemini,
            OverlayReasons::single(OverlayReason::Mcp),
            &home,
            &EnvBaseline::default(),
        )
        .unwrap();
        let _lease = materialize(&plan, tmp.path(), mode).unwrap();

        for sidecar in ["mcp-oauth-tokens.json", "mcp-server-enablement.json"] {
            let dest = overlay.join(sidecar);
            assert!(dest.is_file() && !is_symlink(&dest), "{mode:?} `{sidecar}` is not a real file");
        }
        fs::write(overlay.join("mcp-oauth-tokens.json"), "{\"token\":\"overlay\"}").unwrap();
        assert_eq!(
            fs::read_to_string(source.join("mcp-oauth-tokens.json")).unwrap(),
            "{\"token\":\"user\"}"
        );
        assert_eq!(is_symlink(&overlay.join("settings.json")), mode == MirrorMode::Link);
    }
}

#[test]
fn codex_repo_prompts_source_prefers_codex_dir_then_claude_commands() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    let claude_commands = join(repo_root, &[".claude", "commands"]);
    let codex_prompts = join(repo_root, &[".codex", "prompts"]);

    fs::create_dir_all(&claude_commands).unwrap();
    assert_eq!(
        codex_repo_prompts_source(repo_root).as_deref(),
        Some(claude_commands.as_path())
    );

    fs::create_dir_all(&codex_prompts).unwrap();
    assert_eq!(
        codex_repo_prompts_source(repo_root).as_deref(),
        Some(codex_prompts.as_path())
    );
}

#[cfg(unix)]
#[test]
fn prompt_overlay_merges_user_and_repo_prompts() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path().join("repo");
    let prompts_dir = tmp.path().join("shadow/.codex/prompts");
    let user_prompts = tmp.path().join("home/.codex/prompts");
    let claude_commands = repo_root.join(".claude/commands");
    let repo_review = claude_commands.join("review.md");
    let user_commit = user_prompts.join("commit.md");

    fs::create_dir_all(claude_commands.join("nested")).unwrap();
    fs::create_dir_all(&user_prompts).unwrap();
    fs::write(user_prompts.join("review.md"), "user review").unwrap();
    fs::write(&user_commit, "user commit").unwrap();
    fs::write(&repo_review, "repo review").unwrap();
    fs::write(claude_commands.join("nested/plan.md"), "repo nested").unwrap();

    materialize_prompt_overlay(&user_prompts, &prompts_dir, &repo_root, true, MirrorMode::Link)
        .unwrap();

    assert_eq!(
        fs::read_link(prompts_dir.join("review.md")).unwrap(),
        repo_review
    );
    assert_eq!(
        fs::read_link(prompts_dir.join("commit.md")).unwrap(),
        user_commit
    );
    assert_eq!(
        fs::read_link(prompts_dir.join("nested/plan.md")).unwrap(),
        claude_commands.join("nested/plan.md")
    );
}

#[cfg(unix)]
#[test]
fn prompt_overlay_under_repo_uses_repo_prompts_only() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path().join("repo");
    let prompts_dir = tmp.path().join("shadow/.codex/prompts");
    let user_prompts = tmp.path().join("home/.codex/prompts");
    let claude_commands = repo_root.join(".claude/commands");

    fs::create_dir_all(&user_prompts).unwrap();
    fs::create_dir_all(&claude_commands).unwrap();
    fs::write(user_prompts.join("commit.md"), "user commit").unwrap();
    fs::write(claude_commands.join("review.md"), "repo review").unwrap();

    materialize_prompt_overlay(&user_prompts, &prompts_dir, &repo_root, false, MirrorMode::Link)
        .unwrap();

    assert!(fs::symlink_metadata(prompts_dir.join("commit.md")).is_err());
    assert_eq!(
        fs::read_link(prompts_dir.join("review.md")).unwrap(),
        claude_commands.join("review.md")
    );
}

#[cfg(unix)]
#[test]
fn prompt_overlay_replaces_existing_repo_overlay_when_switching_repos() {
    let tmp = TempDir::new().unwrap();
    let first_repo = tmp.path().join("repo-one");
    let second_repo = tmp.path().join("repo-two");
    let user_prompts = tmp.path().join("home/.codex/prompts");
    let prompts_dir = tmp.path().join("shadow/.codex/prompts");
    let first_review = first_repo.join(".claude/commands/review.md");
    let second_review = second_repo.join(".claude/commands/review.md");

    fs::create_dir_all(first_review.parent().unwrap()).unwrap();
    fs::create_dir_all(second_review.parent().unwrap()).unwrap();
    fs::create_dir_all(&user_prompts).unwrap();
    fs::write(&first_review, "repo one").unwrap();
    fs::write(&second_review, "repo two").unwrap();

    materialize_prompt_overlay(&user_prompts, &prompts_dir, &first_repo, true, MirrorMode::Link)
        .unwrap();
    assert_eq!(
        fs::read_link(prompts_dir.join("review.md")).unwrap(),
        first_review
    );

    materialize_prompt_overlay(&user_prompts, &prompts_dir, &second_repo, true, MirrorMode::Link)
        .unwrap();
    assert_eq!(
        fs::read_link(prompts_dir.join("review.md")).unwrap(),
        second_review
    );
}

/// The copy strategy rebuilds the same merged prompt view as real files.
#[test]
fn prompt_overlay_copy_mode_writes_real_files() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path().join("repo");
    let prompts_dir = join(tmp.path(), &["shadow", ".codex", "prompts"]);
    let user_prompts = join(tmp.path(), &["home", ".codex", "prompts"]);
    write(&user_prompts.join("review.md"), "user review");
    write(&user_prompts.join("commit.md"), "user commit");
    write(&join(&repo_root, &[".claude", "commands", "review.md"]), "repo review");
    write(&join(&repo_root, &[".claude", "commands", "nested", "plan.md"]), "repo nested");

    materialize_prompt_overlay(&user_prompts, &prompts_dir, &repo_root, true, MirrorMode::Copy)
        .unwrap();

    assert_eq!(fs::read_to_string(prompts_dir.join("review.md")).unwrap(), "repo review");
    assert_eq!(fs::read_to_string(prompts_dir.join("commit.md")).unwrap(), "user commit");
    assert_eq!(
        fs::read_to_string(join(&prompts_dir, &["nested", "plan.md"])).unwrap(),
        "repo nested"
    );
    assert!(!is_symlink(&prompts_dir.join("review.md")));
}

/// Claude reads `.claude.json` from `$CLAUDE_CONFIG_DIR` once the selector is
/// set, so a default-rooted overlay carries the user's file inside the
/// provider-visible root.
#[test]
fn a_default_claude_overlay_carries_the_home_root_state_file() {
    for mode in mirror_modes() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let source = home.join(".claude.json");
        write(&join(&home, &[".claude", "settings.json"]), "{}");
        write(&source, "{\"userID\":\"real\"}");
        let plan = planned(
            Provider::Claude,
            OverlayReasons::single(OverlayReason::RepoResources),
            &home,
            &EnvBaseline::default(),
        )
        .unwrap();
        let dest = launch_root(&home, Provider::Claude).join(".claude.json");

        let mut lease = materialize(&plan, tmp.path(), mode).unwrap().unwrap();
        materialize_root_level_state(&plan, mode, lease.write_back(Provider::Claude)).unwrap();

        assert_eq!(fs::read_to_string(&dest).unwrap(), "{\"userID\":\"real\"}", "{mode:?}");
        assert_eq!(is_symlink(&dest), mode == MirrorMode::Link, "{mode:?}");
        assert!(is_absent(&launch_root(&home, Provider::Claude).with_file_name(".claude.json")), "{mode:?}");
    }
}

/// An explicit `CLAUDE_CONFIG_DIR` already holds its own `.claude.json`; the
/// home-root file belongs to a different Claude configuration.
#[test]
fn an_explicit_claude_config_dir_does_not_take_the_home_root_state_file() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let custom = tmp.path().join("custom claude");
    write(&home.join(".claude.json"), "{\"userID\":\"home\"}");
    write(&custom.join(".claude.json"), "{\"userID\":\"custom\"}");
    let env = env_baseline(&[("CLAUDE_CONFIG_DIR", &custom)]);
    let plan = planned(
        Provider::Claude,
        OverlayReasons::single(OverlayReason::RepoResources),
        &home,
        &env,
    )
    .unwrap();

    let mut lease = materialize(&plan, tmp.path(), MirrorMode::native()).unwrap().unwrap();
    materialize_root_level_state(&plan, MirrorMode::native(), lease.write_back(Provider::Claude)).unwrap();

    let dest = launch_root(&home, Provider::Claude).join(".claude.json");
    assert_eq!(fs::read_to_string(dest).unwrap(), "{\"userID\":\"custom\"}");
}

// -- activation reasons -------------------------------------------------------------

#[test]
fn overlay_reasons_detect_codex_prompts_at_the_supplied_repo_root() {
    let tmp = TempDir::new().unwrap();
    let with_prompts = tmp.path().join("with-prompts");
    let without_prompts = tmp.path().join("without-prompts");
    fs::create_dir_all(join(&with_prompts, &[".codex", "prompts"])).unwrap();
    fs::create_dir_all(&without_prompts).unwrap();

    assert_eq!(
        overlay_reasons(Provider::Codex, false, false, &with_prompts),
        OverlayReasons::single(OverlayReason::RepoPrompt),
    );
    assert!(overlay_reasons(Provider::Codex, false, false, &without_prompts).is_empty());

    // Only a provider with a prompt root is affected by prompt detection.
    assert!(overlay_reasons(Provider::Claude, false, false, &with_prompts).is_empty());
}

#[test]
fn overlay_reasons_raise_repo_resources_for_every_provider() {
    let tmp = TempDir::new().unwrap();
    let empty = tmp.path().join("empty");
    fs::create_dir_all(&empty).unwrap();

    let repo = OverlayReasons::single(OverlayReason::RepoResources);
    for provider in PROVIDERS_DISPLAY_ORDER {
        assert_eq!(overlay_reasons(provider, true, false, &empty), repo, "{provider}");
    }
}

/// Audit D1: `--mcp`/`--use` raises `Mcp` only for a provider with an MCP
/// verdict, alongside `--repo` when both are requested. A provider with no
/// runtime injector keeps its export guidance rather than refusing.
#[test]
fn overlay_reasons_raise_mcp_only_for_a_provider_with_an_mcp_verdict() {
    let tmp = TempDir::new().unwrap();
    let empty = tmp.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let mcp = OverlayReasons::single(OverlayReason::Mcp);

    assert_eq!(overlay_reasons(Provider::Gemini, false, true, &empty), mcp);
    assert_eq!(overlay_reasons(Provider::OpenCode, false, true, &empty), mcp);
    assert_eq!(
        overlay_reasons(Provider::Codex, true, true, &empty),
        mcp.with(OverlayReason::RepoResources)
    );
    assert!(overlay_reasons(Provider::Claude, false, true, &empty).is_empty());
    assert_eq!(
        overlay_reasons(Provider::Claude, true, true, &empty),
        OverlayReasons::single(OverlayReason::RepoResources),
        "--repo must not raise an MCP reason a provider would refuse"
    );
}

// -- launch environment ------------------------------------------------------------

/// SQLite state stays at the pre-overlay root, and the legacy overlay under
/// `~/.claudine/.codex` is neither the new overlay nor touched by building it.
#[test]
fn codex_overlay_uses_real_sqlite_directory_and_leaves_legacy_storage_untouched() {
    let tmp = TempDir::new().unwrap();
    let user_home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    let real_codex_home = user_home.join(".codex");
    let legacy_overlay = join(&user_home, &[".claudine", ".codex"]);
    fs::create_dir_all(&real_codex_home).unwrap();
    fs::create_dir_all(&repo).unwrap();
    write(&legacy_overlay.join("state_5.sqlite"), "legacy-shadow-state");
    write(&join(&legacy_overlay, &["skills", "legacy", "SKILL.md"]), "legacy skill");
    let before = snapshot(&legacy_overlay);

    let (env, visible, plan) = launch_env(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::Mcp),
        &repo,
        Some(&repo),
        &user_home,
        &EnvBaseline::default(),
    )
    .unwrap();

    assert_eq!(
        env.get(OsStr::new("CODEX_SQLITE_HOME")),
        Some(&OsString::from(&real_codex_home))
    );
    let overlay = visible.unwrap();
    assert_ne!(overlay, legacy_overlay);
    assert_eq!(env.get(OsStr::new("CODEX_HOME")), Some(&overlay.clone().into_os_string()));
    assert!(is_absent(&overlay.join("skills")), "legacy overlay content reached the launch");
    assert_no_home_variable(&env);
    drop(plan);
    assert_eq!(snapshot(&legacy_overlay), before, "legacy storage changed");
}

#[test]
fn non_codex_overlay_does_not_receive_codex_sqlite_home() {
    let tmp = TempDir::new().unwrap();
    let user_home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    fs::create_dir_all(user_home.join(".claude")).unwrap();
    fs::create_dir_all(&repo).unwrap();

    let (env, visible, _plan) = launch_env(
        Provider::Claude,
        OverlayReasons::single(OverlayReason::RepoResources),
        &repo,
        Some(&repo),
        &user_home,
        &env_baseline(&[("CODEX_SQLITE_HOME", &tmp.path().join("ambient"))]),
    )
    .unwrap();

    assert!(!env.contains_key(OsStr::new("CODEX_SQLITE_HOME")));
    assert_eq!(
        env.get(OsStr::new("CLAUDE_CONFIG_DIR")),
        visible.map(PathBuf::into_os_string).as_ref()
    );
    assert_eq!(
        env.get(OsStr::new("CLAUDE_SECURESTORAGE_CONFIG_DIR")),
        Some(&OsString::new()),
        "Claude's credential store stays at its default entry"
    );
    assert_no_home_variable(&env);
}

/// Gemini's selector names the parent of its config directory, so the root the
/// injector writes to is the `.gemini` child of the launch root.
#[test]
fn gemini_overlay_reports_the_provider_visible_root() {
    let tmp = TempDir::new().unwrap();
    let user_home = tmp.path().join("home");
    fs::create_dir_all(user_home.join(".gemini")).unwrap();

    let (env, visible, plan) = launch_env(
        Provider::Gemini,
        OverlayReasons::single(OverlayReason::Mcp),
        tmp.path(),
        Some(tmp.path()),
        &user_home,
        &EnvBaseline::default(),
    )
    .unwrap();

    let storage = plan.storage_root().unwrap();
    assert!(storage.starts_with(join(&user_home, &[".claudine", "overlays", "gemini"])));
    assert_eq!(visible, Some(storage.join(".gemini")));
    assert_eq!(
        env.get(OsStr::new("GEMINI_CLI_HOME")),
        Some(&storage.as_os_str().to_owned()),
        "the selector names the parent of the provider-visible root"
    );
    assert_no_home_variable(&env);
}

/// `build_overlay` materializes Codex repo prompts from the supplied
/// `effective_root` even when `cwd` points to a different directory (or repo):
/// the caller threads a pre-resolved launch-child root through so the
/// redundant `resolve_repo_root(cwd)` sniff walk is skipped.
#[test]
fn build_overlay_uses_supplied_effective_root_not_cwd() {
    let tmp = TempDir::new().unwrap();
    let fake_home = tmp.path().join("home");
    let launch_repo = tmp.path().join("launch-repo");
    let source_repo = tmp.path().join("source-repo");

    fs::create_dir_all(fake_home.join(".codex")).unwrap();
    write(&join(&launch_repo, &[".claude", "commands", "launch.md"]), "launch");
    write(&join(&source_repo, &[".claude", "commands", "source.md"]), "source");

    let (_env, shadow_path, _plan) = launch_env(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoPrompt),
        &source_repo, // cwd points to source repo (simulates metadata root)
        Some(&launch_repo), // effective_root is launch repo (simulates child_cwd)
        &fake_home,
        &EnvBaseline::default(),
    )
    .unwrap();

    let prompts_dir = shadow_path.expect("overlay path must be returned").join("prompts");
    assert!(
        fs::symlink_metadata(prompts_dir.join("launch.md")).is_ok(),
        "expected launch.md from effective_root in the overlay"
    );
    assert!(
        fs::symlink_metadata(prompts_dir.join("source.md")).is_err(),
        "expected source.md from cwd NOT in the overlay"
    );
}

#[cfg(unix)]
fn init_git_repo(path: &Path) -> bool {
    Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Without a supplied root, `build_overlay` falls back to
/// `resolve_repo_root(cwd)`, which walks up to the *git root* — not the literal
/// `cwd`. The prompt only materializes if resolution ascends to the root.
#[cfg(unix)]
#[test]
fn build_overlay_fallback_resolves_repo_root_from_nested_cwd() {
    let tmp = TempDir::new().unwrap();
    let fake_home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    let nested_cwd = repo.join("crate/src/deep");

    fs::create_dir_all(fake_home.join(".codex")).unwrap();
    fs::create_dir_all(repo.join(".claude/commands")).unwrap();
    fs::create_dir_all(&nested_cwd).unwrap();
    fs::write(repo.join(".claude/commands/review.md"), "review").unwrap();

    if !init_git_repo(&repo) {
        // Skip when git is unavailable: without a detectable repo root the
        // fallback cannot distinguish itself from direct cwd reuse.
        return;
    }

    let (_env, shadow_path, _plan) = launch_env(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::RepoPrompt),
        &nested_cwd,
        None,
        &fake_home,
        &EnvBaseline::default(),
    )
    .unwrap();

    let prompts_dir = shadow_path.expect("overlay path must be returned").join("prompts");
    assert!(
        fs::symlink_metadata(prompts_dir.join("review.md")).is_ok(),
        "expected root-level review.md to materialize via resolve_repo_root from nested cwd"
    );
}

// -- Codex SQLite home ----------------------------------------------------------------

#[test]
fn codex_sqlite_home_defaults_to_pre_overlay_codex_home() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let sqlite_home = crate::commands::wrap::profile::codex_launch_sqlite_home(
        &home_baseline(&home),
        &EnvBaseline::default(),
    )
    .unwrap();
    assert_eq!(sqlite_home, home.join(".codex"));
}

#[test]
fn codex_sqlite_home_respects_codex_home_and_explicit_sqlite_home() {
    let tmp = TempDir::new().unwrap();
    let home = home_baseline(&tmp.path().join("home"));
    let codex_home = tmp.path().join("custom-codex");
    let sqlite_home = tmp.path().join("custom-sqlite");
    let resolve = |env: EnvBaseline| {
        crate::commands::wrap::profile::codex_launch_sqlite_home(&home, &env).unwrap()
    };

    assert_eq!(resolve(env_baseline(&[("CODEX_HOME", &codex_home)])), codex_home);
    assert_eq!(
        resolve(env_baseline(&[
            ("CODEX_HOME", &codex_home),
            ("CODEX_SQLITE_HOME", &sqlite_home),
        ])),
        sqlite_home
    );
}

#[test]
fn codex_sqlite_home_rejects_relative_paths() {
    let tmp = TempDir::new().unwrap();
    let env = env_baseline(&[("CODEX_SQLITE_HOME", Path::new("relative/state"))]);
    assert!(
        crate::commands::wrap::profile::codex_launch_sqlite_home(
            &home_baseline(&tmp.path().join("home")),
            &env,
        )
        .is_err()
    );
}

// -- home identity (Invariant 1) -----------------------------------------------------

/// A profile that pinned a home variable would move the child's home. The
/// patch writer refuses it: loudly in debug builds, silently skipped in release.
#[test]
#[cfg_attr(debug_assertions, should_panic(expected = "home variable"))]
fn an_overlay_patch_never_writes_a_home_variable() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let mut plan = planned(
        Provider::Codex,
        OverlayReasons::single(OverlayReason::Mcp),
        &home,
        &EnvBaseline::default(),
    )
    .unwrap();
    plan.pin_external_state("HOME", home.join(".claudine"));
    let mut env = HashMap::from([(OsString::from("HOME"), home.clone().into_os_string())]);

    apply_overlay_env(&mut env, &plan);

    assert_eq!(env.get(OsStr::new("HOME")), Some(&home.into_os_string()));
    assert!(env.contains_key(OsStr::new("CODEX_HOME")));
}

#[test]
fn home_identity_violation_flags_null_devices_and_overlay_paths() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home with space");
    let overlay = join(&home, &[".claudine", "overlays", "codex", LAUNCH]);
    let violating: [(&str, OsString); 5] = [
        ("HOME", OsString::from("/dev/null")),
        ("USERPROFILE", OsString::from("NUL")),
        ("HOMEPATH", OsString::from("nul")),
        ("HOME", home.join(".claudine").into_os_string()),
        ("USERPROFILE", overlay.clone().into_os_string()),
    ];
    for (name, value) in violating {
        let env = HashMap::from([(OsString::from(name), value.clone())]);
        assert_eq!(
            home_identity_violation(&env),
            Some((OsStr::new(name), value.as_os_str())),
            "{name}={value:?}"
        );
    }

    let preserved = HashMap::from([
        (OsString::from("HOME"), home.clone().into_os_string()),
        (OsString::from("USERPROFILE"), home.into_os_string()),
        (OsString::from("CODEX_HOME"), overlay.into_os_string()),
        (OsString::from("OUTPUT"), OsString::from("/dev/null")),
    ]);
    assert_eq!(home_identity_violation(&preserved), None);
    assert_eq!(home_identity_violation(&HashMap::new()), None, "absent home variables");
}

/// A non-UTF-8 home is preserved byte for byte and is not a violation.
#[cfg(unix)]
#[test]
fn home_identity_violation_accepts_a_non_utf8_home() {
    use std::os::unix::ffi::OsStringExt;
    let home = OsString::from_vec(b"/home/caf\xe9".to_vec());
    let env = HashMap::from([(OsString::from("HOME"), home)]);
    assert_eq!(home_identity_violation(&env), None);
}

// -- provider transitions (Invariant 7) ----------------------------------------------

/// The composition base a rebuild restores from holds every overlay variable
/// at its launch-baseline value: the invocation's own selector and pinned state
/// return to the user's value or disappear, another provider's selector the
/// user set survives, and unrelated variables are untouched.
#[test]
fn restoring_overlay_selectors_returns_every_overlay_variable_to_the_baseline() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let user_codex = join(tmp.path(), &["my codex"]);
    let user_gemini = join(tmp.path(), &["my gemini"]);
    let plan = planned(
        Provider::Claude,
        OverlayReasons::single(OverlayReason::RepoResources),
        &home,
        &EnvBaseline::default(),
    )
    .unwrap();
    let baseline = env_baseline(&[("CODEX_HOME", &user_codex), ("GEMINI_CLI_HOME", &user_gemini)]);

    let mut env: HashMap<OsString, OsString> = HashMap::new();
    apply_overlay_env(&mut env, &plan);
    env.insert("CODEX_HOME".into(), "/stale/.claudine/.codex".into());
    env.insert("GEMINI_CLI_HOME".into(), user_gemini.clone().into_os_string());
    env.insert("PATH".into(), "/usr/bin".into());
    assert!(env.contains_key(OsStr::new("CLAUDE_SECURESTORAGE_CONFIG_DIR")));

    restore_overlay_selectors(&mut env, Some(&plan), &baseline);

    assert_eq!(env.get(OsStr::new("CODEX_HOME")), Some(&user_codex.into_os_string()));
    assert_eq!(env.get(OsStr::new("GEMINI_CLI_HOME")), Some(&user_gemini.into_os_string()));
    for gone in ["CLAUDE_CONFIG_DIR", "CLAUDE_SECURESTORAGE_CONFIG_DIR"] {
        assert!(!env.contains_key(OsStr::new(gone)), "{gone} outlived the transition");
    }
    assert_eq!(env.get(OsStr::new("PATH")), Some(&OsString::from("/usr/bin")));
    assert_no_home_variable(&env);
}

/// An explicit ambient value is restored byte for byte, including one that is
/// not valid UTF-8.
#[cfg(unix)]
#[test]
fn restoring_overlay_selectors_keeps_a_non_utf8_ambient_value() {
    use std::os::unix::ffi::OsStrExt;

    let raw = OsStr::from_bytes(b"/home/u/codex-\xff").to_os_string();
    let baseline = EnvBaseline::from_entries([(OsString::from("CODEX_HOME"), raw.clone())]);
    let mut env = HashMap::from([(OsString::from("CODEX_HOME"), OsString::from("/overlay"))]);

    restore_overlay_selectors(&mut env, None, &baseline);

    assert_eq!(env.get(OsStr::new("CODEX_HOME")), Some(&raw));
}
