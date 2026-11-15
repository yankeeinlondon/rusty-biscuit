//! DMLS classifies a shell command passively with the captured
//! `permissions.shell-commands`, the way `md compose` decides it: the
//! built-in blacklist denies; a chain every segment of which a rule
//! pre-approves, with no `!` entry matching, is approved; anything else
//! (no rule, an exclusion, an uncommitted repository allow, invalid
//! permissions) is unknown, because compose would ask. Classifying writes
//! nothing. The retired `shell.policy_path` setting is reported with the
//! location that replaced it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::expression::{DeclaredTypes, TypeScope};
use dmls::overlay::shell::{CapturedPermissions, ParsedCommand, PolicyVerdict, load_permissions, parse_command, verdict};
use serde_json::json;

use crate::common::{LspFixture, LspWorkspace};

/// A home and a working folder under one temporary directory.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        std::fs::create_dir_all(root.join("work")).unwrap();
        Self { _dir: dir, root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    /// Writes `rules` as the `permissions.shell-commands` of `dir/.darkmatter.yaml`.
    fn permissions(&self, dir: &Path, rules: &[&str]) {
        let list: Vec<String> = rules.iter().map(|rule| serde_json::to_string(rule).unwrap()).collect();
        std::fs::write(
            dir.join(".darkmatter.yaml"),
            format!("permissions:\n  shell-commands: [{}]\n", list.join(", ")),
        )
        .unwrap();
    }

    /// The context of a document in `work`, inside the repository there when
    /// `repository` holds.
    fn context(&self, repository: bool) -> FileResolutionContext {
        let context = FileResolutionContext::from_snapshot(self.work(), Some(self.home()), Default::default());
        if repository { context.with_repository_root(self.work()) } else { context }
    }

    fn classify(&self, repository: bool, command: &str) -> PolicyVerdict {
        let permissions = load_permissions(&self.context(repository));
        judge(&parse_command(command).expect("a command"), &permissions)
    }

    /// Every file under the fixture with its bytes.
    fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    out.insert(path.clone(), std::fs::read(&path).unwrap());
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(&self.root, &mut files);
        files
    }

    fn git(&self, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(self.work())
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }
}

/// [`verdict`] at a schema-free, unguarded site: a literal command's types
/// never matter.
fn judge(command: &ParsedCommand, permissions: &CapturedPermissions) -> PolicyVerdict {
    let declared = DeclaredTypes::new(None);
    verdict(command, &TypeScope::unguarded(&declared), permissions)
}

#[test]
fn a_user_allow_approves_and_an_exclusion_leaves_the_command_unknown() {
    let fixture = Fixture::new();
    fixture.permissions(&fixture.home(), &["prefix ls", "prefix git", "!exact git push origin"]);

    assert_eq!(fixture.classify(false, "ls -la"), PolicyVerdict::Approved);
    assert_eq!(fixture.classify(false, "git status && ls"), PolicyVerdict::Approved);
    assert_eq!(fixture.classify(false, "git push origin"), PolicyVerdict::Unknown, "excluded");
    assert_eq!(fixture.classify(false, "cat notes.md"), PolicyVerdict::Unknown, "no rule");
}

#[test]
fn the_builtin_blacklist_denies_whatever_the_permissions_allow() {
    let fixture = Fixture::new();
    fixture.permissions(&fixture.home(), &["prefix rm", "prefix git"]);

    assert!(matches!(fixture.classify(false, "rm -rf build"), PolicyVerdict::Denied(_)));
    assert!(matches!(fixture.classify(false, "git status && git reset --hard"), PolicyVerdict::Denied(_)));
}

