use super::*;
use claudine::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};
use claudine::config::tts::{Gender, TtsConfigSettings, TtsValue};
use claudine::dispatch::loader::load_repo_override_config;
use darkmatter::config::write::{ConfigTransaction, SourceExpectation, TargetChange};
use std::path::PathBuf;

const USER: &str = "\
# my Claudine settings
preferred_agent: 'claude'   # quoted

logging: true
prompt_for_missing: true
tts:
  gender: female

# written by Darkmatter
permissions:
  network: deny
";

fn file(text: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    std::fs::write(&path, text).unwrap();
    (dir, path)
}

fn open(path: &Path) -> App {
    let config = load_claudine_config(Some(path), None).unwrap();
    App::new(config, None, None, false, None, None)
}

fn open_repo(user: &Path, repo: &Path) -> App {
    let config = load_claudine_config(Some(user), None).unwrap();
    let repo_config = load_repo_override_config(repo).unwrap();
    App::new(config, repo_config, Some(repo.to_path_buf()), true, None, None)
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn a_session_saves_only_the_leaf_it_changed() {
    let (_dir, path) = file(USER);
    let mut app = open(&path);
    app.config.logging = false;

    assert!(save_user(&app, &path).unwrap().written);

    assert_eq!(read(&path), USER.replace("logging: true", "logging: false"));
}

#[test]
fn an_unchanged_session_writes_nothing() {
    let (_dir, path) = file(USER);
    let app = open(&path);
    assert!(!save_user(&app, &path).unwrap().written);
    assert_eq!(read(&path), USER);
}

#[test]
fn two_sessions_and_a_permissions_writer_keep_every_edit() {
    let (_dir, path) = file(USER);
    let mut first = open(&path);
    let mut second = open(&path);

    let mut darkmatter = ConfigTransaction::new();
    darkmatter
        .add(TargetChange::edit(
            &path,
            SourceExpectation::Current,
            vec![persist::set_path("permissions.network", serde_json::json!("allow")).unwrap()],
        ))
        .unwrap();
    darkmatter.commit(&|_| Ok(())).unwrap();

    first.config.logging = false;
    save_user(&first, &path).unwrap();
    second.config.tts =
        TtsValue::Config(TtsConfigSettings { provider: None, voice: None, gender: Gender::Male, prewarm: None });
    save_user(&second, &path).unwrap();

    let expected = USER
        .replace("logging: true", "logging: false")
        .replace("gender: female", "gender: male")
        .replace("network: deny", "network: allow");
    assert_eq!(read(&path), expected);
}

#[test]
fn a_repository_session_writes_only_the_overrides_it_changed() {
    let dir = tempfile::tempdir().unwrap();
    let user = dir.path().join("user.yaml");
    std::fs::write(&user, "{}\n").unwrap();
    let repo = dir.path().join("repo/.claudine/config.yaml");

    // No file and nothing changed: nothing is created.
    let app = App::new(ClaudineConfig::default(), None, Some(repo.clone()), true, None, None);
    assert!(!save_repo(&app).unwrap().unwrap().written);
    assert!(!repo.exists());

    // A new override creates the file with that key alone.
    let mut app = App::new(ClaudineConfig::default(), None, Some(repo.clone()), true, None, None);
    app.repo_config = Some(RepoOverrideConfig { canonical_provider: Some(Provider::Codex), ..RepoOverrideConfig::default() });
    assert!(save_repo(&app).unwrap().unwrap().created);
    assert_eq!(read(&repo), "canonical_provider: codex\n");

    // Clearing it keeps what else the file holds.
    std::fs::write(&repo, "# team settings\ncanonical_provider: codex\npermissions:\n  network: deny\n").unwrap();
    let mut app = open_repo(&user, &repo);
    app.repo_config.as_mut().unwrap().canonical_provider = None;
    save_repo(&app).unwrap().unwrap();
    assert_eq!(read(&repo), "# team settings\npermissions:\n  network: deny\n");
    assert!(load_repo_override_config(&repo).unwrap().unwrap().canonical_provider.is_none());
}

#[test]
fn the_favorite_agent_setter_changes_only_its_setting() {
    let (_dir, path) = file(USER);

    assert_eq!(set_favorite_agent(&path, Some(Provider::Codex)).unwrap(), SetOutcome::Updated);
    assert_eq!(read(&path), USER.replace("preferred_agent: 'claude'", "preferred_agent: codex"));

    assert_eq!(set_favorite_agent(&path, Some(Provider::Codex)).unwrap(), SetOutcome::Unchanged);

    set_favorite_agent(&path, None).unwrap();
    assert_eq!(read(&path), USER.replace("preferred_agent: 'claude'   # quoted\n", ""));
}

#[test]
fn the_prompt_for_missing_setter_changes_only_its_setting() {
    let (_dir, path) = file(USER);

    assert_eq!(set_prompt_for_missing(&path, true).unwrap(), SetOutcome::Unchanged);
    assert_eq!(read(&path), USER);

    assert_eq!(set_prompt_for_missing(&path, false).unwrap(), SetOutcome::Updated);
    assert_eq!(read(&path), USER.replace("prompt_for_missing: true", "prompt_for_missing: false"));
}

#[test]
fn a_setter_on_an_invalid_file_writes_nothing() {
    let (_dir, path) = file("logging: \"yes\"\n");
    assert!(set_prompt_for_missing(&path, false).is_err());
    assert_eq!(read(&path), "logging: \"yes\"\n");
}
