//! The `md` shell approval prompt answered with scripted input, driving the
//! library's real decision path (`compose_preflight_approvals`): what each
//! listed choice saves to the policy files, that a saved rule answers the
//! next run without a prompt, that a failed write approves nothing, and that
//! without a terminal no prompt is shown.

use crate::common::CliProcessFixture;
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

fn read(dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(name)).ok()
}

const WHITELIST: &str = ".darkmatter-shell-whitelist";
const BLACKLIST: &str = ".darkmatter-shell-blacklist";

/// A command whose value is known when it is approved.
const RESOLVED: &str = "---\nword: hello\n---\n::shell git --version {{ word }}\n";
/// A command whose value the run produces later (`outputs` declared late).
const LATE: &str = "::shell git --version {{ last(outputs) }}\n";

fn request(dir: &Path, path: &Path) -> ComposeRequest {
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_shell_policy_root(dir)
        .with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]));
    ComposeRequest::prepare(options, &RequestSnapshot::new(dir)).expect("request")
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

/// One prompt answer and what it must leave behind: the whitelist and
/// blacklist contents, and whether the command is approved by its bytes and
/// by its shape.
type Row = (&'static str, Option<&'static str>, Option<&'static str>, bool, bool);

fn shape(rule: &str) -> TemplateShape {
    TemplateShape::parse_rule(rule).unwrap()
}

#[test]
fn each_choice_for_a_resolved_command_saves_its_row() {
    let cases: [Row; 6] = [
        ("1", Some("exact git --version hello\n"), None, true, false),
        ("2", Some("template git --version {{ string }}\n"), None, false, true),
        ("3", Some("prefix git\n"), None, true, false),
        ("4", None, None, true, false),
        ("5", None, None, false, false),
        ("6", None, Some("exact git --version hello\n"), false, false),
    ];
    for (answer, whitelist, blacklist, by_bytes, by_shape) in cases {
        let dir = TempDir::new().unwrap();
        let session = PromptSession::new(&[answer]);

        let result = approve(dir.path(), RESOLVED, &session);

        let transcript = session.transcript();
        assert!(transcript.contains("Command:  git --version {{ word: string }}"), "{transcript}");
        assert!(transcript.contains("Resolved: git --version hello"), "{transcript}");
        assert!(transcript.contains("Allow this exact command"), "{transcript}");
        assert_eq!(read(dir.path(), WHITELIST).as_deref(), whitelist, "answer {answer}");
        assert_eq!(read(dir.path(), BLACKLIST).as_deref(), blacklist, "answer {answer}");
        match answer {
            "5" => assert!(matches!(result, Err(ShellExpansionError::Denied { .. })), "{result:?}"),
            "6" => assert!(matches!(result, Err(ShellExpansionError::Blacklisted { .. })), "{result:?}"),
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
    let cases: [Row; 5] = [
        ("1", Some("template git --version {{ string }}\n"), None, false, true),
        ("2", Some("prefix git\n"), None, false, true),
        ("3", None, None, false, true),
        ("4", None, None, false, false),
        ("5", None, Some("template git --version {{ string }}\n"), false, false),
    ];
    for (answer, whitelist, blacklist, by_bytes, by_shape) in cases {
        let dir = TempDir::new().unwrap();
        let session = PromptSession::new(&[answer]);

        let result = approve(dir.path(), LATE, &session);

        let transcript = session.transcript();
        assert!(transcript.contains("Command:  git --version {{ last(outputs): string }}"), "{transcript}");
        assert!(!transcript.contains("exact command"), "{transcript}");
        assert!(!transcript.contains("Resolved:"), "{transcript}");
        assert_eq!(read(dir.path(), WHITELIST).as_deref(), whitelist, "answer {answer}");
        assert_eq!(read(dir.path(), BLACKLIST).as_deref(), blacklist, "answer {answer}");
        match answer {
            "4" => assert!(matches!(result, Err(ShellExpansionError::Denied { .. })), "{result:?}"),
            "5" => assert!(matches!(result, Err(ShellExpansionError::Blacklisted { .. })), "{result:?}"),
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
    assert_eq!(
        read(dir.path(), WHITELIST).as_deref(),
        Some("template just clean --out {{ string }}/build\n")
    );
}

#[test]
fn a_numeric_declaration_is_disclosed_and_saved_as_a_string_hole() {
    let dir = TempDir::new().unwrap();
    let session = PromptSession::new(&["2"]);

    approve(dir.path(), "---\n$schema:\n  count: number\ncount: 2\n---\n::shell seq {{ count }}\n", &session)
        .unwrap();

    let transcript = session.transcript();
    assert!(transcript.contains("seq {{ count: string (validated as number) }}"), "{transcript}");
    assert_eq!(read(dir.path(), WHITELIST).as_deref(), Some("template seq {{ string }}\n"));
}

#[test]
fn an_unlisted_number_is_refused_until_a_listed_choice_is_given() {
    let dir = TempDir::new().unwrap();
    // The late command lists five choices; "6" is refused, then "4" denies.
    let session = PromptSession::new(&["6\n4"]);

    let result = approve(dir.path(), LATE, &session);

    assert!(matches!(result, Err(ShellExpansionError::Denied { .. })), "{result:?}");
    assert!(session.transcript().contains("Invalid choice. Please enter 1-5."));
    assert_eq!(read(dir.path(), WHITELIST), None);
    assert_eq!(read(dir.path(), BLACKLIST), None);
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
    assert!(transcript.contains("saves \"prefix git\", \"prefix just\" to the whitelist"), "{transcript}");
    assert_eq!(read(dir.path(), WHITELIST).as_deref(), Some("prefix git\nprefix just\n"));
}

#[test]
fn a_saved_rule_answers_the_next_run_without_a_prompt_and_is_not_written_twice() {
    for (content, answer, saved) in [
        (RESOLVED, "1", "exact git --version hello\n"),
        (RESOLVED, "2", "template git --version {{ string }}\n"),
        (LATE, "1", "template git --version {{ string }}\n"),
        (LATE, "2", "prefix git\n"),
    ] {
        let dir = TempDir::new().unwrap();
        approve(dir.path(), content, &PromptSession::new(&[answer])).unwrap();
        assert_eq!(read(dir.path(), WHITELIST).as_deref(), Some(saved));

        for _ in 0..2 {
            let silent = PromptSession::new(&[]);
            let approvals = approve(dir.path(), content, &silent).unwrap();
            assert_eq!(silent.prompts(), 0, "{content:?} answer {answer}");
            assert_eq!(approvals.stats.already_whitelisted, 1, "{content:?} answer {answer}");
            assert_eq!(read(dir.path(), WHITELIST).as_deref(), Some(saved), "{content:?} answer {answer}");
        }
    }
}

#[test]
fn a_failed_write_is_a_policy_failure_and_approves_nothing() {
    for (content, answer) in [(RESOLVED, "1"), (RESOLVED, "2"), (LATE, "1"), (LATE, "2"), (LATE, "5")] {
        let dir = TempDir::new().unwrap();
        let whitelist = dir.path().join(WHITELIST);
        let blacklist = dir.path().join(BLACKLIST);
        // The policy files become directories while the user is answering,
        // so the rules load but the append after the answer fails.
        let session = Arc::new(PromptSession {
            before_answer: Some(Box::new(move || {
                std::fs::create_dir_all(&whitelist).unwrap();
                std::fs::create_dir_all(&blacklist).unwrap();
            })),
            ..Arc::into_inner(PromptSession::new(&[answer])).unwrap()
        });

        let error = approve(dir.path(), content, &session).expect_err("the write fails");

        assert!(matches!(error, ShellExpansionError::PolicyIo { .. }), "{content:?} {answer}: {error:?}");
        assert_eq!(session.prompts(), 1);
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
    assert!(!fixture.cwd().join(WHITELIST).exists());
}
