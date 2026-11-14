//! Claudine's shell preparation decides every command the way `md` does,
//! from the same captured `permissions` (F2 parity).
//!
//! Each row composes one document through Darkmatter's own pre-flight
//! approval (`Markdown::compose_preflight_approvals`, what `md compose`
//! runs) and through Claudine's (`resolve_shell_approvals` over
//! `CapturedPermissions` of the same request's snapshot), with no handler or
//! with one that approves everything (`--yolo`), and requires the same
//! decision: pre-approved, approval required, refused by the built-in
//! blacklist, or a configuration error. The fixtures write `permissions` into
//! a temporary home's `~/.darkmatter.yaml` and, for the commit rule, a
//! temporary repository's `.darkmatter.yaml`.
//!
//! `shared_fixture` decides the table `md`'s and DMLS's suites decide too
//! (`darkmatter/lib/tests/fixtures/permission_parity/cases.json`): its
//! commands, and its file and URL reads as schema slots, through `md`'s
//! composition and Claudine's preparation (`prepare_direct`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use claudine::composition::preflight::resolve_shell_approvals;
use claudine::composition::{CompositionError, ShellApprovalFailure};
use claudine::harness::{CapturedPermissions, HarnessError, ShellApprovalOptions};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};

/// What a host decided about a document's one command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    /// Approved without asking (or, with an approving handler, approved).
    Approved,
    /// Nothing approves it, and there is no handler to ask.
    ApprovalRequired,
    /// The built-in blacklist refused it.
    Refused,
    /// The permissions are invalid, so nothing was judged.
    ConfigError,
}

/// Approves everything it is asked about, like `--yolo`.
struct ApproveAll;

impl ShellApprovalHandler for ApproveAll {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// A home and a work folder under one temporary directory.
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

