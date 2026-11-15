//! `darkmatter::config::permissions::persist`: saving "always allow"
//! approvals to the user scope through the public API. Each case reads the
//! configuration, saves, and reads it back; every byte outside the edited
//! list is compared.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::load::{CONFIG_NOT_SAVED_CODE, ConfigLoadMode, ConfigSnapshot};
use darkmatter::config::permissions::persist::{NotSavedReason, PersistentApproval, save_user_approval};
use darkmatter::config::schema::CONFIG_FILE_NAME;
use darkmatter::markdown::compose::shell_expansion::store::parse_rule_entry;
use sniff::filesystem::git::GitRepo;

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        for name in ["home", "work", "outside"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        Self { _dir: dir, root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn user_file(&self) -> PathBuf {
        self.home().join(CONFIG_FILE_NAME)
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.home()), HashMap::new())
    }

    fn load(&self) -> ConfigSnapshot {
        ConfigSnapshot::load(&self.context(), None, ConfigLoadMode::Passive).unwrap()
    }

    fn file_approval(&self, target: &Path) -> PersistentApproval {
        PersistentApproval::FileRead { target: target.to_path_buf(), reading: Box::new(self.context()) }
    }
}

fn shell_rule(text: &str) -> PersistentApproval {
    PersistentApproval::ShellRule { rule: parse_rule_entry(text).unwrap(), command: None }
}

fn save(snapshot: &ConfigSnapshot, approval: &PersistentApproval) -> Result<darkmatter::config::permissions::persist::SavedApproval, darkmatter::config::permissions::persist::NotSaved> {
    save_user_approval(snapshot.policy().unwrap(), approval)
}

const CLAUDINE: &str = "\
# Claudine's settings: comments, order, and quoting must survive.
model: 'opus'   # trailing comment

permissions:
    # commands I trust
    shell-commands:
        - prefix ls
    network: deny
hooks: {}
";

#[test]
fn an_approval_through_a_reference_edits_only_the_list_in_the_target() {
    let fixture = Fixture::new();
    fixture.write("home/.darkmatter.yaml", "# points at Claudine\npermissions: ~/.claudine/config.yaml\n");
    let target = fixture.write("home/.claudine/config.yaml", CLAUDINE);
    let snapshot = fixture.load();
    let rule = shell_rule("template git add {{ string }}");

    let saved = save(&snapshot, &rule).unwrap();
    assert_eq!((saved.file.as_path(), saved.list, saved.added), (target.as_path(), "shell-commands", true));
    assert_eq!(saved.entry, "template git add {{ string }}");
    let expected = CLAUDINE.replace("        - prefix ls\n", "        - prefix ls\n        - template git add {{ string }}\n");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), expected);
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), "# points at Claudine\npermissions: ~/.claudine/config.yaml\n");

    // Read again: the saved rule is part of the policy, and saving it again
    // changes nothing.
    let reread = fixture.load();
    let rules = &reread.policy().unwrap().user().shell_allows().entries;
    assert!(rules.contains(&parse_rule_entry("template git add {{ string }}").unwrap()), "{rules:?}");
    assert!(!save(&reread, &rule).unwrap().added);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), expected);
}

#[test]
fn an_omitted_list_gets_its_schema_default_before_the_approval() {
    let fixture = Fixture::new();
    let original = "# mine\npermissions:\n  network: deny # keep\n";
    fixture.write("home/.darkmatter.yaml", original);
    let file = fixture.write("home/data/[id].yaml", "a: 1\n");

    let saved = save(&fixture.load(), &fixture.file_approval(&file)).unwrap();
    assert!(saved.materialized_defaults && saved.added);
    assert_eq!(saved.entry, "~/data/[[]id[]].yaml");
    let text = std::fs::read_to_string(fixture.user_file()).unwrap();
    assert!(text.starts_with("# mine\npermissions:\n  network: deny # keep\n"), "{text}");

    let reread = fixture.load();
    let policy = reread.policy().unwrap();
    let block = policy.user().permissions();
    assert!(block.file_read.authored);
    let patterns: Vec<&str> = block.file_read.value.iter().map(|entry| entry.authored.as_str()).collect();
    assert_eq!(patterns.len(), 11, "{patterns:?}");
    assert_eq!((patterns[0], patterns[10]), ("./*.txt", "~/data/[[]id[]].yaml"));
    assert!(policy.file_read(&file, &fixture.context()).is_preapproved());
    assert!(!policy.file_read(&fixture.home().join("data/i.yaml"), &fixture.context()).is_preapproved(), "exact, not a pattern");
}

