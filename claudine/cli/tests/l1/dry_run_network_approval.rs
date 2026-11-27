//! A dry run's remote reads follow the captured `permissions.network`
//! (F3, L1): `allow` fetches, `deny` is the real run's refusal as an error
//! row, and `ask` sends no request and is one `not checked` row ("needs
//! network approval"), the dry run exiting 0. Each cell is one spawn of
//! `compose`, `inline-compose`, or `sequence --dry-run --json` in its own
//! fixture, whose home's `.darkmatter.yaml` sets `network`, and counts the
//! requests a loopback responder receives. A dry run's `ask` and `deny`
//! cells send no request, so they can write no remote cache.
//!
//! The readers are `load_data` in a prompt (each command), a remote `::file`
//! and `::code` (compose), and a schema slot (compose). A sequence source
//! expression (`sequence: "{{ load_data(…) }}"`) is local-only by contract,
//! so it sends no request under any setting.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::common::CliProcessFixture;

const APPROVAL_ROW: &str = "needs network approval (`permissions.network: ask`), which a dry run does not ask for";

/// A loopback HTTP responder serving fixed routes and counting every
/// request for the rest of the process.
pub(crate) struct Responder {
    base: String,
    hits: Arc<Mutex<HashMap<String, usize>>>,
}

impl Responder {
    pub(crate) fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let base = format!("http://127.0.0.1:{}", listener.local_addr().expect("address").port());
        let hits = Arc::new(Mutex::new(HashMap::new()));
        let counter = Arc::clone(&hits);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(&stream);
                let mut request_line = String::new();
                if reader.read_line(&mut request_line).is_err() {
                    continue;
                }
                let mut header = String::new();
                while reader.read_line(&mut header).is_ok_and(|read| read > 2) {
                    header.clear();
                }
                let route = request_line.split_whitespace().nth(1).unwrap_or_default().to_string();
                *counter.lock().unwrap().entry(route.clone()).or_default() += 1;
                let body = match route.as_str() {
                    "/data.json" => "[\"a\", \"b\"]",
                    "/rows.json" => "[\"./ask.md\"]",
                    "/child.md" => "Remote child.",
                    "/snippet.rs" => "fn remote() {}",
                    "/os.yaml" => "- macOS\n- Linux\n",
                    _ => "",
                };
                let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = (&stream).write_all(response.as_bytes());
            }
        });
        Self { base, hits }
    }

    pub(crate) fn url(&self, route: &str) -> String {
        format!("{}{route}", self.base)
    }

    /// How many distinct routes were requested.
    fn routes(&self) -> usize {
        self.hits.lock().unwrap().len()
    }

    pub(crate) fn requests(&self) -> usize {
        self.hits.lock().unwrap().values().sum()
    }
}

/// A fixture whose home says `network: <setting>`, with a provider stub the
/// check only looks up.
fn fixture(setting: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named("dry-run-network");
    fs::write(fixture.home().join(".darkmatter.yaml"), format!("permissions:\n  network: {setting}\n")).unwrap();
    let claude = fixture.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX));
    fs::write(&claude, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fixture
}

/// `claudine <command> <file> --dry-run --json`: the exit code and the
/// envelope.
fn dry_run(fixture: &CliProcessFixture, command: &str, file: &str) -> (Option<i32>, Value) {
    let output = fixture.command().args([command, file, "--dry-run", "--json"]).output().unwrap();
    assert!(!fixture.audio_spool().exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    (output.status.code(), envelope)
}

fn rows(envelope: &Value) -> Vec<(String, String, String)> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let text = |key: &str| row[key].as_str().unwrap_or_default().to_string();
            (text("code"), text("severity"), text("message"))
        })
        .collect()
}

/// The outcome every reader shares for `setting`: `allow` is clean having
/// requested `fetched` distinct URLs (a preparation may read one more than
/// once, as the real run does), `deny` is an error naming the refused host with
/// none, and `ask` is one approval reason with none. An inline prompt
/// is both a frontmatter value and the body composed from it, so its one
/// read is two rows sharing that reason, listed together in the report.
fn assert_cell(setting: &str, server: &Responder, (code, envelope): (Option<i32>, Value), fetched: usize, denied: &str) {
    let rows = rows(&envelope);
    let approvals: Vec<_> = rows.iter().filter(|(_, _, message)| message.contains(APPROVAL_ROW)).collect();
    match setting {
        "allow" => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(approvals.is_empty(), "{rows:#?}");
            assert_eq!(server.routes(), fetched, "{rows:#?}");
        }
        "deny" => {
            assert_eq!(code, Some(2), "{rows:#?}");
            assert!(rows.iter().any(|(_, severity, message)| severity == "error" && message.contains(denied)), "{rows:#?}");
            assert!(approvals.is_empty(), "{rows:#?}");
            assert_eq!(server.requests(), 0, "{rows:#?}");
        }
        _ => {
            assert_eq!(code, Some(0), "{rows:#?}");
            let reasons: std::collections::BTreeSet<_> =
                approvals.iter().filter_map(|(_, _, message)| message.split_once(" (").map(|(_, reason)| reason)).collect();
            assert_eq!(reasons.len(), 1, "{rows:#?}");
            assert!(approvals.iter().all(|(code, _, _)| code == "dm.check.not_checked"), "{rows:#?}");
            assert!(!rows.iter().any(|(_, severity, _)| severity == "error" || severity == "warning"), "{rows:#?}");
            assert_eq!(server.requests(), 0, "{rows:#?}");
        }
    }
}

