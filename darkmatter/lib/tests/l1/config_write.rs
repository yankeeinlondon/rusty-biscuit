//! `darkmatter::config::write`: locked, re-read, validated, atomic
//! configuration transactions, exercised through the public API in temporary
//! directories.

use biscuit_file::canonicalize_simplified;
use darkmatter::config::write::{
    ConfigTransaction, ConfigWriteError, LOCK_SUFFIX, PathProblem, SourceExpectation, TargetChange,
    ValidationRefusal,
};
use darkmatter::yaml_edit::{YamlEdit, YamlEditErrorKind};
use serde_json::Value;
use simplified_schema::PropertyPath;
use std::path::{Path, PathBuf};

fn set(path: &str, value: impl Into<Value>) -> YamlEdit {
    YamlEdit::Set { path: PropertyPath::parse(path).unwrap(), value: value.into() }
}

fn accept(_: &darkmatter::config::write::ValidationCandidate<'_>) -> Result<(), ValidationRefusal> {
    Ok(())
}

/// A temporary directory spelled canonically, so expected paths compare with
/// the writer's resolved ones (`/var` is `/private/var` on macOS).
fn workspace() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let real = canonicalize_simplified(dir.path()).unwrap();
    (dir, real)
}

/// Creates a file symlink, or returns `false` on a Windows host that may not
/// create one (`ERROR_PRIVILEGE_NOT_HELD`), so only the alias setup is skipped.
fn symlink_file(original: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(original, link).unwrap();
        true
    }
    #[cfg(windows)]
    {
        match std::os::windows::fs::symlink_file(original, link) {
            Ok(()) => true,
            Err(error) if error.raw_os_error() == Some(1314) => {
                eprintln!("symlink creation needs privilege on this host; alias setup skipped");
                false
            }
            Err(error) => panic!("creating {}: {error}", link.display()),
        }
    }
}

/// The names in `dir`, sorted.
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn lock_name(target: &str) -> String {
    format!("{target}{LOCK_SUFFIX}")
}

#[test]
fn cooperating_writers_through_two_symlink_aliases_both_land() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "# shared\nfirst:\n  seed: 0\nsecond:\n  seed: 0\n").unwrap();
    let alias_one = root.join("alias-one.yaml");
    let alias_two = root.join("alias-two.yaml");
    let (alias_one, alias_two) = if symlink_file(&target, &alias_one) && symlink_file(&target, &alias_two) {
        (alias_one, alias_two)
    } else {
        (target.clone(), target.clone())
    };
    const ROUNDS: usize = 12;

    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        for (alias, group) in [(&alias_one, "first"), (&alias_two, "second")] {
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                for round in 0..ROUNDS {
                    let mut transaction = ConfigTransaction::new();
                    transaction
                        .add(TargetChange::edit(
                            alias,
                            SourceExpectation::Current,
                            vec![set(&format!("{group}.k{round}"), round as u64)],
                        ))
                        .unwrap();
                    transaction.commit(&accept).unwrap();
                }
            });
        }
    });

    let text = std::fs::read_to_string(&target).unwrap();
    let value: Value = biscuit_file::serde_yaml_ng::from_str(&text).unwrap();
    for group in ["first", "second"] {
        for round in 0..ROUNDS {
            assert_eq!(value[group][format!("k{round}")], round as u64, "{group}.k{round} lost:\n{text}");
        }
    }
    assert!(text.starts_with("# shared\n"), "the comment is kept:\n{text}");
    if alias_one != target {
        assert!(std::fs::symlink_metadata(&alias_one).unwrap().file_type().is_symlink());
        assert!(std::fs::symlink_metadata(&alias_two).unwrap().file_type().is_symlink());
    }
}

#[test]
fn an_alias_and_the_real_path_are_one_target_with_one_lock() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "a: 1\n").unwrap();
    let alias = root.join("alias.yaml");
    if !symlink_file(&target, &alias) {
        return;
    }

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("b", 2)])).unwrap();
    transaction.add(TargetChange::edit(&alias, SourceExpectation::Current, vec![set("c", 3)])).unwrap();
    assert_eq!(transaction.lock_order(), [target.as_path()]);
    let outcome = transaction.commit(&accept).unwrap();

    assert_eq!(outcome.targets.len(), 1);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1\nb: 2\nc: 3\n");
}

#[test]
fn an_alias_named_with_a_conflicting_expectation_is_refused() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "a: 1\n").unwrap();
    let alias = root.join("alias.yaml");
    if !symlink_file(&target, &alias) {
        return;
    }

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![])).unwrap();
    let error = transaction
        .add(TargetChange::edit(&alias, SourceExpectation::Bytes(b"a: 1\n".to_vec()), vec![]))
        .unwrap_err();

    assert!(matches!(error, ConfigWriteError::ConflictingChanges { path } if path == target));
}

