//! A recovery wrapper never turns an unattempted read into a settled result.
//!
//! `try_frontmatter`, the lazy `try`, and the document readers that share
//! `frontmatter`'s load (`frontmatter`, `markdown_title`,
//! `markdown_body_empty`, `validate_schema`) each guard a `::block`. In a dry
//! run, a read that needs network approval (`network: ask`) or reads a file
//! the launch creates (`PassiveSession::defer_created`) is left unchecked:
//! the block stays as written with one deferral and no request. Under
//! `allow` the read happens once, and under `deny`, or for an ordinary
//! missing file, each wrapper keeps the settled outcome a real run has.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::passive::{DeferralReason, DeferredConstruct, PassiveSession};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};

use crate::network_approval::Responder;

/// How the guarded block settles when the read is attempted and fails.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Settled {
    /// The wrapper turns the failure into data: the block is left out.
    Omitted,
    /// The failure stops the composition.
    Refused,
}

/// Each reader as a block condition over `{}` (the reference), with how a
/// remote denial and an ordinary local miss settle.
const READERS: &[(&str, Settled, Settled)] = &[
    ("try_frontmatter('{}').ok", Settled::Omitted, Settled::Omitted),
    ("frontmatter('{}').title", Settled::Refused, Settled::Refused),
    // A denial is not a recoverable read failure; a clean miss is.
    ("try(frontmatter('{}'), {}).title", Settled::Refused, Settled::Omitted),
    ("markdown_title('{}')", Settled::Refused, Settled::Refused),
    ("markdown_body_empty('{}') == false", Settled::Refused, Settled::Refused),
    ("validate_schema('{}')", Settled::Refused, Settled::Refused),
];

fn guarded(condition: &str, reference: &str) -> String {
    let condition = condition.replacen("{}", reference, 1);
    format!("Before.\n\n::block when=\"{condition}\"\nGuarded.\n::end-block\n")
}

fn has_block(body: &str, condition: &str, reference: &str) -> bool {
    body.contains(&format!("::block when=\"{}\"", condition.replacen("{}", reference, 1)))
}

/// A home whose `.darkmatter.yaml` sets `network`, and a work folder.
struct Home {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Home {
    fn new(network: &str) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("home")).expect("home");
        std::fs::create_dir_all(root.join("work")).expect("work");
        std::fs::write(root.join("home/.darkmatter.yaml"), format!("permissions:\n  network: {network}\n")).expect("config");
        Self { _dir: dir, root }
    }

    fn compose(&self, text: &str, dry_run: bool) -> (Result<String, String>, Option<PassiveSession>) {
        let path = self.root.join("work/doc.md");
        std::fs::write(&path, text).expect("write");
        let snapshot = RequestSnapshot::new(self.root.join("work")).with_home(Some(self.root.join("home")));
        let options = ComposeOptions::new().with_source_file(&path).with_allow_remote_transclusion(true);
        let options = if dry_run { options.with_dry_run() } else { options };
        let request = ComposeRequest::prepare(options, &snapshot).expect("a request");
        let session = request.options().passive_session().cloned();
        let outcome = Markdown::try_from(path.as_path())
            .expect("markdown")
            .compose_with(&request)
            .map(|(composed, _)| composed.content().to_string())
            .map_err(|error| error.to_string());
        (outcome, session)
    }
}

fn assert_settled(outcome: &Result<String, String>, settled: Settled, cell: &str) {
    match settled {
        Settled::Omitted => {
            let body = outcome.as_ref().unwrap_or_else(|error| panic!("{cell}: {error}"));
            assert!(body.contains("Before.") && !body.contains("Guarded.") && !body.contains("::block"), "{cell}: {body}");
        }
        Settled::Refused => assert!(outcome.is_err(), "{cell}: {outcome:?}"),
    }
}

#[test]
fn a_remote_read_under_ask_is_unchecked_through_every_wrapper_and_allow_and_deny_settle() {
    for &(condition, on_deny, _) in READERS {
        for (setting, dry_run) in [("ask", true), ("allow", true), ("deny", true), ("deny", false)] {
            let server = Responder::start();
            let home = Home::new(setting);
            let url = server.url("/facts.md");
            let cell = format!("{condition}, {setting}, dry run {dry_run}");

            let (outcome, session) = home.compose(&guarded(condition, &url), dry_run);

            match setting {
                "ask" => {
                    let body = outcome.as_ref().unwrap_or_else(|error| panic!("{cell}: {error}"));
                    assert!(has_block(body, condition, &url), "{cell}: {body}");
                    let deferrals = session.expect("a dry run").deferrals();
                    assert_eq!(deferrals.len(), 1, "{cell}: {deferrals:?}");
                    assert_eq!(deferrals[0].construct, DeferredConstruct::BlockCondition, "{cell}");
                    assert_eq!(deferrals[0].reason, DeferralReason::NetworkNeedsApproval { url: url.clone() }, "{cell}");
                    assert_eq!(server.requests(), 0, "{cell}");
                }
                "allow" => {
                    let body = outcome.as_ref().unwrap_or_else(|error| panic!("{cell}: {error}"));
                    assert!(body.contains("Guarded.") && !body.contains("::block"), "{cell}: {body}");
                    assert!(session.expect("a dry run").deferrals().is_empty(), "{cell}");
                    assert_eq!(server.requests(), 1, "{cell}");
                }
                _ => {
                    assert_settled(&outcome, on_deny, &cell);
                    assert_eq!(server.requests(), 0, "{cell}");
                }
            }
        }
    }
}

fn dry_compose_created(dir: &Path, text: &str, declared: &str) -> (Result<String, String>, PassiveSession) {
    let path = dir.join("doc.md");
    std::fs::write(&path, text).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone())
        .with_dry_run();
    let session = options.passive_session().cloned().unwrap();
    session.defer_created(&dir.join(declared), DeferralReason::CreatedAtLaunch { path: declared.to_string() });
    let composed = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed.content().to_string())
        .map_err(|error| error.to_string());
    (composed, session)
}

#[test]
fn a_file_the_launch_creates_is_unchecked_through_every_wrapper_and_an_ordinary_miss_settles() {
    for &(condition, _, on_miss) in READERS {
        let dir = tempfile::tempdir().unwrap();
        let reference = "gen/facts.md";

        let (outcome, session) = dry_compose_created(dir.path(), &guarded(condition, reference), reference);
        let body = outcome.as_ref().unwrap_or_else(|error| panic!("{condition}, created: {error}"));
        assert!(has_block(body, condition, reference), "{condition}, created: {body}");
        let deferred: Vec<_> = session.deferrals().into_iter().map(|deferral| (deferral.construct, deferral.reason)).collect();
        assert_eq!(
            deferred,
            vec![(DeferredConstruct::BlockCondition, DeferralReason::CreatedAtLaunch { path: reference.to_string() })],
            "{condition}, created"
        );

        // Another file being created leaves this one an ordinary miss.
        let (outcome, session) = dry_compose_created(dir.path(), &guarded(condition, reference), "gen/other.md");
        assert_settled(&outcome, on_miss, &format!("{condition}, unrelated writer"));
        assert!(session.deferrals().is_empty(), "{condition}, unrelated writer: {:?}", session.deferrals());
    }
}