    /// `dir/.darkmatter.yaml` with `rules` as its `permissions.shell-commands`.
    fn permissions(&self, dir: &Path, rules: &[&str]) {
        let list: Vec<String> = rules.iter().map(|rule| serde_json::to_string(rule).unwrap()).collect();
        std::fs::write(dir.join(".darkmatter.yaml"), format!("permissions:\n  shell-commands: [{}]\n", list.join(", ")))
            .unwrap();
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

    /// `md`'s and Claudine's decisions for `document`, composed as
    /// `work/doc.md`, without a handler and with an approving one.
    fn decide(&self, document: &str) -> [(Decision, Decision); 2] {
        let path = self.work().join("doc.md");
        std::fs::write(&path, document).unwrap();
        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let snapshot = RequestSnapshot::new(self.work()).with_home(Some(self.home()));
        let request =
            ComposeRequest::prepare(ComposeOptions::new().with_source_file(&path), &snapshot).expect("a request");
        let permissions = CapturedPermissions::of_snapshot(request.config_snapshot());
        [None, Some(Arc::new(ApproveAll) as Arc<dyn ShellApprovalHandler>)].map(|handler| {
            let md = md_decision(markdown.compose_preflight_approvals(&request, handler.clone()));
            let options = ShellApprovalOptions {
                permissions: permissions.clone(),
                approval_handler: handler,
                ..Default::default()
            };
            let claudine = claudine_decision(resolve_shell_approvals(Some(&markdown), Some(&request), &options, None, None));
            (md, claudine)
        })
    }
}

fn md_decision<T>(result: Result<T, ShellExpansionError>) -> Decision {
    match result {
        Ok(_) => Decision::Approved,
        Err(ShellExpansionError::ApprovalRequired { .. }) => Decision::ApprovalRequired,
        Err(ShellExpansionError::Blacklisted { .. }) => Decision::Refused,
        Err(ShellExpansionError::Config(_)) => Decision::ConfigError,
        Err(other) => panic!("md: {other}"),
    }
}

fn claudine_decision<T>(result: Result<T, CompositionError>) -> Decision {
    match result {
        Ok(_) => Decision::Approved,
        Err(CompositionError::ShellApprovalUnavailable { failure: ShellApprovalFailure::NoHandler, .. }) => {
            Decision::ApprovalRequired
        }
        Err(CompositionError::ShellApprovalUnavailable { failure: ShellApprovalFailure::Blacklisted(_), .. }) => {
            Decision::Refused
        }
        Err(CompositionError::PreFlightShellAuditFailed { source: HarnessError::ShellPermissionsInvalid { .. } }) => {
            Decision::ConfigError
        }
        Err(other) => panic!("claudine: {other}"),
    }
}

/// Both hosts decide `expected` without a handler and `with_handler` with an
/// approving one.
#[track_caller]
fn assert_parity(decisions: [(Decision, Decision); 2], expected: Decision, with_handler: Decision) {
    let [(md, claudine), (md_yolo, claudine_yolo)] = decisions;
    assert_eq!((md, claudine), (expected, expected), "(md, claudine) without a handler");
    assert_eq!((md_yolo, claudine_yolo), (with_handler, with_handler), "(md, claudine) approving everything");
}

#[test]
fn an_allowed_command_is_preapproved() {
    let fixture = Fixture::new();
    fixture.permissions(&fixture.home(), &["prefix echo"]);

    let decisions = fixture.decide("::shell echo hello\n");

    assert_parity(decisions, Decision::Approved, Decision::Approved);
}

#[test]
fn an_excluded_command_requires_approval() {
    let fixture = Fixture::new();
    fixture.permissions(&fixture.home(), &["prefix echo", "!exact echo secret"]);

    let decisions = fixture.decide("::shell echo secret\n");

    assert_parity(decisions, Decision::ApprovalRequired, Decision::Approved);
}

#[test]
fn a_builtin_blacklisted_command_is_refused_even_when_everything_is_approved() {
    let fixture = Fixture::new();
    fixture.permissions(&fixture.home(), &["prefix chmod"]);

    let decisions = fixture.decide("::shell chmod 600 notes.md\n");

    assert_parity(decisions, Decision::Refused, Decision::Refused);
}

#[test]
fn a_template_rule_approves_its_shape() {
    let fixture = Fixture::new();
    fixture.permissions(&fixture.home(), &["template git --version {{ string }}"]);

    let decisions = fixture.decide("---\nword: hello\n---\n::shell git --version {{ word }}\n");

    assert_parity(decisions, Decision::Approved, Decision::Approved);
}

#[test]
fn a_repository_allow_applies_only_once_committed() {
    let fixture = Fixture::new();
    fixture.permissions(&fixture.work(), &["prefix echo"]);
    fixture.git(&["init", "--quiet"]);
    fixture.git(&["add", ".darkmatter.yaml"]);
    fixture.git(&["commit", "--quiet", "-m", "permissions"]);
    // Widened in the working copy only: not effective until committed.
    fixture.permissions(&fixture.work(), &["prefix echo", "prefix printf"]);

    let committed = fixture.decide("::shell echo hello\n");
    let uncommitted = fixture.decide("::shell printf hello\n");

    assert_parity(committed, Decision::Approved, Decision::Approved);
    assert_parity(uncommitted, Decision::ApprovalRequired, Decision::Approved);
}

#[test]
fn invalid_permissions_are_an_error_never_permissive() {
    let fixture = Fixture::new();
    std::fs::write(fixture.home().join(".darkmatter.yaml"), "permissions:\n  shell-commands:\n    - frobnicate echo\n")
        .unwrap();

    let decisions = fixture.decide("::shell echo hello\n");

    assert_parity(decisions, Decision::ConfigError, Decision::ConfigError);
}

/// The shared permission parity fixture: the table `md`'s and DMLS's suites
/// decide too (`darkmatter/lib/tests/fixtures/permission_parity/cases.json`).
mod shared_fixture {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    use claudine::composition::{PrepareOptions, ResolvedCompositionSource, prepare_direct};
    use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
    use serde_json::Value;

    use super::*;

    const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";

    impl Fixture {
        /// The fixture's `user_permissions` with `network`, and its home
        /// files, written into the home.
        fn shared(cases: &Value, network: &str) -> Self {
            let fixture = Self::new();
            let config = format!("{}  network: {network}\n", cases["user_permissions"].as_str().expect("user_permissions"));
            std::fs::write(fixture.home().join(".darkmatter.yaml"), config).unwrap();
            for (relative, text) in cases["home_files"].as_object().expect("home_files") {
                let path = relative.split('/').fold(fixture.home(), |path, part| path.join(part));
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, text.as_str().expect("file text")).unwrap();
            }
            fixture
        }

