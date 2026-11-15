//! The `md` shell approval prompt answered with scripted input, driving the
//! library's real decision path (`compose_preflight_approvals`): what each
//! listed choice saves to the user's `permissions.shell-commands`, that a
//! saved rule answers the next run without a prompt, that a rule an
//! exclusion still matches approves the run but is not saved, and that
//! without a terminal no prompt is shown.

use crate::common::CliProcessFixture;
use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::preflight::ComposePreflightApprovals;
use darkmatter::markdown::compose::shell_expansion::template::{DeferredShellRoots, TemplateShape};
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};
use darkmatter_cli::approval::approve_with_io;
use predicates::prelude::*;
use std::collections::VecDeque;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// The installed prompt, fed one scripted line per request. It records what
/// the user would have seen; a request with no line left fails the test.
struct PromptSession {
    answers: Mutex<VecDeque<String>>,
    transcript: Mutex<String>,
    /// Runs before the answer is read, to change the world mid-prompt.
    before_answer: Option<Box<dyn Fn() + Send + Sync>>,
}

impl PromptSession {
    fn new(answers: &[&str]) -> Arc<Self> {
        Arc::new(Self {
            answers: Mutex::new(answers.iter().map(|answer| format!("{answer}\n")).collect()),
            transcript: Mutex::new(String::new()),
            before_answer: None,
        })
    }

    fn transcript(&self) -> String {
        strip_ansi(&self.transcript.lock().unwrap())
    }

    fn prompts(&self) -> usize {
        self.transcript().matches("Shell Approval Required").count()
    }
}

impl ShellApprovalHandler for PromptSession {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        let answer = self
            .answers
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| panic!("unexpected prompt for {}", request.raw_command));
        if let Some(before_answer) = &self.before_answer {
            before_answer();
        }
        let mut output = Vec::new();
        let decision = approve_with_io(&request, &mut Cursor::new(answer.into_bytes()), &mut output);
        self.transcript
            .lock()
            .unwrap()
            .push_str(&String::from_utf8(output).unwrap());
        decision
    }
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for inner in chars.by_ref() {
                if inner == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// The user's `shell-commands` entries under the home `dir`, one per line,
/// or `None` when there are none.
fn saved(dir: &Path) -> Option<String> {
    let snapshot =
        ConfigSnapshot::load_user(&RequestSnapshot::new(dir).with_home(Some(dir.to_path_buf())), ConfigLoadMode::Passive)
            .unwrap();
    let policy = snapshot.policy().unwrap();
    let entries = &policy.user().permissions().shell_commands.value;
    (!entries.is_empty()).then(|| entries.iter().map(|entry| format!("{}\n", entry.authored)).collect())
}

/// A command whose value is known when it is approved.
const RESOLVED: &str = "---\nword: hello\n---\n::shell git --version {{ word }}\n";
/// A command whose value the run produces later (`outputs` declared late).
const LATE: &str = "::shell git --version {{ last(outputs) }}\n";

fn request(dir: &Path, path: &Path) -> ComposeRequest {
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]));
    // The document's folder is the home, so a saved rule lands beside it.
    ComposeRequest::prepare(options, &RequestSnapshot::new(dir).with_home(Some(dir.to_path_buf()))).expect("request")
}

fn approve(
    dir: &Path,
    content: &str,
    session: &Arc<PromptSession>,
) -> Result<ComposePreflightApprovals, ShellExpansionError> {
    let path = write(dir, "doc.md", content);
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_preflight_approvals(&request(dir, &path), Some(Arc::clone(session) as Arc<dyn ShellApprovalHandler>))
}

/// One prompt answer and what it must leave behind: the saved
/// `shell-commands`, and whether the command is approved by its bytes and
/// by its shape.
type Row = (&'static str, Option<&'static str>, bool, bool);

fn shape(rule: &str) -> TemplateShape {
    TemplateShape::parse_rule(rule).unwrap()
}

