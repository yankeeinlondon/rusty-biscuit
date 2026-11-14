//! `darkmatter::config::load`: the field cells of the configuration
//! input-robustness matrix, through the public `ConfigSnapshot` API.
//!
//! `file-read` gets the full shape walk of the non-null string-list
//! validator; `shell-commands` and `register` share that shape and get a
//! control plus one invalid case each. Every invalid shape is a located
//! error; none is recovered with a default.

use std::collections::HashMap;
use std::path::PathBuf;

use biscuit_file::{FileReferenceKind, FileResolutionContext, PORTABLE_ENV_VARIABLES, canonicalize_simplified};
use darkmatter::config::load::{
    CONFIG_INVALID_CODE, CONFIG_REFERENCE_CODE, ConfigError, ConfigErrorKind, ConfigLoadMode, ConfigSnapshot,
    NetworkAccess, PermissionsSetting, ScopePermissions,
};
use darkmatter::config::schema::CONFIG_FILE_NAME;
use darkmatter::markdown::compose::shell_expansion::types::ShellRuleEntry;
use sniff::filesystem::git::GitRepo;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    User,
    Repository,
}

/// Loads `text` as the `scope` file in a fresh temporary home and working
/// directory (a `git init` repository for [`Scope::Repository`]).
fn load(scope: Scope, text: &str, env: HashMap<String, String>) -> (ConfigSnapshot, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = canonicalize_simplified(dir.path()).unwrap();
    let (home, work) = (root.join("home"), root.join("work"));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    let context = FileResolutionContext::from_snapshot(&work, Some(home.clone()), env);
    let (snapshot, file) = match scope {
        Scope::User => {
            let file = home.join(CONFIG_FILE_NAME);
            std::fs::write(&file, text).unwrap();
            (ConfigSnapshot::load(&context, None, ConfigLoadMode::Passive), file)
        }
        Scope::Repository => {
            git2::Repository::init(&work).unwrap();
            let file = work.join(CONFIG_FILE_NAME);
            std::fs::write(&file, text).unwrap();
            let tree = GitRepo::discover(&work).unwrap().unwrap().committed_tree().unwrap();
            (ConfigSnapshot::load(&context.with_repository_root(&work), Some(&tree), ConfigLoadMode::Passive), file)
        }
    };
    (snapshot.unwrap_or_else(|error| panic!("{text}: whole-file failure {error}")), file)
}

/// The scope's own block from `text`.
fn block(scope: Scope, text: &str) -> ScopePermissions {
    block_with(scope, text, HashMap::new())
}

fn block_with(scope: Scope, text: &str, env: HashMap<String, String>) -> ScopePermissions {
    let (snapshot, _) = load(scope, text, env);
    let permissions = snapshot.permissions().unwrap_or_else(|error| panic!("{text}: {error}"));
    let setting = match scope {
        Scope::User => &permissions.user,
        Scope::Repository => &permissions.repository.as_ref().unwrap().worktree,
    };
    match setting {
        PermissionsSetting::Inline(block) => block.clone(),
        other => panic!("{text}: {other:?}"),
    }
}

/// The permissions error `text` produces, with its first problem's pointer
/// and 1-based line.
fn refused(scope: Scope, text: &str) -> (ConfigError, String, Option<usize>) {
    refused_with(scope, text, HashMap::new())
}

fn refused_with(scope: Scope, text: &str, env: HashMap<String, String>) -> (ConfigError, String, Option<usize>) {
    let (snapshot, file) = load(scope, text, env);
    let error = snapshot.permissions().expect_err(text);
    let first = error.problems()[0].clone();
    assert_eq!(first.file, file, "{text}");
    assert!(first.column.is_some() || first.line.is_none(), "{text}");
    (error, first.pointer, first.line)
}

fn patterns(block: &ScopePermissions) -> Vec<&str> {
    block.file_read.value.iter().map(|entry| entry.authored.as_str()).collect()
}

const TEXT_FORMATS: [&str; 10] = ["txt", "text", "md", "csv", "tsv", "json", "json5", "yaml", "yml", "toml"];

// ---- file-read: the full shape walk ------------------------------------------

#[test]
fn absent_file_read_takes_each_scope_default_from_the_schema() {
    let user = block(Scope::User, "permissions: {}\n");
    let expected: Vec<String> = TEXT_FORMATS.iter().map(|ext| format!("./*.{ext}")).collect();
    assert_eq!(patterns(&user), expected);
    assert!(!user.file_read.authored);

    let repository = block(Scope::Repository, "register: []\n");
    let expected: Vec<String> = TEXT_FORMATS.iter().map(|ext| format!("&**/*.{ext}")).collect();
    assert_eq!(patterns(&repository), expected);
    assert!(!repository.file_read.authored);
}

