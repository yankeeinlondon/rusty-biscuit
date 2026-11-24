//! `claudine::config::setup`: Claudine's setup handshake with Darkmatter's
//! `.darkmatter.yaml`, in-process over a temporary home. Each case reads both
//! files back with the loaders that use them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use claudine::config::claudine_config::ClaudineConfig;
use claudine::config::persist::{SaveOutcome, initialize_config};
use claudine::config::schema::ConfigScope;
use claudine::config::setup::{PermissionsHandoff, SetupError, setup_with_darkmatter};
use darkmatter::config::init::{InitAction, InitOptions, default_permissions, init_user};
use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot};
use darkmatter::config::schema::ConfigScope as DarkmatterScope;
use serde_json::{Value, json};
use simplified_schema::yaml_documents::load_yaml_document;

const CUSTOM: &str = "# my permissions\npermissions:\n  shell-commands:\n    - prefix ls   # listing\n  network: allow\n";

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        for name in ["home", "work", "repo"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        Self { _dir: dir, root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn darkmatter(&self) -> PathBuf {
        self.home().join(".darkmatter.yaml")
    }

    fn claudine(&self) -> PathBuf {
        self.home().join(".claudine/config.yaml")
    }

    fn write(&self, path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.home()), HashMap::new())
    }

    /// Claudine's setup with its default configuration, as a headless first run writes it.
    fn setup(&self) -> Result<claudine::config::setup::DarkmatterSetup, SetupError> {
        setup_with_darkmatter(&self.context(), ConfigScope::User, write_default)
    }

    /// The user block the Darkmatter policy is evaluated from, and the file holding it.
    fn policy_block(&self) -> (PathBuf, Value) {
        let snapshot = ConfigSnapshot::load(&self.context(), None, ConfigLoadMode::Passive).unwrap();
        let policy = snapshot.policy().unwrap_or_else(|error| panic!("{error}"));
        let file = policy.user().block_file().to_path_buf();
        let block = document(&file)["permissions"].clone();
        (file, block)
    }

    /// Every file under the fixture, with its bytes.
    fn files(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    out.push((path.clone(), std::fs::read(&path).unwrap()));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out.sort();
        out
    }
}

fn write_default(path: &Path) -> claudine::error::Result<SaveOutcome> {
    initialize_config(path, ConfigScope::User, &ClaudineConfig::default(), &ClaudineConfig::default())
}

fn document(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap();
    load_yaml_document(&text, path).unwrap().json
}

fn validate_claudine(path: &Path) {
    let text = std::fs::read_to_string(path).unwrap();
    claudine::config::persist::validate_config_text(&text, path, ConfigScope::User).unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn setup_neither_file_creates_the_reference_and_the_default_block() {
    let fixture = Fixture::new();

    let setup = fixture.setup().unwrap();

    assert_eq!(setup.permissions, PermissionsHandoff::Referenced);
    assert_eq!(setup.darkmatter_action, InitAction::Created);
    assert_eq!(document(&fixture.darkmatter()), json!({ "permissions": "~/.claudine/config.yaml", "register": [] }));
    assert_eq!(document(&fixture.claudine())["permissions"], default_permissions(DarkmatterScope::User));
    validate_claudine(&fixture.claudine());
    let (file, block) = fixture.policy_block();
    assert_eq!((file, block), (fixture.claudine(), default_permissions(DarkmatterScope::User)));
}

#[test]
fn setup_claudine_first_then_md_init_leaves_the_reference() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    let before = std::fs::read(fixture.darkmatter()).unwrap();

    let report = init_user(&fixture.context(), InitOptions::default()).unwrap();

    assert_eq!(report.files[0].action, InitAction::Unchanged);
    assert_eq!(std::fs::read(fixture.darkmatter()).unwrap(), before);
}