#[cfg(feature = "effects-instrumentation")]
#[test]
fn lock_deadline_expiry_returns_the_typed_lock_error() {
    use darkmatter::config::write::test_hooks;
    use std::time::Duration;

    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "a: 1\n").unwrap();
    let held = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(lock_name("config.yaml")))
        .unwrap();
    held.lock().unwrap();
    let _deadline = test_hooks::lock_deadline(Duration::from_millis(60));

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("a", 2)])).unwrap();
    let error = transaction.commit(&accept).unwrap_err();

    match error {
        ConfigWriteError::LockTimeout { lock, waited } => {
            assert_eq!(lock, root.join(lock_name("config.yaml")));
            assert!(waited >= Duration::from_millis(60), "gave up after {waited:?}");
            assert!(waited < Duration::from_secs(2), "the injected deadline applied: {waited:?}");
        }
        other => panic!("expected a lock timeout, got {other:?}"),
    }
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1\n");
}

#[test]
fn multi_target_locks_are_taken_in_portable_path_order() {
    let (_dir, root) = workspace();
    let names = ["zeta.yaml", "alpha.yaml", "mid.yaml"];
    for name in names {
        std::fs::write(root.join(name), "n: 0\n").unwrap();
    }
    let expected = [root.join("alpha.yaml"), root.join("mid.yaml"), root.join("zeta.yaml")];

    for order in [names, [names[2], names[0], names[1]]] {
        let mut transaction = ConfigTransaction::new();
        for name in order {
            transaction.add(TargetChange::edit(root.join(name), SourceExpectation::Current, vec![set("n", 1)])).unwrap();
        }
        assert_eq!(transaction.lock_order(), expected.iter().map(PathBuf::as_path).collect::<Vec<_>>());
        let outcome = transaction.commit(&accept).unwrap();
        let saved: Vec<_> = outcome.targets.iter().map(|target| target.path.clone()).collect();
        assert_eq!(saved, expected);
    }
    for name in names {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), "n: 1\n");
    }
}

#[test]
fn changed_input_aborts_the_save_and_leaves_the_file_intact() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "a: 1\n").unwrap();
    let read_by_caller = std::fs::read(&target).unwrap();
    std::fs::write(&target, "a: 1\n# edited by hand\n").unwrap();

    for expected in [
        SourceExpectation::Bytes(read_by_caller.clone()),
        SourceExpectation::Blake3(darkmatter::config::migration_receipt::content_hash(&read_by_caller)),
    ] {
        let mut transaction = ConfigTransaction::new();
        transaction.add(TargetChange::edit(&target, expected, vec![set("a", 2)])).unwrap();
        let error = transaction.commit(&accept).unwrap_err();
        assert!(matches!(&error, ConfigWriteError::Changed { path } if *path == target), "{error:?}");
    }
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1\n# edited by hand\n");
}

#[test]
fn matching_expected_bytes_save() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "a: 1\n").unwrap();

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Bytes(b"a: 1\n".to_vec()), vec![set("a", 2)])).unwrap();
    let outcome = transaction.commit(&accept).unwrap();

    assert!(outcome.targets[0].written);
    assert!(!outcome.targets[0].created);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 2\n");
}

#[test]
fn an_unsupported_alias_edit_is_located_and_leaves_the_file_intact() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    let source = "base: &shared\n  x: 1\nother: *shared\n";
    std::fs::write(&target, source).unwrap();

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("other.x", 2)])).unwrap();
    let error = transaction.commit(&accept).unwrap_err();

    match error {
        ConfigWriteError::UnsupportedEdit { path, source } => {
            assert_eq!(path, target);
            assert_eq!(source.kind, YamlEditErrorKind::Alias);
            assert_eq!(source.location.line, 3, "{source}");
        }
        other => panic!("expected an unsupported edit, got {other:?}"),
    }
    assert_eq!(std::fs::read_to_string(&target).unwrap(), source);
    assert_eq!(entries(&root), ["config.yaml", &lock_name("config.yaml")]);
}