#[test]
fn each_choice_for_a_resolved_command_saves_its_row() {
    let cases: [Row; 5] = [
        ("1", Some("exact git --version hello\n"), true, false),
        ("2", Some("template git --version {{ string }}\n"), false, true),
        ("3", Some("prefix git\n"), true, false),
        ("4", None, true, false),
        ("5", None, false, false),
    ];
    for (answer, rules, by_bytes, by_shape) in cases {
        let dir = TempDir::new().unwrap();
        let session = PromptSession::new(&[answer]);

        let result = approve(dir.path(), RESOLVED, &session);

        let transcript = session.transcript();
        assert!(transcript.contains("Command:  git --version {{ word: string }}"), "{transcript}");
        assert!(transcript.contains("Resolved: git --version hello"), "{transcript}");
        assert!(transcript.contains("Allow this exact command"), "{transcript}");
        assert_eq!(saved(dir.path()).as_deref(), rules, "answer {answer}");
        match answer {
            "5" => assert!(matches!(result, Err(ShellExpansionError::Denied { .. })), "{result:?}"),
            _ => {
                let approvals = result.unwrap().pre_approved_commands;
                assert_eq!(approvals.contains_exact("git --version hello"), by_bytes, "answer {answer}");
                assert_eq!(
                    approvals.contains_template(&shape("git --version {{ string }}")),
                    by_shape,
                    "answer {answer}"
                );
            }
        }
    }
}

#[test]
fn each_choice_for_a_late_command_saves_its_row() {
    // A late command has no bytes, so none is ever approved by them.
    let cases: [Row; 4] = [
        ("1", Some("template git --version {{ string }}\n"), false, true),
        ("2", Some("prefix git\n"), false, true),
        ("3", None, false, true),
        ("4", None, false, false),
    ];
    for (answer, rules, by_bytes, by_shape) in cases {
        let dir = TempDir::new().unwrap();
        let session = PromptSession::new(&[answer]);

        let result = approve(dir.path(), LATE, &session);

        let transcript = session.transcript();
        assert!(transcript.contains("Command:  git --version {{ last(outputs): string }}"), "{transcript}");
        assert!(!transcript.contains("exact command"), "{transcript}");
        assert!(!transcript.contains("Resolved:"), "{transcript}");
        assert_eq!(saved(dir.path()).as_deref(), rules, "answer {answer}");
        match answer {
            "4" => assert!(matches!(result, Err(ShellExpansionError::Denied { .. })), "{result:?}"),
            _ => {
                let approvals = result.unwrap().pre_approved_commands;
                assert_eq!(approvals.exact().count() > 0, by_bytes, "answer {answer}");
                assert_eq!(
                    approvals.contains_template(&shape("git --version {{ string }}")),
                    by_shape,
                    "answer {answer}"
                );
            }
        }
    }
}

#[test]
fn a_single_document_command_offers_the_exact_bytes_and_saves_the_shape() {
    let dir = TempDir::new().unwrap();
    let session = PromptSession::new(&["2"]);

    approve(dir.path(), "---\nout: dist\n---\n::shell just clean --out {{ out }}/build\n", &session).unwrap();

    let transcript = session.transcript();
    assert!(transcript.contains("Command:  just clean --out {{ out: string }}/build"), "{transcript}");
    assert!(transcript.contains("Resolved: just clean --out dist/build"), "{transcript}");
    assert!(transcript.contains("1 │ Allow this exact command"), "{transcript}");
    assert_eq!(saved(dir.path()).as_deref(), Some("template just clean --out {{ string }}/build\n"));
}

#[test]
fn a_numeric_declaration_is_disclosed_and_saved_as_a_string_hole() {
    let dir = TempDir::new().unwrap();
    let session = PromptSession::new(&["2"]);

    approve(dir.path(), "---\n$schema:\n  count: number\ncount: 2\n---\n::shell seq {{ count }}\n", &session)
        .unwrap();

    let transcript = session.transcript();
    assert!(transcript.contains("seq {{ count: string (validated as number) }}"), "{transcript}");
    assert_eq!(saved(dir.path()).as_deref(), Some("template seq {{ string }}\n"));
}