#[test]
fn setup_darkmatter_first_moves_the_inline_block_into_claudine() {
    let fixture = Fixture::new();
    fixture.write(&fixture.darkmatter(), &format!("{CUSTOM}register:\n  - ~/hosts/mine.yaml\n"));
    let inline = document(&fixture.darkmatter())["permissions"].clone();

    let setup = fixture.setup().unwrap();

    assert_eq!(setup.permissions, PermissionsHandoff::Transferred);
    assert_eq!(setup.darkmatter_action, InitAction::Updated);
    assert_eq!(document(&fixture.claudine())["permissions"], inline);
    let text = std::fs::read_to_string(fixture.darkmatter()).unwrap();
    assert!(text.starts_with("# my permissions\npermissions: ~/.claudine/config.yaml\n"), "{text}");
    assert!(text.ends_with("register:\n  - ~/hosts/mine.yaml\n"), "register entries are kept: {text}");
    validate_claudine(&fixture.claudine());
    assert_eq!(fixture.policy_block(), (fixture.claudine(), inline), "the same policy, now held by Claudine");
}

#[test]
fn setup_repeated_changes_nothing_and_keeps_authored_values() {
    let fixture = Fixture::new();
    fixture.write(&fixture.darkmatter(), CUSTOM);
    fixture.setup().unwrap();
    let mut claudine_text = std::fs::read_to_string(fixture.claudine()).unwrap();
    claudine_text.insert_str(0, "# authored by me\n");
    std::fs::write(fixture.claudine(), &claudine_text).unwrap();
    let before = fixture.files();

    let setup = fixture.setup().unwrap();

    assert_eq!(setup.permissions, PermissionsHandoff::AlreadyReferenced);
    assert_eq!(setup.darkmatter_action, InitAction::Unchanged);
    assert_eq!(fixture.files(), before, "byte for byte, comments included");
}

#[test]
fn setup_hands_off_an_aliased_user_file_as_the_user_file() {
    // The user file seen from a repository: through a linked repository
    // `.darkmatter.yaml`, or with the home as the repository root. Its new
    // `~/.claudine/config.yaml` reference is a user reference, which a
    // repository file could not hold.
    let mut layouts = vec!["home"];
    if cfg!(unix) {
        layouts.push("linked");
    }
    for layout in layouts {
        let fixture = Fixture::new();
        let inline = "# mine\npermissions:\n  file-read:\n    - ~/notes/*.md\nregister: []\n";
        fixture.write(&fixture.darkmatter(), inline);
        let root = if layout == "home" { fixture.home() } else { fixture.root.join("repo") };
        #[cfg(unix)]
        if layout == "linked" {
            std::os::unix::fs::symlink(fixture.darkmatter(), root.join(".darkmatter.yaml")).unwrap();
        }
        let context =
            FileResolutionContext::from_snapshot(root.clone(), Some(fixture.home()), HashMap::new()).with_repository_root(&root);

        let setup = setup_with_darkmatter(&context, ConfigScope::User, write_default)
            .unwrap_or_else(|error| panic!("{layout}: {error}"));

        assert_eq!(setup.permissions, PermissionsHandoff::Transferred, "{layout}");
        assert_eq!(document(&fixture.darkmatter())["permissions"], "~/.claudine/config.yaml", "{layout}");
        assert_eq!(document(&fixture.claudine())["permissions"], json!({ "file-read": ["~/notes/*.md"] }), "{layout}");
    }
}

#[test]
fn setup_conflicting_blocks_write_neither_permissions() {
    let fixture = Fixture::new();
    fixture.write(&fixture.darkmatter(), CUSTOM);
    fixture.write(&fixture.claudine(), "permissions:\n  network: deny\n  shell-commands:\n    - exact ps -e\n");
    let darkmatter_before = std::fs::read(fixture.darkmatter()).unwrap();

    let error = fixture.setup().unwrap_err();

    match &error {
        SetupError::Conflict { darkmatter, claudine } => {
            assert_eq!((darkmatter, claudine), (&fixture.darkmatter(), &fixture.claudine()));
        }
        other => panic!("{other}"),
    }
    let message = error.to_string();
    assert!(message.contains(".darkmatter.yaml") && message.contains("config.yaml"), "{message}");
    assert_eq!(std::fs::read(fixture.darkmatter()).unwrap(), darkmatter_before);
    assert_eq!(document(&fixture.claudine())["permissions"], json!({ "network": "deny", "shell-commands": ["exact ps -e"] }));
}

