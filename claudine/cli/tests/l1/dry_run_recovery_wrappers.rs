//! A recovery wrapper keeps a dry run's unattempted read unchecked (L1).
//!
//! A prompt's whole body sits in a `::block` guarded by a wrapper over a
//! remote document: `try_frontmatter(url).ok`, which turns a failed read into
//! `{ ok: false }`, and the lazy `try(frontmatter(url), {})`. Under
//! `network: ask` the dry run sends no request and leaves the condition
//! `not checked` with the approval reason, exiting 0, instead of settling it
//! as false and reporting an empty body. `allow` reads the document once,
//! and `deny` settles as a real run does: `try_frontmatter` turns the
//! refusal into `ok: false`, so the body is empty, while `try` does not
//! recover a refused host. Each route (`compose`, `inline-compose`,
//! `sequence`) is one spawn per cell.

use std::fs;

use crate::common::CliProcessFixture;
use crate::dry_run_network_approval::{Responder, dry_run, fixture, rows};

const APPROVAL_ROW: &str = "needs network approval (`permissions.network: ask`), which a dry run does not ask for";
const EMPTY_BODY: &str = "dm.compose.empty_body";

/// The guarded body, as Markdown lines.
fn guarded(condition: &str) -> String {
    format!("::block when=\"{condition}\"\nGuarded.\n::end-block\n")
}

/// Writes the route's files and returns the command and the file it runs.
fn stage(fixture: &CliProcessFixture, route: &str, condition: &str) -> (&'static str, &'static str) {
    let body = guarded(condition);
    match route {
        "compose" => {
            fs::write(fixture.cwd().join("doc.md"), format!("---\nagent: claude\n---\n{body}")).unwrap();
            ("compose", "doc.md")
        }
        "inline-compose" => {
            let prompt: String = body.lines().map(|line| format!("  {line}\n")).collect();
            fs::write(fixture.cwd().join("doc.md"), format!("---\nagent: claude\nprompt: |\n{prompt}---\nBody.\n")).unwrap();
            ("inline-compose", "doc.md")
        }
        _ => {
            fs::write(fixture.cwd().join("ask.md"), format!("---\nagent: claude\n---\n{body}")).unwrap();
            fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: ask\n    prompt: ./ask.md\n---\nBody.\n").unwrap();
            ("sequence", "seq.md")
        }
    }
}

/// How `deny` settles the wrapper.
enum OnDeny {
    /// The refusal becomes data: the condition is false and the body empty.
    EmptyBody,
    /// The refusal propagates as an error naming the host.
    Refused,
}

fn cell(route: &str, setting: &str, wrapper: &str, on_deny: OnDeny) {
    let server = Responder::start();
    let fixture = fixture(setting);
    let condition = wrapper.replace("URL", &server.url("/facts.md"));
    let (command, file) = stage(&fixture, route, &condition);

    let (code, envelope) = dry_run(&fixture, command, file);

    let rows = rows(&envelope);
    let approvals: Vec<_> = rows.iter().filter(|(_, _, message)| message.contains(APPROVAL_ROW)).collect();
    let errors: Vec<&str> = rows.iter().filter(|(_, severity, _)| severity == "error").map(|(code, _, _)| code.as_str()).collect();
    match setting {
        "ask" => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(!approvals.is_empty(), "{rows:#?}");
            assert!(approvals.iter().all(|(code, _, message)| code == "dm.check.not_checked" && message.contains("condition")), "{rows:#?}");
            assert!(!rows.iter().any(|(_, severity, _)| severity == "error" || severity == "warning"), "{rows:#?}");
            assert_eq!(server.requests(), 0, "{rows:#?}");
        }
        "allow" => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(approvals.is_empty() && errors.is_empty(), "{rows:#?}");
            assert_eq!(server.hits("/facts.md"), server.requests(), "{rows:#?}");
            assert!(server.requests() >= 1, "{rows:#?}");
        }
        _ => {
            assert_eq!(code, Some(2), "{rows:#?}");
            assert!(approvals.is_empty(), "{rows:#?}");
            match on_deny {
                OnDeny::EmptyBody => assert_eq!(errors, [EMPTY_BODY], "{rows:#?}"),
                OnDeny::Refused => assert!(
                    rows.iter().any(|(_, severity, message)| severity == "error" && message.contains("remote read denied for host '127.0.0.1'")),
                    "{rows:#?}"
                ),
            }
            assert_eq!(server.requests(), 0, "{rows:#?}");
        }
    }
}

const TRY_FRONTMATTER: &str = "try_frontmatter('URL').ok";
const LAZY_TRY: &str = "try(frontmatter('URL'), {}).title";

macro_rules! cells {
    ($($name:ident: $route:expr, $wrapper:expr, $on_deny:expr;)*) => {$(
        mod $name {
            use super::*;

            #[test]
            fn allow() { cell($route, "allow", $wrapper, $on_deny); }
            #[test]
            fn ask() { cell($route, "ask", $wrapper, $on_deny); }
            #[test]
            fn deny() { cell($route, "deny", $wrapper, $on_deny); }
        }
    )*};
}

cells! {
    // R4-F3.1
    compose_try_frontmatter: "compose", TRY_FRONTMATTER, OnDeny::EmptyBody;
    inline_compose_try_frontmatter: "inline-compose", TRY_FRONTMATTER, OnDeny::EmptyBody;
    sequence_try_frontmatter: "sequence", TRY_FRONTMATTER, OnDeny::EmptyBody;
    // R4-F3.3: the lazy `try` around `frontmatter`.
    compose_lazy_try: "compose", LAZY_TRY, OnDeny::Refused;
}