#[test]
fn a_repository_allow_counts_only_once_committed() {
    let fixture = Fixture::new();
    fixture.permissions(&fixture.work(), &["prefix ls"]);
    fixture.git(&["init", "--quiet"]);
    fixture.git(&["add", "-A"]);
    fixture.git(&["commit", "--quiet", "-m", "permissions"]);
    // Widened in the working copy only: not effective until committed.
    fixture.permissions(&fixture.work(), &["prefix ls", "prefix cat"]);

    assert_eq!(fixture.classify(true, "ls"), PolicyVerdict::Approved, "HEAD and the working copy allow it");
    assert_eq!(fixture.classify(true, "cat notes.md"), PolicyVerdict::Unknown, "only the working copy allows it");
}

#[test]
fn invalid_permissions_never_approve() {
    let fixture = Fixture::new();
    std::fs::write(fixture.home().join(".darkmatter.yaml"), "permissions:\n  shell-commands:\n    - frobnicate ls\n")
        .unwrap();

    let permissions = load_permissions(&fixture.context(false));

    assert!(permissions.is_err(), "the configuration error is kept");
    assert_eq!(judge(&parse_command("ls").unwrap(), &permissions), PolicyVerdict::Unknown);
    assert!(matches!(judge(&parse_command("rm x").unwrap(), &permissions), PolicyVerdict::Denied(_)));
}

#[test]
fn classifying_writes_nothing() {
    let fixture = Fixture::new();
    fixture.permissions(&fixture.home(), &["prefix ls"]);
    let before = fixture.files();

    for command in ["ls", "cat x", "rm x"] {
        fixture.classify(false, command);
    }

    assert_eq!(fixture.files(), before);
    assert!(!fixture.work().join(".darkmatter.yaml").exists());
}

/// The decision `md`'s compose pre-flight reaches for `document`, composed as
/// `dir/doc.md` with `home` as its home: pre-approved, approval required, or
/// refused by the built-in blacklist.
fn md_decision(dir: &Path, home: &Path, document: &str) -> &'static str {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot, ShellExpansionError};

    let path = dir.join("doc.md");
    std::fs::write(&path, document).unwrap();
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let snapshot = RequestSnapshot::new(dir).with_home(Some(home.to_path_buf()));
    let request = ComposeRequest::prepare(ComposeOptions::new().with_source_file(&path), &snapshot).expect("a request");
    let decision = match markdown.compose_preflight_approvals(&request, None) {
        Ok(_) => "preapproved",
        Err(ShellExpansionError::ApprovalRequired { .. }) => "approval-required",
        Err(ShellExpansionError::Blacklisted { .. }) => "refused",
        Err(other) => panic!("md: {other}"),
    };
    std::fs::remove_file(&path).unwrap();
    decision
}

/// DMLS's decision for each document of `documents`, read from the policy
/// line of a real LSP hover on its first shell command (a `::shell` line or
/// the first line of a `::shell-block` body), with `config` as the
/// `~/.darkmatter.yaml` of the session's home; and `md`'s decision for the
/// same document with the same home.
fn hover_and_md_decisions(config: &str, documents: &[&str]) -> Vec<(&'static str, &'static str)> {
    let workspace = LspWorkspace::new();
    let home = workspace.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join(".darkmatter.yaml"), config).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    let root = url::Url::from_directory_path(workspace.path()).unwrap();
    fixture.initialize(json!({
        "processId": null,
        "capabilities": {},
        "workspaceFolders": [{ "uri": root.as_str(), "name": "scratch" }],
    }));
    let decisions = documents
        .iter()
        .enumerate()
        .map(|(index, document)| {
            let path = workspace.path().join(format!("hover-{index}.md"));
            std::fs::write(&path, document).unwrap();
            let uri = url::Url::from_file_path(&path).unwrap();
            fixture.notify(
                "textDocument/didOpen",
                json!({ "textDocument": { "uri": uri.as_str(), "languageId": "markdown", "version": 1, "text": document } }),
            );
            let lines: Vec<&str> = document.lines().collect();
            let line = match lines.iter().position(|line| line.starts_with("::shell ")) {
                Some(line) => line,
                None => lines.iter().position(|line| *line == "::shell-block").expect("a shell command") + 1,
            };
            let response = fixture.request(
                "textDocument/hover",
                json!({ "textDocument": { "uri": uri.as_str() }, "position": { "line": line, "character": 1 } }),
            );
            let text = response.result.expect("a hover")["contents"]["value"].as_str().unwrap_or_default().to_string();
            let policy = text.split("Policy: ").nth(1).unwrap_or_else(|| panic!("no policy line: {text}"));
            let dmls = if policy.contains("approved") {
                "preapproved"
            } else if policy.contains("denied") {
                "refused"
            } else {
                assert!(policy.contains("unknown"), "{policy}");
                "approval-required"
            };
            (dmls, md_decision(workspace.path(), &home, document))
        })
        .collect();
    fixture.shutdown();
    decisions
}