#[test]
fn setup_equal_blocks_transfer() {
    let fixture = Fixture::new();
    fixture.write(&fixture.darkmatter(), "permissions:\n  network: deny\n");
    fixture.write(&fixture.claudine(), &format!("permissions: {}\n", default_permissions(DarkmatterScope::User)));

    let setup = fixture.setup().unwrap();

    assert_eq!(setup.permissions, PermissionsHandoff::Transferred, "a block equal once defaults are filled is no conflict");
    assert_eq!(document(&fixture.darkmatter())["permissions"], "~/.claudine/config.yaml");
}

#[test]
fn setup_keeps_a_third_party_reference() {
    let fixture = Fixture::new();
    fixture.write(&fixture.root.join("home/.other/policy.yaml"), "permissions:\n  network: allow\n");
    let authored = "permissions: ~/.other/policy.yaml\nregister: []\n";
    fixture.write(&fixture.darkmatter(), authored);

    let setup = fixture.setup().unwrap();

    assert_eq!(setup.permissions, PermissionsHandoff::ThirdParty { reference: "~/.other/policy.yaml".to_string() });
    assert_eq!(std::fs::read_to_string(fixture.darkmatter()).unwrap(), authored);
    assert_eq!(fixture.policy_block().0, fixture.root.join("home/.other/policy.yaml"));
}

#[test]
fn setup_broken_reference_is_repaired_by_md_init_inline() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    std::fs::write(fixture.claudine(), "{}\n").unwrap();
    assert!(ConfigSnapshot::load(&fixture.context(), None, ConfigLoadMode::Passive).unwrap().policy().is_err());

    let report = init_user(&fixture.context(), InitOptions { inline: true }).unwrap();

    assert_eq!(report.notices.len(), 1, "defaults stand in for the unreadable block: {report:?}");
    assert_eq!(fixture.policy_block(), (fixture.darkmatter(), default_permissions(DarkmatterScope::User)));
}

#[test]
fn setup_interrupted_transfer_leaves_the_inline_block_in_force() {
    let fixture = Fixture::new();
    fixture.write(&fixture.darkmatter(), CUSTOM);
    let darkmatter_before = std::fs::read(fixture.darkmatter()).unwrap();
    let inline = document(&fixture.darkmatter())["permissions"].clone();

    // Claudine's own write and the block's move into its file succeed; the
    // reference's replacement fails.
    let error = {
        let _hook = darkmatter::config::write::test_hooks::fail_replacement_after(2);
        fixture.setup().unwrap_err()
    };

    assert!(matches!(error, SetupError::Darkmatter(_)), "{error}");
    assert_eq!(std::fs::read(fixture.darkmatter()).unwrap(), darkmatter_before);
    assert_eq!(document(&fixture.claudine())["permissions"], inline, "the copy landed first");
    assert_eq!(fixture.policy_block(), (fixture.darkmatter(), inline), "the original inline block is still in force");

    let retried = fixture.setup().unwrap();
    assert_eq!(retried.permissions, PermissionsHandoff::Transferred, "a retry finishes the move");
    assert_eq!(document(&fixture.darkmatter())["permissions"], "~/.claudine/config.yaml");
}

#[test]
fn setup_repository_outside_a_repository_writes_nothing() {
    let fixture = Fixture::new();
    let before = fixture.files();
    let mut wrote = false;

    let error = setup_with_darkmatter(&fixture.context(), ConfigScope::Repository, |_| {
        wrote = true;
        Ok(SaveOutcome::default())
    })
    .unwrap_err();

    assert!(matches!(error, SetupError::NotARepository { .. }), "{error}");
    assert!(!wrote, "Claudine's own write never ran");
    assert_eq!(fixture.files(), before);
}