#[test]
fn a_missing_user_file_is_created_with_the_approval() {
    let fixture = Fixture::new();
    let saved = save(&fixture.load(), &shell_rule("exact ps -e")).unwrap();
    assert_eq!(saved.file, fixture.user_file());
    let reread = fixture.load();
    let block = reread.policy().unwrap().user().permissions().clone();
    assert_eq!(block.shell_commands.value[0].authored, "exact ps -e");
    assert!(!block.file_read.authored, "other lists keep their defaults");
}

#[test]
fn an_exclusion_that_still_matches_is_reported_and_kept() {
    let fixture = Fixture::new();
    let original = "permissions:\n  shell-commands:\n    - \"!prefix rm\"\n  file-read:\n    - ./*.md\n    - \"!~/private/**\"\n";
    fixture.write("home/.darkmatter.yaml", original);
    let snapshot = fixture.load();
    let private = fixture.write("home/private/a.md", "x\n");

    for approval in [shell_rule("exact rm -rf build"), shell_rule("prefix rm"), fixture.file_approval(&private)] {
        let refused = save(&snapshot, &approval).expect_err("excluded");
        assert_eq!((refused.reason, refused.code()), (NotSavedReason::Excluded, CONFIG_NOT_SAVED_CODE), "{approval:?}");
        assert!(refused.message.contains("still excludes"), "{}", refused.message);
    }
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), original);
}

#[test]
fn approvals_that_cannot_be_portable_exact_files_are_not_saved() {
    let fixture = Fixture::new();
    let snapshot = fixture.load();
    let outside = fixture.write("outside/a.md", "x\n");
    let refused = save(&snapshot, &fixture.file_approval(&outside)).expect_err("outside the home");
    assert_eq!(refused.reason, NotSavedReason::NotPortable);
    std::fs::create_dir_all(fixture.home().join("data")).unwrap();
    let refused_dir = save(&snapshot, &fixture.file_approval(&fixture.home().join("data"))).expect_err("a directory");
    assert_eq!(refused_dir.reason, NotSavedReason::NotAFile);
    assert!(!fixture.user_file().exists(), "nothing was written");

    let no_home = FileResolutionContext::from_snapshot(fixture.root.join("work"), None, HashMap::new());
    let snapshot = ConfigSnapshot::load(&no_home, None, ConfigLoadMode::Passive).unwrap();
    let refused = save(&snapshot, &shell_rule("prefix ls")).expect_err("no home");
    assert_eq!(refused.reason, NotSavedReason::NoHome);
}

#[test]
fn an_edit_that_would_be_invalid_or_cannot_be_made_in_place_leaves_the_file_untouched() {
    let fixture = Fixture::new();
    fixture.write("home/.darkmatter.yaml", "permissions:\n  shell-commands: [prefix ls]\n");
    let snapshot = fixture.load();
    let refused = save(&snapshot, &shell_rule("prefix cat")).expect_err("a flow list is not edited in place");
    assert_eq!(refused.reason, NotSavedReason::InvalidEdit, "{refused}");
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), "permissions:\n  shell-commands: [prefix ls]\n");

    // Edited after capture into a block the save would leave invalid.
    fixture.write("home/.darkmatter.yaml", "permissions:\n  shell-commands:\n    - prefix ls\n  network: sometimes\n");
    let refused = save(&snapshot, &shell_rule("prefix cat")).expect_err("the edited block would be invalid");
    assert_eq!(refused.reason, NotSavedReason::InvalidEdit, "{refused}");
    assert_eq!(
        std::fs::read_to_string(fixture.user_file()).unwrap(),
        "permissions:\n  shell-commands:\n    - prefix ls\n  network: sometimes\n"
    );
}

#[test]
fn a_save_inside_a_repository_never_edits_the_repository_file() {
    let fixture = Fixture::new();
    let work = fixture.root.join("work");
    git2::Repository::init(&work).unwrap();
    let repository_file = fixture.write("work/.darkmatter.yaml", "permissions:\n  shell-commands:\n    - prefix ls\n");
    let tree = GitRepo::discover(&work).unwrap().unwrap().committed_tree().unwrap();
    let snapshot =
        ConfigSnapshot::load(&fixture.context().with_repository_root(&work), Some(&tree), ConfigLoadMode::Passive).unwrap();
    let saved = save(&snapshot, &shell_rule("prefix cat")).unwrap();
    assert_eq!(saved.file, fixture.user_file());
    assert_eq!(std::fs::read_to_string(repository_file).unwrap(), "permissions:\n  shell-commands:\n    - prefix ls\n");
}