        /// Claudine's preparation of a document whose schema slot reads
        /// `target`, and md's composition of the same document.
        fn read(&self, target: &str) -> (Decision, Decision) {
            std::fs::write(self.work().join("schema.yaml"), slot_schema(target)).unwrap();
            let text = "---\n$schema: ./schema.yaml\nos: macOS\n---\n\nBody.\n";
            let path = self.work().join("doc.md");
            std::fs::write(&path, text).unwrap();
            let markdown = Markdown::try_from(path.as_path()).unwrap();
            let snapshot = RequestSnapshot::new(self.work()).with_home(Some(self.home()));
            let request =
                ComposeRequest::prepare(ComposeOptions::new().with_source_file(&path), &snapshot).expect("a request");
            let md = read_decision(markdown.compose_with(&request).map(drop).map_err(|error| Box::new(error) as Box<dyn std::error::Error>));
            let source = ResolvedCompositionSource {
                original_ref: path.display().to_string(),
                resolved_path: path.clone(),
                original_text: text.to_string(),
                markdown,
            };
            let options = PrepareOptions::new(request.resolution_context().clone());
            let claudine = read_decision(prepare_direct(&source, options).map(drop).map_err(|error| Box::new(error) as Box<dyn std::error::Error>));
            (md, claudine)
        }
    }

    fn read_decision(result: Result<(), Box<dyn std::error::Error>>) -> Decision {
        match result {
            Ok(()) => Decision::Approved,
            Err(error) => {
                let codes: Vec<String> = error_diagnostics(error.as_ref()).into_iter().map(|diagnostic| diagnostic.code).collect();
                assert!(codes.iter().any(|code| code == APPROVAL_REQUIRED), "{error} ({codes:?})");
                Decision::ApprovalRequired
            }
        }
    }

    fn expected(row: &Value) -> Decision {
        match row["decision"].as_str().expect("decision") {
            "preapproved" => Decision::Approved,
            "approval-required" => Decision::ApprovalRequired,
            "refused" => Decision::Refused,
            other => panic!("unknown decision `{other}`"),
        }
    }

    /// A schema whose `os` members a slot loads from `target`.
    fn slot_schema(target: &str) -> String {
        format!("kind: schema\n$schema:\n    os: \"enum({{{{ load_data(\\\"{target}\\\", \\\"string[]\\\") }}}})\"\n")
    }

    /// Serves `body` to every request on a loopback port for the rest of the
    /// process, returning `http://127.0.0.1:{port}/os.yaml`.
    fn serve(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("address").port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                    line.clear();
                }
                let response =
                    format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = (&stream).write_all(response.as_bytes());
            }
        });
        format!("http://127.0.0.1:{port}/os.yaml")
    }

    #[test]
    fn every_command_row_is_decided_alike_by_md_and_claudine() {
        let cases: Value =
            serde_json::from_str(include_str!("../../../../darkmatter/lib/tests/fixtures/permission_parity/cases.json"))
                .expect("cases");
        let fixture = Fixture::shared(&cases, "deny");

        for row in cases["commands"].as_array().expect("commands") {
            let document = match row["command"].as_str() {
                Some(command) => format!("::shell {command}\n"),
                None => row["document"].as_str().expect("a command or a document").to_string(),
            };
            let [(md, claudine), _] = fixture.decide(&document);

            assert_eq!((md, claudine), (expected(row), expected(row)), "(md, claudine): {}", row["name"]);
        }
    }

    #[test]
    fn every_file_read_row_is_decided_alike_by_md_and_claudine_preparation() {
        let cases: Value =
            serde_json::from_str(include_str!("../../../../darkmatter/lib/tests/fixtures/permission_parity/cases.json"))
                .expect("cases");
        let fixture = Fixture::shared(&cases, "deny");

        for row in cases["file_reads"].as_array().expect("file_reads") {
            let decided = fixture.read(row["target"].as_str().expect("target"));

            assert_eq!(decided, (expected(row), expected(row)), "(md, claudine): {}", row["name"]);
        }
    }

    /// Claudine has no per-run host allowance (`md --allow host=HOST`), so
    /// only the rows that allow no host apply to it.
    #[test]
    fn every_url_read_row_without_a_run_host_is_decided_alike_by_md_and_claudine_preparation() {
        let cases: Value =
            serde_json::from_str(include_str!("../../../../darkmatter/lib/tests/fixtures/permission_parity/cases.json"))
                .expect("cases");
        let url = serve("- macOS\n- Linux\n");
        let rows: Vec<&Value> = cases["url_reads"]
            .as_array()
            .expect("url_reads")
            .iter()
            .filter(|row| row["allow_hosts"].as_array().is_some_and(Vec::is_empty))
            .collect();
        assert!(rows.len() >= 2, "the table keeps an allow and a deny row without a run host");

        for row in rows {
            let fixture = Fixture::shared(&cases, row["network"].as_str().expect("network"));

            let decided = fixture.read(&url);

            assert_eq!(decided, (expected(row), expected(row)), "(md, claudine): {}", row["name"]);
        }
    }
}