#[test]
fn setup_repository_scope_refers_to_the_repository_claudine_file() {
    let fixture = Fixture::new();
    let root = fixture.root.join("repo");
    let context = fixture.context().with_repository_root(&root);

    let setup = setup_with_darkmatter(&context, ConfigScope::Repository, |path| {
        initialize_config(
            path,
            ConfigScope::Repository,
            &claudine::config::claudine_config::RepoOverrideConfig::default(),
            &claudine::config::claudine_config::RepoOverrideConfig::default(),
        )
    })
    .unwrap();

    assert_eq!(setup.permissions, PermissionsHandoff::Referenced);
    assert_eq!(document(&root.join(".darkmatter.yaml"))["permissions"], "./.claudine/config.yaml");
    assert_eq!(document(&root.join(".claudine/config.yaml"))["permissions"], default_permissions(DarkmatterScope::Repository));
    assert!(!fixture.darkmatter().exists(), "the user scope is untouched");
}

#[test]
fn setup_without_a_home_writes_nothing() {
    let fixture = Fixture::new();
    let context = FileResolutionContext::from_snapshot(fixture.root.join("work"), None, HashMap::new());
    let before = fixture.files();

    let error = setup_with_darkmatter(&context, ConfigScope::User, write_default).unwrap_err();

    assert!(matches!(error, SetupError::NoHome), "{error}");
    assert_eq!(fixture.files(), before);
}

/// `permissions: null` is no authored block to Darkmatter's loader (the
/// schema default stands in for it) and to Claudine's, so setup hands it off
/// as an omitted setting: only the reference is written, Claudine's file
/// keeps its own block or gets the default one, and `null` is never copied.
macro_rules! null_handoff_cases {
    ($($name:ident: darkmatter $darkmatter:expr, claudine $claudine:expr => $handoff:expr, $block:expr;)*) => {$(
        #[test]
        fn $name() {
            let fixture = Fixture::new();
            fixture.write(&fixture.darkmatter(), $darkmatter);
            if let Some(claudine) = $claudine {
                fixture.write(&fixture.claudine(), claudine);
            }

            let setup = fixture.setup().unwrap_or_else(|error| panic!("{error}"));

            assert_eq!(setup.permissions, $handoff);
            assert_eq!(document(&fixture.darkmatter())["permissions"], "~/.claudine/config.yaml");
            let block: Value = $block;
            assert_eq!(document(&fixture.claudine())["permissions"], block, "never `null`");
            validate_claudine(&fixture.claudine());
            assert_eq!(fixture.policy_block(), (fixture.claudine(), block), "the reference resolves to the block");
        }
    )*};
}

null_handoff_cases! {
    setup_null_darkmatter_permissions_with_no_claudine_block_gets_the_default:
        darkmatter "permissions: null\n", claudine None::<&str>
        => PermissionsHandoff::Referenced, default_permissions(DarkmatterScope::User);
    setup_null_darkmatter_permissions_keeps_claudines_block:
        darkmatter "permissions: null\n", claudine Some("permissions:\n  network: allow\n")
        => PermissionsHandoff::Referenced, json!({ "network": "allow" });
    setup_null_claudine_permissions_with_omitted_darkmatter_gets_the_default:
        darkmatter "register: []\n", claudine Some("permissions: null\n")
        => PermissionsHandoff::Referenced, default_permissions(DarkmatterScope::User);
    setup_null_claudine_permissions_takes_the_inline_block:
        darkmatter "permissions:\n  network: allow\n", claudine Some("permissions: null\n")
        => PermissionsHandoff::Transferred, json!({ "network": "allow" });
    setup_null_on_both_sides_gets_the_default:
        darkmatter "permissions: null\n", claudine Some("permissions: null\n")
        => PermissionsHandoff::Referenced, default_permissions(DarkmatterScope::User);
}