#[test]
fn a_validation_refusal_leaves_the_file_intact() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "network: deny\n").unwrap();
    let seen = std::sync::Mutex::new(Vec::new());
    let validate = |candidate: &darkmatter::config::write::ValidationCandidate<'_>| {
        seen.lock().unwrap().push((candidate.original.map(str::to_string), candidate.candidate.to_string()));
        Err(ValidationRefusal::at(PropertyPath::parse("network").unwrap(), "must be allow or deny"))
    };

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("network", "maybe")])).unwrap();
    let error = transaction.commit(&validate).unwrap_err();

    match error {
        ConfigWriteError::ValidationFailed { path, refusal } => {
            assert_eq!(path, target);
            assert_eq!(refusal.path.unwrap().to_string(), "network");
        }
        other => panic!("expected a validation failure, got {other:?}"),
    }
    assert_eq!(
        seen.into_inner().unwrap(),
        [(Some("network: deny\n".to_string()), "network: maybe\n".to_string())]
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "network: deny\n");
}

#[test]
fn a_validation_refusal_of_any_target_saves_none() {
    let (_dir, root) = workspace();
    for name in ["one.yaml", "two.yaml"] {
        std::fs::write(root.join(name), "n: 0\n").unwrap();
    }
    let validate = |candidate: &darkmatter::config::write::ValidationCandidate<'_>| {
        if candidate.target.ends_with("two.yaml") { Err(ValidationRefusal::new("no")) } else { Ok(()) }
    };

    let mut transaction = ConfigTransaction::new();
    for name in ["one.yaml", "two.yaml"] {
        transaction.add(TargetChange::edit(root.join(name), SourceExpectation::Current, vec![set("n", 1)])).unwrap();
    }
    assert!(matches!(transaction.commit(&validate), Err(ConfigWriteError::ValidationFailed { .. })));

    for name in ["one.yaml", "two.yaml"] {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), "n: 0\n");
    }
}

#[cfg(feature = "effects-instrumentation")]
#[test]
fn an_injected_replacement_failure_leaves_the_file_intact_and_no_temporary_file() {
    use darkmatter::config::write::test_hooks;

    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "a: 1\n").unwrap();
    let _failure = test_hooks::fail_next_replacement();

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("a", 2)])).unwrap();
    let error = transaction.commit(&accept).unwrap_err();

    match error {
        ConfigWriteError::ReplacementFailed { path, published, .. } => {
            assert_eq!(path, target);
            assert!(published.is_empty());
        }
        other => panic!("expected a replacement failure, got {other:?}"),
    }
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1\n");
    assert_eq!(entries(&root), ["config.yaml", &lock_name("config.yaml")]);
}

#[test]
fn a_successful_save_leaves_no_temporary_file_and_retains_the_lock_file() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "a: 1\n").unwrap();

    for value in [2, 3] {
        let mut transaction = ConfigTransaction::new();
        transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("a", value)])).unwrap();
        transaction.commit(&accept).unwrap();
        assert_eq!(entries(&root), ["config.yaml", &lock_name("config.yaml")]);
    }
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 3\n");
}

#[cfg(unix)]
#[test]
fn the_source_permission_bits_are_kept() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, root) = workspace();
    for mode in [0o640, 0o444] {
        let target = root.join(format!("config-{mode:o}.yaml"));
        std::fs::write(&target, "a: 1\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode)).unwrap();

        let mut transaction = ConfigTransaction::new();
        transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("a", 2)])).unwrap();
        transaction.commit(&accept).unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 2\n");
        assert_eq!(std::fs::metadata(&target).unwrap().permissions().mode() & 0o777, mode);
    }
}

#[cfg(windows)]
#[test]
fn a_read_only_file_is_refused_and_keeps_its_attribute() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "a: 1\n").unwrap();
    let mut permissions = std::fs::metadata(&target).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&target, permissions.clone()).unwrap();

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("a", 2)])).unwrap();
    let error = transaction.commit(&accept).unwrap_err();

    assert!(matches!(error, ConfigWriteError::ReplacementFailed { .. }), "{error:?}");
    assert!(std::fs::metadata(&target).unwrap().permissions().readonly());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1\n");
    assert_eq!(entries(&root), ["config.yaml", &lock_name("config.yaml")]);
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&target, permissions).unwrap();
}

#[test]
fn a_missing_target_is_created_in_an_existing_directory() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::create(&target, "# created\n", vec![set("network", "deny")])).unwrap();
    let outcome = transaction.commit(&accept).unwrap();

    assert!(outcome.targets[0].created);
    assert_eq!(outcome.targets[0].path, target);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "# created\nnetwork: deny\n");

    let mut again = ConfigTransaction::new();
    again.add(TargetChange::create(&target, "# other\n", vec![])).unwrap();
    assert!(matches!(again.commit(&accept), Err(ConfigWriteError::Changed { .. })));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "# created\nnetwork: deny\n");
}