/// The shared permission parity fixture
/// (`darkmatter/lib/tests/fixtures/permission_parity/cases.json`): for every
/// command row, literal or a whole document, DMLS's passive verdict (the
/// policy line of its hover) is the decision `md`'s compose pre-flight
/// reaches (approved is pre-approved, unknown is approval required, denied
/// is the built-in refusal), and both are the row's. Classifying the literal
/// rows writes nothing.
#[test]
fn every_command_row_is_classified_as_md_decides_it() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../lib/tests/fixtures/permission_parity/cases.json")).expect("cases");
    let config = format!("{}  network: deny\n", cases["user_permissions"].as_str().expect("user_permissions"));
    let rows = cases["commands"].as_array().expect("commands");
    assert!(rows.iter().any(|row| row["document"].is_string()), "the table keeps document rows");
    let documents: Vec<String> = rows
        .iter()
        .map(|row| match row["command"].as_str() {
            Some(command) => format!("::shell {command}\n"),
            None => row["document"].as_str().expect("a command or a document").to_string(),
        })
        .collect();

    let decisions = hover_and_md_decisions(&config, &documents.iter().map(String::as_str).collect::<Vec<_>>());

    for (row, decision) in rows.iter().zip(decisions) {
        let expected = row["decision"].as_str().unwrap();
        assert_eq!(decision, (expected, expected), "(dmls, md): {}", row["name"]);
    }

    let fixture = Fixture::new();
    std::fs::write(fixture.home().join(".darkmatter.yaml"), &config).unwrap();
    let before = fixture.files();
    for row in rows.iter().filter_map(|row| row["command"].as_str()) {
        fixture.classify(false, row);
    }
    assert_eq!(fixture.files(), before, "classifying writes nothing");
}

/// The `~/.darkmatter.yaml` whose `permissions.shell-commands` are `rules`.
fn shell_rules(rules: &[&str]) -> String {
    let list: Vec<String> = rules.iter().map(|rule| serde_json::to_string(rule).unwrap()).collect();
    format!("permissions:\n  shell-commands: [{}]\n", list.join(", "))
}

/// A document declaring a string `title` whose body is `body`.
fn titled(body: &str) -> String {
    format!("---\n$schema:\n  title: string\ntitle: hello\n---\n{body}\n")
}