const DENIED_HOST: &str = "remote read denied for host '127.0.0.1'";

fn load_data_body(server: &Responder) -> String {
    format!("Data {{{{ load_data('{}', 'string[]') }}}}.\n", server.url("/data.json"))
}

fn compose_load_data(setting: &str) {
    let server = Responder::start();
    let fixture = fixture(setting);
    fs::write(fixture.cwd().join("doc.md"), format!("---\nagent: claude\n---\n{}", load_data_body(&server))).unwrap();
    assert_cell(setting, &server, dry_run(&fixture, "compose", "doc.md"), 1, DENIED_HOST);
}

fn inline_compose_load_data(setting: &str) {
    let server = Responder::start();
    let fixture = fixture(setting);
    let text = format!(
        "---\nagent: claude\nprompt: \"Summarize {{{{ load_data('{}', 'string[]') }}}}\"\n---\nBody.\n",
        server.url("/data.json")
    );
    fs::write(fixture.cwd().join("doc.md"), &text).unwrap();
    assert_cell(setting, &server, dry_run(&fixture, "inline-compose", "doc.md"), 1, DENIED_HOST);
    assert_eq!(fs::read_to_string(fixture.cwd().join("doc.md")).unwrap(), text, "the dry run left the file intact");
}

fn sequence_load_data(setting: &str) {
    let server = Responder::start();
    let fixture = fixture(setting);
    fs::write(fixture.cwd().join("ask.md"), format!("---\nagent: claude\n---\n{}", load_data_body(&server))).unwrap();
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: ask\n    prompt: ./ask.md\n---\nBody.\n").unwrap();
    assert_cell(setting, &server, dry_run(&fixture, "sequence", "seq.md"), 1, DENIED_HOST);
}

/// A remote `::file` and `::code` in one prompt: two fetches under `allow`.
fn compose_transclusion(setting: &str) {
    let server = Responder::start();
    let fixture = fixture(setting);
    let text = format!("---\nagent: claude\n---\nBefore.\n\n::file {}\n\n::code {}\n", server.url("/child.md"), server.url("/snippet.rs"));
    fs::write(fixture.cwd().join("doc.md"), text).unwrap();
    let (code, envelope) = dry_run(&fixture, "compose", "doc.md");
    if setting == "ask" {
        // One row per URL, listed together under the reason they share.
        let rows = rows(&envelope);
        assert_eq!(code, Some(0), "{rows:#?}");
        for route in ["/child.md", "/snippet.rs"] {
            assert!(rows.iter().any(|(code, _, message)| code == "dm.check.not_checked" && message.contains(&server.url(route))), "{route}: {rows:#?}");
        }
        assert_eq!(server.requests(), 0, "{rows:#?}");
        return;
    }
    assert_cell(setting, &server, (code, envelope), 2, DENIED_HOST);
}

/// A schema slot whose enum reads the responder.
fn compose_schema_slot(setting: &str) {
    let server = Responder::start();
    let fixture = fixture(setting);
    fs::write(
        fixture.cwd().join("schema.yaml"),
        format!("kind: schema\n$schema:\n    os: \"enum({{{{ load_data(\\\"{}\\\", \\\"string[]\\\") }}}})\"\n", server.url("/os.yaml")),
    )
    .unwrap();
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n$schema: ./schema.yaml\nos: macOS\n---\nBody.\n").unwrap();
    let (code, envelope) = dry_run(&fixture, "compose", "doc.md");
    if setting == "deny" {
        let rows = rows(&envelope);
        assert!(rows.iter().any(|(code, _, _)| code == "dm.schema.slot_network_denied"), "{rows:#?}");
        assert!(!rows.iter().any(|(code, _, _)| code == "dm.schema.slot_approval_required" || code == "dm.check.not_checked"), "the typed refusal alone: {rows:#?}");
    }
    assert_cell(setting, &server, (code, envelope), 1, "`permissions.network` is `deny`");
}

/// A sequence source expression reads local files only, whatever the
/// setting: its remote `load_data` sends nothing, and the list is a finding.
fn sequence_source_expression(setting: &str) {
    let server = Responder::start();
    let fixture = fixture(setting);
    fs::write(fixture.cwd().join("ask.md"), "---\nagent: claude\n---\nAsk.\n").unwrap();
    fs::write(
        fixture.cwd().join("seq.md"),
        format!("---\nsequence: \"{{{{ load_data('{}', 'string[]') }}}}\"\n---\nBody.\n", server.url("/rows.json")),
    )
    .unwrap();
    let (code, envelope) = dry_run(&fixture, "sequence", "seq.md");
    let rows = rows(&envelope);
    assert_eq!(code, Some(2), "{rows:#?}");
    assert!(!rows.iter().any(|(_, _, message)| message.contains(APPROVAL_ROW)), "{rows:#?}");
    assert_eq!(server.requests(), 0, "a sequence source never fetches: {rows:#?}");
}

macro_rules! cells {
    ($($reader:ident),* $(,)?) => {
        $(mod $reader {
            #[test]
            fn allow() { super::$reader("allow"); }
            #[test]
            fn ask() { super::$reader("ask"); }
            #[test]
            fn deny() { super::$reader("deny"); }
        })*
    };
}

cells!(
    compose_load_data,
    inline_compose_load_data,
    sequence_load_data,
    compose_transclusion,
    compose_schema_slot,
    sequence_source_expression,
);
