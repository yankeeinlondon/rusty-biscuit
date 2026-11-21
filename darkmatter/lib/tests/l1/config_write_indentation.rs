//! `ConfigTransaction` lays out what it writes for the resolved target: a
//! generated document and its edits take the destination's preference, an
//! edit copies the file's own layout first, and the preference is resolved
//! at most once per target and never for a save that changes nothing.

use biscuit_file::{Indentation, canonicalize_simplified};
use darkmatter::config::write::{ConfigTransaction, ConfigWriteError, TargetChange, ValidationCandidate, ValidationRefusal};
use darkmatter::config::write::SourceExpectation;
use darkmatter::yaml_edit::YamlEdit;
use serde_json::{Value, json};
use simplified_schema::PropertyPath;
use std::num::NonZeroU8;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn set(path: &str, value: Value) -> YamlEdit {
    YamlEdit::Set { path: PropertyPath::parse(path).unwrap(), value }
}

fn accept(_: &ValidationCandidate<'_>) -> Result<(), ValidationRefusal> {
    Ok(())
}

fn spaces(width: u8) -> Indentation {
    Indentation::spaces(NonZeroU8::new(width).unwrap())
}

fn workspace() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let real = canonicalize_simplified(dir.path()).unwrap();
    (dir, real)
}

/// A Git worktree whose `.editorconfig` asks for two-space YAML.
fn two_space_repository() -> (tempfile::TempDir, PathBuf) {
    let (dir, root) = workspace();
    git2::Repository::init(&root).unwrap();
    std::fs::write(root.join(".editorconfig"), "root = true\n\n[*.yaml]\nindent_style = space\nindent_size = 2\n").unwrap();
    (dir, root)
}

/// A resolver that answers `width` and counts how often it was asked.
fn counting(width: u8) -> (Arc<AtomicUsize>, impl Fn(&Path) -> Result<Indentation, String> + Send + Sync + 'static) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    (calls, move |_: &Path| {
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(spaces(width))
    })
}

fn commit(change: TargetChange) -> darkmatter::config::write::CommitOutcome {
    let mut transaction = ConfigTransaction::new();
    transaction.add(change).unwrap();
    transaction.commit(&accept).unwrap()
}

fn permissions() -> Value {
    json!({"permissions": {"file-read": ["./*.txt", "./*.md"]}})
}

#[test]
fn a_generated_document_takes_the_destinations_editorconfig_width() {
    let (_dir, root) = two_space_repository();
    let target = root.join("config.yaml");
    let outcome = commit(TargetChange::create_yaml(&target, permissions(), vec![set("permissions.network", json!({"allow": ["a"]}))]));
    let text = std::fs::read_to_string(&target).unwrap();
    assert_eq!(text, "permissions:\n  file-read:\n    - ./*.txt\n    - ./*.md\n  network:\n    allow:\n      - a\n");
    let read: Value = biscuit_file::serde_yaml_ng::from_str(&text).unwrap();
    assert_eq!(read, json!({"permissions": {"file-read": ["./*.txt", "./*.md"], "network": {"allow": ["a"]}}}));
    assert_eq!(read["permissions"].as_object().unwrap().keys().collect::<Vec<_>>(), ["file-read", "network"]);
    assert!(outcome.targets[0].created && outcome.targets[0].indentation_warning.is_none());
}

#[test]
fn a_generated_document_outside_a_worktree_takes_four_spaces() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    commit(TargetChange::create_yaml(&target, permissions(), Vec::new()));
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "permissions:\n    file-read:\n        - ./*.txt\n        - ./*.md\n"
    );
}