#[test]
fn a_missing_target_under_a_missing_directory_is_refused_without_creating_it() {
    let (_dir, root) = workspace();
    let target = root.join("absent").join("config.yaml");

    let error = ConfigTransaction::new().add(TargetChange::create(&target, "", vec![])).map(|_| ()).unwrap_err();

    assert!(matches!(error, ConfigWriteError::PathResolution { problem: PathProblem::MissingParent, .. }), "{error:?}");
    assert!(!root.join("absent").exists());
}

#[test]
fn a_relative_target_is_refused() {
    let error = ConfigTransaction::new()
        .add(TargetChange::edit("config.yaml", SourceExpectation::Current, vec![]))
        .map(|_| ())
        .unwrap_err();

    assert!(matches!(error, ConfigWriteError::PathResolution { problem: PathProblem::NotAbsolute, .. }));
}

#[test]
fn editing_a_missing_file_is_an_io_error() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("a", 1)])).unwrap();
    let error = transaction.commit(&accept).unwrap_err();

    assert!(matches!(&error, ConfigWriteError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound), "{error:?}");
    assert!(!target.exists());
}

#[test]
fn a_symlinked_target_is_edited_in_place_and_stays_a_link() {
    let (_dir, root) = workspace();
    std::fs::create_dir(root.join("real")).unwrap();
    let real = root.join("real").join("config.yaml");
    std::fs::write(&real, "a: 1\n").unwrap();
    let link = root.join("config.yaml");
    if !symlink_file(&real, &link) {
        return;
    }

    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&link, SourceExpectation::Current, vec![set("a", 2)])).unwrap();
    let outcome = transaction.commit(&accept).unwrap();

    assert_eq!(outcome.targets[0].path, real);
    assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "a: 2\n");
    assert!(root.join("real").join(lock_name("config.yaml")).exists(), "the lock is the real file's sibling");
    assert!(!root.join(lock_name("config.yaml")).exists());
}

#[test]
fn a_dangling_symlink_target_is_refused() {
    let (_dir, root) = workspace();
    let link = root.join("config.yaml");
    if !symlink_file(&root.join("missing.yaml"), &link) {
        return;
    }

    let error = ConfigTransaction::new().add(TargetChange::create(&link, "", vec![])).map(|_| ()).unwrap_err();

    assert!(matches!(error, ConfigWriteError::PathResolution { problem: PathProblem::DanglingSymlink, .. }), "{error:?}");
}

// ── TargetChange::edit_refusal ──────────────────────────────────────────────

/// The dry run applies the change's edits to the bytes it expects, with the
/// staging editor, and agrees with what a commit then does.
#[test]
fn edit_refusal_predicts_the_commit_for_the_expected_bytes() {
    let (_dir, root) = workspace();
    let cases: [(&str, &str, Option<YamlEditErrorKind>); 5] = [
        ("inline root", "{a: 1}\n", Some(YamlEditErrorKind::FlowCollection)),
        ("inline list walked into", "a: 1\nlist: [x]\n", Some(YamlEditErrorKind::FlowCollection)),
        ("block root", "a: 1\n", None),
        ("empty inline root", "{}\n", None),
        ("inline value replaced whole", "a: {x: 1}\n", None),
    ];
    for (case, text, expected) in cases {
        let path = root.join(format!("{}.yaml", case.replace(' ', "-")));
        std::fs::write(&path, text).unwrap();
        let edit = if case == "inline list walked into" { set("list.1", "y") } else { set("a", 2) };
        let change = TargetChange::edit(&path, SourceExpectation::Bytes(text.as_bytes().to_vec()), vec![edit]);

        assert_eq!(change.edit_refusal().map(|refusal| refusal.kind), expected, "{case}");
        let mut transaction = ConfigTransaction::new();
        transaction.add(change).unwrap();
        let committed = transaction.commit(&accept);
        match expected {
            Some(_) => {
                assert!(matches!(committed, Err(ConfigWriteError::UnsupportedEdit { .. })), "{case}: {committed:?}");
                assert_eq!(std::fs::read_to_string(&path).unwrap(), text, "{case}");
            }
            None => assert!(committed.is_ok(), "{case}: {committed:?}"),
        }
    }
}

#[test]
fn edit_refusal_cannot_judge_bytes_it_does_not_know() {
    let (_dir, root) = workspace();
    let path = root.join("current.yaml");
    std::fs::write(&path, "{a: 1}\n").unwrap();
    let change = TargetChange::edit(&path, SourceExpectation::Current, vec![set("a", 2)]);
    assert!(change.edit_refusal().is_none());
}
