//! `md`'s side of the permission parity fixture
//! (`tests/fixtures/permission_parity/cases.json`, F2, L1): every command,
//! file read, URL read, and exclusion row decided through the library paths
//! `md compose` runs, with no approval handler.
//!
//! - A command row is decided by `Markdown::compose_preflight_approvals`.
//! - A file or URL row is a schema slot reading the target, decided by
//!   composing the document (`Markdown::compose_with`): the request's slot
//!   session gates the read under the captured `permissions`, the network
//!   permission, and `--allow host=HOST` values.
//!
//! Claudine's preparation and DMLS's passive classification read the same
//! table in their own suites and must reach the same decisions.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot, ShellExpansionError};
use serde_json::Value;

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";

/// A home and a work folder under one temporary directory, the home holding
/// the fixture's configuration (with `network`) and files.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new(cases: &Value, network: &str) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let fixture = Self { _dir: dir, root };
        std::fs::create_dir_all(fixture.work()).expect("work");
        let config = format!("{}  network: {network}\n", cases["user_permissions"].as_str().expect("user_permissions"));
        fixture.write_home(".darkmatter.yaml", &config);
        for (relative, text) in cases["home_files"].as_object().expect("home_files") {
            fixture.write_home(relative, text.as_str().expect("file text"));
        }
        fixture
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    fn write_home(&self, relative: &str, text: &str) {
        let path = relative.split('/').fold(self.home(), |path, part| path.join(part));
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, text).expect("write");
    }

    /// `document` as `work/doc.md` and its request, allowing `hosts` for the
    /// run.
    fn request(&self, document: &str, hosts: &[String]) -> (Markdown, ComposeRequest) {
        let path = self.work().join("doc.md");
        std::fs::write(&path, document).expect("document");
        let markdown = Markdown::try_from(path.as_path()).expect("markdown");
        let snapshot = RequestSnapshot::new(self.work()).with_home(Some(self.home()));
        let options = hosts.iter().fold(ComposeOptions::new().with_source_file(&path), |options, host| options.with_allowed_host(host));
        (markdown, ComposeRequest::prepare(options, &snapshot).expect("a request"))
    }

    /// md's decision about the document's commands.
    fn command(&self, document: &str) -> &'static str {
        let (markdown, request) = self.request(document, &[]);
        match markdown.compose_preflight_approvals(&request, None) {
            Ok(_) => "preapproved",
            Err(ShellExpansionError::ApprovalRequired { .. }) => "approval-required",
            Err(ShellExpansionError::Blacklisted { .. }) => "refused",
            Err(other) => panic!("md: {other}"),
        }
    }

    /// md's decision about a schema slot reading `target`.
    fn read(&self, target: &str, hosts: &[String]) -> &'static str {
        std::fs::write(self.work().join("schema.yaml"), slot_schema(target)).expect("schema");
        let (markdown, request) = self.request("---\n$schema: ./schema.yaml\nos: macOS\n---\n\nBody.\n", hosts);
        match markdown.compose_with(&request) {
            Ok(_) => "preapproved",
            Err(error) => {
                let codes: Vec<String> = error_diagnostics(&error).into_iter().map(|diagnostic| diagnostic.code).collect();
                assert!(codes.iter().any(|code| code == APPROVAL_REQUIRED), "md: {error} ({codes:?})");
                "approval-required"
            }
        }
    }
}

/// A schema whose `os` members a slot loads from `target`.
fn slot_schema(target: &str) -> String {
    format!("kind: schema\n$schema:\n    os: \"enum({{{{ load_data(\\\"{target}\\\", \\\"string[]\\\") }}}})\"\n")
}

/// The `::shell` document a command row composes.
fn command_document(row: &Value) -> String {
    match row["command"].as_str() {
        Some(command) => format!("::shell {command}\n"),
        None => row["document"].as_str().expect("a command or a document").to_string(),
    }
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
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = (&stream).write_all(response.as_bytes());
        }
    });
    format!("http://127.0.0.1:{port}/os.yaml")
}

fn strings(value: &Value) -> Vec<String> {
    value.as_array().into_iter().flatten().filter_map(|item| item.as_str().map(str::to_string)).collect()
}

#[test]
fn md_decides_every_command_row_as_the_fixture_expects() {
    let cases: Value = serde_json::from_str(include_str!("../fixtures/permission_parity/cases.json")).expect("cases");
    let fixture = Fixture::new(&cases, "deny");

    for row in cases["commands"].as_array().expect("commands") {
        let decided = fixture.command(&command_document(row));

        assert_eq!(decided, row["decision"], "{}", row["name"]);
    }
}

#[test]
fn md_decides_every_file_read_row_as_the_fixture_expects() {
    let cases: Value = serde_json::from_str(include_str!("../fixtures/permission_parity/cases.json")).expect("cases");
    let fixture = Fixture::new(&cases, "deny");

    for row in cases["file_reads"].as_array().expect("file_reads") {
        let decided = fixture.read(row["target"].as_str().expect("target"), &[]);

        assert_eq!(decided, row["decision"], "{}", row["name"]);
    }
}

#[test]
fn md_decides_every_url_read_row_as_the_fixture_expects() {
    let cases: Value = serde_json::from_str(include_str!("../fixtures/permission_parity/cases.json")).expect("cases");
    let url = serve("- macOS\n- Linux\n");

    for row in cases["url_reads"].as_array().expect("url_reads") {
        let fixture = Fixture::new(&cases, row["network"].as_str().expect("network"));

        let decided = fixture.read(&url, &strings(&row["allow_hosts"]));

        assert_eq!(decided, row["decision"], "{}", row["name"]);
    }
}