#[test]
fn an_authored_file_read_list_replaces_the_default() {
    let user = block(Scope::User, "permissions:\n  file-read: [\"&docs/**/*.md\", \"!&docs/secret/**\"]\n");
    assert_eq!(patterns(&user), ["&docs/**/*.md", "!&docs/secret/**"]);
    assert!(user.file_read.authored);
    assert!(!user.file_read.value[0].exclude && user.file_read.value[1].exclude);
    assert_eq!(user.file_read.value[1].pattern, "&docs/secret/**");

    let empty = block(Scope::User, "permissions:\n  file-read: []\n");
    assert!(empty.file_read.value.is_empty() && empty.file_read.authored, "`[]` is authored, never absent");

    let exclusions = block(Scope::User, "permissions:\n  file-read: [\"!./*.md\"]\n");
    assert_eq!(patterns(&exclusions), ["!./*.md"], "a list of exclusions only is allowed");
}

#[test]
fn every_wrong_file_read_shape_is_a_located_error() {
    for (text, pointer, line) in [
        ("permissions:\n  file-read:\n", "/permissions/file-read", 2),
        ("permissions:\n  file-read: null\n", "/permissions/file-read", 2),
        ("permissions:\n  file-read: ./*.md\n", "/permissions/file-read", 2),
        ("permissions:\n  file-read: {a: b}\n", "/permissions/file-read", 2),
        ("permissions:\n  file-read: 3\n", "/permissions/file-read", 2),
        ("permissions:\n  file-read:\n    - ./*.md\n    - 4\n", "/permissions/file-read/1", 4),
        ("permissions:\n  file-read: [1, true]\n", "/permissions/file-read/0", 2),
        ("permissions:\n  file-read: [\"   \"]\n", "/permissions/file-read/0", 2),
        ("permissions:\n  file-read: [null]\n", "/permissions/file-read/0", 2),
    ] {
        let (error, at, at_line) = refused(Scope::User, text);
        assert_eq!(error.kind(), ConfigErrorKind::InvalidSetting, "{text}");
        assert_eq!(error.code(), CONFIG_INVALID_CODE, "{text}");
        assert_eq!((at.as_str(), at_line), (pointer, Some(line)), "{text}: {error}");
    }
    let (error, _, _) = refused(Scope::User, "permissions:\n  file-read: [1, true]\n");
    assert_eq!(error.problems().len(), 2, "every wrong element is reported: {error}");
}

#[test]
fn file_read_entries_must_be_portable_glob_references() {
    for (entry, reason) in [
        ("/etc/*.md", "absolute"),
        ("C:\\\\data\\\\*.md", "absolute"),
        ("!", "needs a pattern"),
        ("![", "glob"),
        ("./[", "glob"),
        ("https://example.com/*.md", "remote"),
        ("%./*.md", "%"),
        ("@notes/*.md", "search or remote root"),
        ("{{UNDECLARED}}/*.md", "not a declared portable variable"),
    ] {
        let text = format!("permissions:\n  file-read:\n    - ./*.md\n    - \"{entry}\"\n");
        let (error, pointer, line) = refused(Scope::User, &text);
        assert_eq!((pointer.as_str(), line), ("/permissions/file-read/1", Some(4)), "{entry}");
        assert!(error.to_string().contains(reason), "{entry}: {error}");
    }
}

#[test]
fn home_and_declared_variables_are_user_scope_patterns() {
    let env = HashMap::from([
        (PORTABLE_ENV_VARIABLES.to_string(), "NOTES".to_string()),
        ("NOTES".to_string(), "/elsewhere/notes".to_string()),
    ]);
    let user = block_with(Scope::User, "permissions:\n  file-read: [\"~/notes/*.md\", \"{{NOTES}}/*.md\", \"^*.md\"]\n", env);
    assert_eq!(patterns(&user), ["~/notes/*.md", "{{NOTES}}/*.md", "^*.md"]);
}