#[test]
fn the_preference_is_resolved_once_across_batches_and_never_when_nothing_renders() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    std::fs::write(&target, "# mine\nname: x\n").unwrap();

    let (calls, resolver) = counting(2);
    let mut transaction = ConfigTransaction::new().with_indentation_resolver(resolver);
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("a.b", json!(1))])).unwrap();
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("c.d", json!(2))])).unwrap();
    transaction.commit(&accept).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1, "two batches with no example share one resolution");
    // The second batch had the first batch's block as its example; only the
    // first needed the preference.
    let first = std::fs::read_to_string(&target).unwrap();
    assert_eq!(first, "# mine\nname: x\na:\n  b: 1\nc:\n  d: 2\n");

    let (calls, resolver) = counting(7);
    let mut transaction = ConfigTransaction::new().with_indentation_resolver(resolver);
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("e.f", json!(3))])).unwrap();
    transaction.commit(&accept).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0, "a local example needs no preference");
    assert!(std::fs::read_to_string(&target).unwrap().ends_with("e:\n  f: 3\n"));

    // Two identical saves: the second changes no value, writes nothing, and
    // resolves nothing.
    let before = std::fs::read(&target).unwrap();
    let (calls, resolver) = counting(7);
    let mut transaction = ConfigTransaction::new().with_indentation_resolver(resolver);
    transaction.add(TargetChange::edit(&target, SourceExpectation::Current, vec![set("a.b", json!(1)), set("name", json!("x"))])).unwrap();
    let outcome = transaction.commit(&accept).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(!outcome.targets[0].written);
    assert_eq!(std::fs::read(&target).unwrap(), before);
}

#[test]
fn a_failed_resolution_falls_back_to_four_spaces_with_a_warning_naming_the_target() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    let transaction = ConfigTransaction::new().with_indentation_resolver(|_| Err("the worktree is unreadable".to_string()));
    let mut transaction = transaction;
    transaction.add(TargetChange::create_yaml(&target, json!({"a": {"b": 1}}), Vec::new())).unwrap();
    let outcome = transaction.commit(&accept).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "a:\n    b: 1\n");
    let warning = outcome.targets[0].indentation_warning.as_deref().expect("a warning");
    assert!(warning.contains(&target.display().to_string()), "{warning}");
    assert!(warning.contains("the worktree is unreadable") && warning.contains("using 4 spaces"), "{warning}");
}

#[test]
fn a_symlinked_target_is_laid_out_for_the_real_file() {
    let (_repo_dir, root) = two_space_repository();
    let real = root.join("config.yaml");
    std::fs::write(&real, "name: x\n").unwrap();
    let (_outside_dir, outside) = workspace();
    let link = outside.join("link.yaml");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &link).unwrap();
    #[cfg(windows)]
    if std::os::windows::fs::symlink_file(&real, &link).is_err() {
        eprintln!("symlink creation needs privilege on this host; skipped");
        return;
    }
    commit(TargetChange::edit(&link, SourceExpectation::Current, vec![set("a.b", json!(1))]));
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "name: x\na:\n  b: 1\n");
}

#[test]
fn an_authored_skeleton_is_an_example_for_its_edits() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    let (calls, resolver) = counting(2);
    let mut transaction = ConfigTransaction::new().with_indentation_resolver(resolver);
    transaction.add(TargetChange::create(&target, "x:\n   y: 1\n", vec![set("z.w", json!(1))])).unwrap();
    transaction.commit(&accept).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "x:\n   y: 1\nz:\n   w: 1\n");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn a_refused_generated_document_is_never_written() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::create_yaml(&target, permissions(), Vec::new())).unwrap();
    let error = transaction
        .commit(&|candidate| {
            assert!(candidate.candidate.starts_with("permissions:\n    file-read:\n"), "{}", candidate.candidate);
            Err(ValidationRefusal::new("no"))
        })
        .unwrap_err();
    assert!(matches!(error, ConfigWriteError::ValidationFailed { .. }), "{error:?}");
    assert!(!target.exists());
}

#[test]
fn a_generated_document_meets_a_file_created_since_as_a_change() {
    let (_dir, root) = workspace();
    let target = root.join("config.yaml");
    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::create_yaml(&target, permissions(), Vec::new())).unwrap();
    std::fs::write(&target, "other: 1\n").unwrap();
    let error = transaction.commit(&accept).unwrap_err();
    assert!(matches!(error, ConfigWriteError::Changed { .. }), "{error:?}");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "other: 1\n");
}