#[test]
fn an_unlisted_number_is_refused_until_a_listed_choice_is_given() {
    let dir = TempDir::new().unwrap();
    // The late command lists four choices; "5" is refused, then "4" denies.
    let session = PromptSession::new(&["5\n4"]);

    let result = approve(dir.path(), LATE, &session);

    assert!(matches!(result, Err(ShellExpansionError::Denied { .. })), "{result:?}");
    assert!(session.transcript().contains("Invalid choice. Please enter 1-4."));
    assert_eq!(saved(dir.path()), None);
}

#[test]
fn a_chain_lists_and_saves_a_prefix_rule_for_every_program() {
    let dir = TempDir::new().unwrap();
    let session = PromptSession::new(&["2"]);

    approve(
        dir.path(),
        "::shell git --version {{ last(outputs) }} && just build {{ last(outputs) }} && git status\n",
        &session,
    )
    .unwrap();

    let transcript = session.transcript();
    assert!(transcript.contains("saves \"prefix git\", \"prefix just\" to your shell-commands"), "{transcript}");
    assert_eq!(saved(dir.path()).as_deref(), Some("prefix git\nprefix just\n"));
}

#[test]
fn a_saved_rule_answers_the_next_run_without_a_prompt_and_is_not_written_twice() {
    for (content, answer, rule) in [
        (RESOLVED, "1", "exact git --version hello\n"),
        (RESOLVED, "2", "template git --version {{ string }}\n"),
        (LATE, "1", "template git --version {{ string }}\n"),
        (LATE, "2", "prefix git\n"),
    ] {
        let dir = TempDir::new().unwrap();
        approve(dir.path(), content, &PromptSession::new(&[answer])).unwrap();
        assert_eq!(saved(dir.path()).as_deref(), Some(rule));

        for _ in 0..2 {
            let silent = PromptSession::new(&[]);
            let approvals = approve(dir.path(), content, &silent).unwrap();
            assert_eq!(silent.prompts(), 0, "{content:?} answer {answer}");
            assert_eq!(approvals.stats.already_whitelisted, 1, "{content:?} answer {answer}");
            assert_eq!(saved(dir.path()).as_deref(), Some(rule), "{content:?} answer {answer}");
        }
    }
}

#[test]
fn an_always_choice_an_exclusion_still_matches_approves_the_run_and_is_not_saved() {
    for (content, answer) in [(RESOLVED, "1"), (RESOLVED, "2"), (RESOLVED, "3"), (LATE, "1"), (LATE, "2")] {
        let dir = TempDir::new().unwrap();
        write(dir.path(), ".darkmatter.yaml", "permissions:\n  shell-commands:\n    - \"!prefix git\"\n");
        let before = std::fs::read(dir.path().join(".darkmatter.yaml")).unwrap();
        let session = PromptSession::new(&[answer]);

        let approvals = approve(dir.path(), content, &session).expect("the run's approval stands");

        assert!(session.transcript().contains("Excluded: !prefix git"), "{}", session.transcript());
        assert!(!approvals.pre_approved_commands.is_empty(), "{content:?} {answer}");
        let not_saved = approvals.warnings.iter().filter(|warning| warning.code.as_deref() == Some("dm.config.not_saved"));
        assert_eq!(not_saved.count(), 1, "{content:?} {answer}: {:?}", approvals.warnings);
        assert_eq!(std::fs::read(dir.path().join(".darkmatter.yaml")).unwrap(), before, "{content:?} {answer}");
    }
}

#[test]
fn without_a_terminal_a_command_shape_is_not_prompted_for() {
    let fixture = CliProcessFixture::named("approval_prompt_without_a_terminal");
    let md_path = fixture.write_file("cwd/doc.md", "---\nout: dist\n---\n::shell just clean --out {{ out }}/build\n");

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Approval required for"))
        .stderr(predicate::str::contains("Shell Approval Required").not());
    assert_eq!(
        std::fs::read_to_string(fixture.home().join(".darkmatter.yaml")).unwrap(),
        crate::common::fixture::USER_CONFIG,
        "nothing was saved"
    );
}