#[test]
fn repository_patterns_must_stay_inside_the_repository() {
    let env = HashMap::from([
        (PORTABLE_ENV_VARIABLES.to_string(), "NOTES".to_string()),
        ("NOTES".to_string(), if cfg!(windows) { "C:\\elsewhere".to_string() } else { "/elsewhere".to_string() }),
    ]);
    for (entry, reason) in [
        ("~/notes/*.md", "home directory"),
        ("&../*.md", "leave the repository"),
        ("./../../*.md", "leave the repository"),
        ("{{NOTES}}/*.md", "outside the repository"),
    ] {
        let text = format!("permissions:\n  file-read: [\"{entry}\"]\n");
        let (error, pointer, _) = refused_with(Scope::Repository, &text, env.clone());
        assert_eq!(pointer, "/permissions/file-read/0", "{entry}");
        assert!(error.to_string().contains(reason), "{entry}: {error}");
    }
    let inside = block(Scope::Repository, "permissions:\n  file-read: [\"&docs/../*.md\", \"./*.md\"]\n");
    assert_eq!(patterns(&inside), ["&docs/../*.md", "./*.md"]);
}

// ---- shell-commands: control plus one invalid wiring ---------------------------

#[test]
fn shell_commands_type_each_rule_and_default_to_none() {
    let rules = block(
        Scope::User,
        "permissions:\n  shell-commands:\n    - prefix ls\n    - exact ps -e\n    - template git add {{ string }}\n    - \"!prefix git add secrets\"\n",
    );
    let shell = &rules.shell_commands.value;
    assert!(rules.shell_commands.authored);
    assert_eq!(shell[0].rule, ShellRuleEntry::Prefix("ls".into()));
    assert_eq!(shell[1].rule, ShellRuleEntry::Exact("ps -e".into()));
    assert!(matches!(shell[2].rule, ShellRuleEntry::Template(_)));
    assert!(shell[3].exclude && shell[3].rule == ShellRuleEntry::Prefix("git add secrets".into()));

    let absent = block(Scope::User, "permissions:\n  network: deny\n");
    assert!(absent.shell_commands.value.is_empty() && !absent.shell_commands.authored);
}

#[test]
fn every_invalid_shell_rule_is_a_located_error() {
    for (entry, reason) in [
        ("exact ", "expected 'exact"),
        ("!", "needs a rule"),
        ("!  ", "needs a rule"),
        ("prefix    ", "expected 'exact"),
        ("run ls", "expected 'exact"),
        ("template", "expected 'exact"),
        ("template git {{ nope", ""),
    ] {
        let text = format!("permissions:\n  shell-commands:\n    - prefix ls\n    - \"{entry}\"\n");
        let (error, pointer, line) = refused(Scope::User, &text);
        assert_eq!((pointer.as_str(), line), ("/permissions/shell-commands/1", Some(4)), "{entry}: {error}");
        assert!(error.to_string().contains(reason), "{entry}: {error}");
    }
    let (_, pointer, _) = refused(Scope::User, "permissions:\n  shell-commands: [prefix ls, 3]\n");
    assert_eq!(pointer, "/permissions/shell-commands/1");
    let (_, pointer, _) = refused(Scope::User, "permissions:\n  shell-commands: null\n");
    assert_eq!(pointer, "/permissions/shell-commands");
}

// ---- network --------------------------------------------------------------------

#[test]
fn network_defaults_to_deny_and_refuses_every_other_shape() {
    let absent = block(Scope::User, "permissions: {}\n");
    assert_eq!((absent.network.value, absent.network.authored), (NetworkAccess::Deny, false));
    let allow = block(Scope::User, "permissions:\n  network: allow\n");
    assert_eq!((allow.network.value, allow.network.authored), (NetworkAccess::Allow, true));

    for value in ["null", "\"\"", "true", "[allow]", "maybe", "Allow"] {
        let text = format!("permissions:\n  network: {value}\n");
        let (error, pointer, line) = refused(Scope::User, &text);
        assert_eq!((pointer.as_str(), line), ("/permissions/network", Some(2)), "{value}: {error}");
    }
}

// ---- permissions: mapping or reference ------------------------------------------

#[test]
fn an_empty_permissions_mapping_expands_every_child_default() {
    for text in ["permissions: {}\n", "register: []\n"] {
        let (snapshot, _) = load(Scope::User, text, HashMap::new());
        let effective = &snapshot.user().effective()["permissions"];
        assert_eq!(effective["file-read"].as_array().unwrap().len(), 10, "{text}");
        assert_eq!((&effective["shell-commands"], &effective["network"]), (&serde_json::json!([]), &serde_json::json!("deny")));
        assert!(!snapshot.user().defaults().is_empty());
    }
}