/// Every rule kind (`exact`, `prefix`, `template`, and a `!` entry of each),
/// over a literal command, a placeholder's declared shape, a chain, and a
/// `::shell-block` line: DMLS's hover verdict is `md`'s pre-flight decision.
#[test]
fn every_rule_kind_is_classified_as_md_decides_it() {
    let rows: [(&str, &[&str], String, &str); 14] = [
        ("a template allow covers a declared string", &["template echo {{ string }}"], titled("::shell echo {{ title }}"), "preapproved"),
        ("a template allow never covers a literal", &["template echo {{ string }}"], titled("::shell echo hi"), "approval-required"),
        ("an exact allow", &["exact echo hi"], titled("::shell echo hi"), "preapproved"),
        ("a prefix allow", &["prefix echo"], titled("::shell echo hi"), "preapproved"),
        ("a template exclusion over a prefix allow, literal", &["prefix echo", "!template echo {{ string }}"], titled("::shell echo hi"), "approval-required"),
        ("a template exclusion over a prefix allow, by shape", &["prefix echo", "!template echo {{ string }}"], titled("::shell echo {{ title }}"), "approval-required"),
        ("a prefix exclusion over a template allow", &["template echo {{ string }}", "!prefix echo"], titled("::shell echo {{ title }}"), "approval-required"),
        ("an exact exclusion of the value over a template allow", &["template echo {{ string }}", "!exact echo hello"], titled("::shell echo {{ title }}"), "approval-required"),
        ("a template allow on a shell-block line", &["template echo {{ string }}"], titled("::shell-block\necho {{ title }}\n::end-block"), "preapproved"),
        ("a chain template allow", &["template ls && echo {{ string }}"], titled("::shell ls && echo {{ title }}"), "preapproved"),
        ("a chain with a segment no rule covers", &["template echo {{ string }}", "prefix ls"], titled("::shell ls && echo {{ title }}"), "approval-required"),
        ("a prefix allow for every program of a chain", &["prefix ls", "prefix echo"], titled("::shell ls && echo {{ title }}"), "preapproved"),
        ("a chain template exclusion, literal", &["prefix ls", "prefix echo", "!template ls && echo {{ string }}"], titled("::shell ls && echo hi"), "approval-required"),
        ("the built-in blacklist over a template allow", &["template chmod 600 {{ string }}"], titled("::shell chmod 600 {{ title }}"), "refused"),
    ];

    for (name, rules, document, expected) in &rows {
        let [decision] = hover_and_md_decisions(&shell_rules(rules), &[document.as_str()])[..] else { unreachable!() };
        assert_eq!(decision, (*expected, *expected), "(dmls, md): {name}");
    }
}

#[test]
fn a_hover_reports_the_captured_permission_verdict() {
    const DOC: &str = "# Shell\n\n::shell-block\nls -la\ncat notes.md\n::end-block\n";
    let workspace = LspWorkspace::new();
    std::fs::create_dir_all(workspace.path().join("home")).unwrap();
    std::fs::write(workspace.path().join("home/.darkmatter.yaml"), "permissions:\n  shell-commands: [\"prefix ls\"]\n")
        .unwrap();
    std::fs::write(workspace.path().join("doc.md"), DOC).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    let root = url::Url::from_directory_path(workspace.path()).unwrap();
    fixture.initialize(json!({
        "processId": null,
        "capabilities": {},
        "workspaceFolders": [{ "uri": root.as_str(), "name": "scratch" }],
    }));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri.as_str(), "languageId": "markdown", "version": 1, "text": DOC } }),
    );
    let mut hover = |line: u32| {
        let response = fixture.request(
            "textDocument/hover",
            json!({ "textDocument": { "uri": uri.as_str() }, "position": { "line": line, "character": 1 } }),
        );
        response.result.expect("a hover")["contents"]["value"].as_str().unwrap_or_default().to_string()
    };

    let allowed = hover(3);
    let asked = hover(4);

    assert!(allowed.contains("approved"), "{allowed}");
    assert!(asked.contains("unknown"), "{asked}");
    fixture.shutdown();
}

#[test]
fn a_retired_shell_policy_path_is_reported_with_its_replacement() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join(".dmls.toml"), "[shell]\npolicy_path = \"/policies\"\n").unwrap();

    let state = dmls::ConfigState::load(vec![workspace.path().to_path_buf()], None);

    let [finding] = state.diagnostics() else { panic!("{:?}", state.diagnostics()) };
    assert_eq!(finding.severity, dmls::config::ConfigSeverity::Warning);
    assert_eq!(finding.key.as_deref(), Some("shell.policy_path"));
    let message = finding.to_string();
    assert!(message.contains("permissions.shell-commands"), "{message}");
    assert!(message.contains("~/.darkmatter.yaml") && message.contains("{repo}/.darkmatter.yaml"), "{message}");
}