#[test]
fn a_permissions_reference_is_typed_and_anchored_at_its_file() {
    let (snapshot, file) = load(Scope::User, "permissions: ~/.claudine/config.yaml\n", HashMap::new());
    match &snapshot.permissions().unwrap().user {
        PermissionsSetting::Reference(reference) => {
            assert_eq!(reference.reference, "~/.claudine/config.yaml");
            assert_eq!(reference.kind, FileReferenceKind::Home);
            assert_eq!((reference.declared_in.as_path(), reference.base_dir.as_path()), (file.as_path(), file.parent().unwrap()));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_invalid_permissions_value_blocks_permissions_with_a_located_error() {
    for (text, kind, reason) in [
        ("permissions: \"  \"\n", ConfigErrorKind::InvalidReference, "blank"),
        ("permissions: https://example.com/config.yaml\n", ConfigErrorKind::InvalidReference, "remote"),
        ("permissions: null\n", ConfigErrorKind::InvalidSetting, "null"),
        ("permissions: 3\n", ConfigErrorKind::InvalidSetting, "mapping or a file reference"),
        ("permissions: [a]\n", ConfigErrorKind::InvalidSetting, "mapping or a file reference"),
        ("permissions:\n  file-reads: []\n", ConfigErrorKind::InvalidSetting, "`file-reads`"),
    ] {
        let (error, pointer, line) = refused(Scope::User, text);
        assert_eq!(error.kind(), kind, "{text}: {error}");
        assert!(pointer.starts_with("/permissions") && line.is_some(), "{text}: {error}");
        assert!(error.to_string().contains(reason), "{text}: {error}");
        let code = if kind == ConfigErrorKind::InvalidReference { CONFIG_REFERENCE_CODE } else { CONFIG_INVALID_CODE };
        assert_eq!(error.code(), code, "{text}");
    }
    let (error, _, _) = refused(Scope::Repository, "permissions: ~/.claudine/config.yaml\n");
    assert_eq!(error.kind(), ConfigErrorKind::InvalidReference);
    assert!(error.to_string().contains("inside the repository"), "{error}");
}

// ---- register --------------------------------------------------------------------

#[test]
fn register_defaults_to_empty_and_keeps_valid_entries() {
    let (snapshot, file) = load(Scope::User, "register:\n  - ./host.yaml\n  - ~/hosts/claudine.yaml\n", HashMap::new());
    let registrations = snapshot.registrations();
    let references: Vec<_> = registrations.entries.iter().map(|entry| (entry.reference.as_str(), entry.index)).collect();
    assert_eq!(references, [("./host.yaml", 0), ("~/hosts/claudine.yaml", 1)]);
    assert_eq!(registrations.entries[0].declared_in, file);
    assert!(registrations.findings.is_empty());

    let (empty, _) = load(Scope::User, "register: []\n", HashMap::new());
    assert!(empty.registrations().entries.is_empty() && empty.registrations().findings.is_empty());
}

#[test]
fn an_invalid_register_entry_omits_only_that_entry_and_leaves_permissions_usable() {
    let (snapshot, _) = load(
        Scope::User,
        "permissions:\n  network: allow\nregister:\n  - ./a.yaml\n  - 4\n  - \"  \"\n  - https://example.com/m.yaml\n  - ./b.yaml\n",
        HashMap::new(),
    );
    let kept: Vec<_> = snapshot.registrations().entries.iter().map(|entry| entry.index).collect();
    assert_eq!(kept, [0, 4]);
    let findings: Vec<_> = snapshot
        .registrations()
        .findings
        .iter()
        .map(|finding| (finding.problems()[0].pointer.clone(), finding.problems()[0].line))
        .collect();
    assert_eq!(
        findings,
        [("/register/1".to_string(), Some(5)), ("/register/2".to_string(), Some(6)), ("/register/3".to_string(), Some(7))]
    );
    assert!(snapshot.permissions().is_ok(), "a broken registration never blocks permissions");
    assert_eq!(snapshot.diagnostics().len(), 3);
}

#[test]
fn an_invalid_register_list_is_one_finding_and_registers_nothing() {
    for value in ["null", "./host.yaml", "{a: b}", "[1, 2]"] {
        let text = format!("register: {value}\n");
        let (snapshot, _) = load(Scope::User, &text, HashMap::new());
        assert!(snapshot.registrations().entries.is_empty(), "{value}");
        let first = &snapshot.registrations().findings[0].problems()[0];
        assert!(first.pointer.starts_with("/register") && first.line == Some(1), "{value}: {first}");
        assert!(snapshot.permissions().is_ok(), "{value}");
    }
}

#[test]
fn invalid_permissions_never_hide_valid_registrations() {
    let (snapshot, _) = load(Scope::User, "permissions:\n  network: maybe\nregister: [./host.yaml]\n", HashMap::new());
    assert!(snapshot.permissions().is_err());
    assert_eq!(snapshot.registrations().entries.len(), 1);
    assert_eq!(snapshot.diagnostics().len(), 1, "the permissions error is reported");
}
