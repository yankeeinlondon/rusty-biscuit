//! `permissions.network: ask` at every remote read dispatch (F2, L1).
//!
//! Each cell writes a home whose `.darkmatter.yaml` sets `network` to
//! `allow`, `ask`, or `deny`, composes a document in a work folder beside it,
//! and counts the requests a loopback responder receives:
//!
//! - `allow` fetches, in a real run and a dry run alike;
//! - `deny` refuses with no request, in both;
//! - `ask` in a dry run sends nothing and records one
//!   [`DeferralReason::NetworkNeedsApproval`], leaving what depends on the
//!   read as written; in a real run a composition read refuses with its typed
//!   approval error (composition has no prompt), and a schema slot, which
//!   can ask, needs approval where nothing asks.
//!
//! The readers covered are `load_data`, a remote `::file` and `::code`, a
//! `prologue`, and a schema slot. A dry run writes no persistent cache under
//! any setting, and a real run writes none under `ask` or `deny`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use darkmatter::config::network::NetworkPermission;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::passive::{Deferral, DeferralReason, DeferredConstruct};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};

const NETWORK_DENIED: &str = "dm.schema.slot_network_denied";
const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const TRANSCLUSION_NEEDS_APPROVAL: &str = "dm.transclusion.remote_needs_approval";

/// A loopback HTTP responder serving fixed routes and counting every request
/// by path for the rest of the process.
struct Responder {
    base: String,
    hits: Arc<Mutex<HashMap<String, usize>>>,
}

impl Responder {
    fn start() -> Self {
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
                    "/child.md" => "Remote child.",
                    "/snippet.rs" => "fn remote() {}",
                    "/os.yaml" => "- macOS\n- Linux\n",
                    _ => "",
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nCache-Control: max-age=3600\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = (&stream).write_all(response.as_bytes());
            }
        });
        Self { base, hits }
    }

    fn url(&self, route: &str) -> String {
        format!("{}{route}", self.base)
    }

    fn requests(&self) -> usize {
        self.hits.lock().unwrap().values().sum()
    }
}

/// A home whose `.darkmatter.yaml` sets `network`, and a work folder beside
/// it, under one temporary directory.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new(network: &str) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("home")).expect("home");
        std::fs::create_dir_all(root.join("work")).expect("work");
        std::fs::write(root.join("home/.darkmatter.yaml"), format!("permissions:\n  network: {network}\n")).expect("config");
        Self { _dir: dir, root }
    }

    fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join("work").join(name);
        std::fs::write(&path, text).expect("write");
        path
    }

    fn request(&self, path: &Path, dry_run: bool) -> ComposeRequest {
        let snapshot = RequestSnapshot::new(self.root.join("work")).with_home(Some(self.root.join("home")));
        let options = ComposeOptions::new()
            .with_source_file(path)
            .with_allow_remote_transclusion(true)
            .with_cache_root(self.cache());
        let options = if dry_run { options.with_dry_run() } else { options };
        ComposeRequest::prepare(options, &snapshot).expect("a request")
    }

    /// `text` composed as `work/doc.md`: the body, or the failure's codes
    /// and text, and the dry run's deferrals.
    fn compose(&self, text: &str, dry_run: bool) -> Composed {
        let path = self.write("doc.md", text);
        let request = self.request(&path, dry_run);
        let session = request.options().passive_session().cloned();
        let outcome = Markdown::try_from(path.as_path()).expect("markdown").compose_with(&request);
        Composed {
            outcome: outcome.map(|(composed, _)| composed.content().to_string()).map_err(|error| {
                let codes = error_diagnostics(&error).into_iter().map(|diagnostic| diagnostic.code).collect();
                (codes, error.to_string())
            }),
            deferrals: session.map(|session| session.deferrals()).unwrap_or_default(),
        }
    }
}

struct Composed {
    outcome: Result<String, (Vec<String>, String)>,
    deferrals: Vec<Deferral>,
}

impl Composed {
    fn body(&self) -> &str {
        self.outcome.as_ref().unwrap_or_else(|(codes, error)| panic!("composition failed: {error} ({codes:?})"))
    }

    fn failure(&self) -> &(Vec<String>, String) {
        self.outcome.as_ref().expect_err("composition refused the read")
    }

    /// The deferrals that name network approval for `url`.
    fn approvals(&self, url: &str) -> Vec<&Deferral> {
        self.deferrals
            .iter()
            .filter(|deferral| deferral.reason == DeferralReason::NetworkNeedsApproval { url: url.to_string() })
            .collect()
    }
}

#[test]
fn the_network_setting_reads_allow_ask_and_deny_and_refuses_anything_else() {
    for (setting, expected) in [("allow", Some(NetworkPermission::Allow)), ("ask", Some(NetworkPermission::Ask)), ("deny", Some(NetworkPermission::Deny)), ("sometimes", None)] {
        let fixture = Fixture::new(setting);
        let request = fixture.request(&fixture.write("doc.md", "Body.\n"), false);

        let permission = NetworkPermission::from_snapshot(Some(request.config_snapshot()));

        match expected {
            Some(expected) => assert_eq!(permission, expected, "{setting}"),
            None => assert!(permission.config_error().is_some(), "{setting}: {permission:?}"),
        }
    }
}

/// `load_data` with a URL: the same decision in every body reader that
/// fetches through the request's remote runtime.
#[test]
fn load_data_fetches_on_allow_refuses_on_deny_and_defers_on_ask() {
    for (setting, dry_run) in [("allow", true), ("allow", false), ("deny", true), ("deny", false), ("ask", true), ("ask", false)] {
        let server = Responder::start();
        let fixture = Fixture::new(setting);
        let url = server.url("/data.json");
        let document = format!("Before.\n\nData {{{{ load_data('{url}', 'string[]') }}}}.\n");

        let run = fixture.compose(&document, dry_run);

        let cell = format!("{setting}, dry run {dry_run}");
        match (setting, dry_run) {
            ("allow", _) => {
                assert!(run.body().contains("Data [\"a\",\"b\"]."), "{cell}: {}", run.body());
                assert_eq!(server.requests(), 1, "{cell}");
            }
            ("deny", _) => {
                assert!(run.failure().1.contains("remote read denied for host '127.0.0.1'"), "{cell}: {:?}", run.failure());
                assert_eq!(server.requests(), 0, "{cell}");
            }
            ("ask", true) => {
                assert!(run.body().contains("Before.") && run.body().contains(&format!("Data {{{{ load_data('{url}', 'string[]') }}}}.")), "{cell}: {}", run.body());
                assert_eq!(run.approvals(&url).len(), 1, "{cell}: {:?}", run.deferrals);
                assert_eq!(run.deferrals.len(), 1, "{cell}: {:?}", run.deferrals);
                assert_eq!(server.requests(), 0, "{cell}");
            }
            _ => {
                let (_, error) = run.failure();
                assert!(error.contains("needs approval") && error.contains("`permissions.network` is `ask`"), "{cell}: {error}");
                assert_eq!(server.requests(), 0, "{cell}");
            }
        }
        if dry_run || setting != "allow" {
            assert!(!fixture.cache().exists(), "{cell}: the remote cache was written");
        }
    }
}

/// A remote `::file`, `::code`, and `prologue`: one decision for every
/// transclusion, the dry run under `ask` leaving each directive as written.
#[test]
fn remote_transclusion_fetches_on_allow_refuses_on_deny_and_defers_on_ask() {
    let documents = [
        ("/child.md", "Before.\n\n::file {url}\n", "Remote child."),
        ("/snippet.rs", "Before.\n\n::code {url}\n", "fn remote() {}"),
        ("/child.md", "---\nprologue: {url}\n---\nBody.\n", "Remote child."),
    ];
    for (route, template, fetched) in documents {
        for (setting, dry_run) in [("allow", true), ("deny", true), ("deny", false), ("ask", true), ("ask", false)] {
            let server = Responder::start();
            let fixture = Fixture::new(setting);
            let url = server.url(route);
            let document = template.replace("{url}", &url);

            let run = fixture.compose(&document, dry_run);

            let cell = format!("{template:?}, {setting}, dry run {dry_run}");
            match (setting, dry_run) {
                ("allow", _) => {
                    assert!(run.body().contains(fetched), "{cell}: {}", run.body());
                    assert_eq!(server.requests(), 1, "{cell}");
                }
                ("deny", _) => {
                    assert!(run.failure().1.contains("remote read denied for host '127.0.0.1'"), "{cell}: {:?}", run.failure());
                    assert_eq!(server.requests(), 0, "{cell}");
                }
                ("ask", true) => {
                    assert!(!run.body().contains(fetched), "{cell}: {}", run.body());
                    if let Some(directive) = document.lines().find(|line| line.starts_with("::")) {
                        assert!(run.body().contains(directive), "{cell}: {}", run.body());
                    }
                    let approvals = run.approvals(&url);
                    assert_eq!(approvals.len(), 1, "{cell}: {:?}", run.deferrals);
                    assert_eq!(approvals[0].construct, DeferredConstruct::Transclusion, "{cell}");
                    assert_eq!(server.requests(), 0, "{cell}");
                }
                _ => {
                    let (codes, error) = run.failure();
                    assert!(codes.iter().any(|code| code == TRANSCLUSION_NEEDS_APPROVAL), "{cell}: {codes:?} {error}");
                    assert_eq!(server.requests(), 0, "{cell}");
                }
            }
            if dry_run || setting != "allow" {
                assert!(!fixture.cache().exists(), "{cell}: the remote cache was written");
            }
        }
    }
}

/// A schema slot whose enum reads the responder: `allow` fetches once,
/// `deny` is its typed refusal alone, and `ask` is only the deferral in a
/// dry run and an approval to give where nothing asks in a real run.
#[test]
fn a_schema_slot_fetches_on_allow_refuses_on_deny_and_defers_on_ask() {
    for (setting, dry_run) in [("allow", true), ("deny", true), ("deny", false), ("ask", true), ("ask", false)] {
        let server = Responder::start();
        let fixture = Fixture::new(setting);
        let url = server.url("/os.yaml");
        fixture.write(
            "schema.yaml",
            &format!("kind: schema\n$schema:\n    os: \"enum({{{{ load_data(\\\"{url}\\\", \\\"string[]\\\") }}}})\"\n"),
        );

        let run = fixture.compose("---\n$schema: ./schema.yaml\nos: macOS\n---\n\nBody.\n", dry_run);

        let cell = format!("{setting}, dry run {dry_run}");
        match (setting, dry_run) {
            ("allow", _) => {
                assert!(run.body().contains("Body."), "{cell}");
                assert!(run.deferrals.is_empty(), "{cell}: {:?}", run.deferrals);
                assert_eq!(server.requests(), 1, "{cell}");
            }
            ("deny", _) => {
                let (codes, error) = run.failure();
                assert!(codes.iter().any(|code| code == NETWORK_DENIED), "{cell}: {codes:?} {error}");
                assert!(!codes.iter().any(|code| code == APPROVAL_REQUIRED), "{cell}: {codes:?}");
                assert!(run.deferrals.is_empty(), "{cell}: {:?}", run.deferrals);
                assert_eq!(server.requests(), 0, "{cell}");
            }
            ("ask", true) => {
                assert!(run.body().contains("Body."), "{cell}");
                let approvals = run.approvals(&url);
                assert_eq!(approvals.len(), 1, "{cell}: {:?}", run.deferrals);
                assert_eq!(approvals[0].construct, DeferredConstruct::SchemaTemplate, "{cell}");
                assert_eq!(run.deferrals.len(), 1, "{cell}: {:?}", run.deferrals);
                assert_eq!(server.requests(), 0, "{cell}");
            }
            _ => {
                let (codes, error) = run.failure();
                assert!(codes.iter().any(|code| code == APPROVAL_REQUIRED), "{cell}: {codes:?} {error}");
                assert_eq!(server.requests(), 0, "{cell}");
            }
        }
        if dry_run || setting != "allow" {
            assert!(!fixture.cache().exists(), "{cell}: the remote cache was written");
        }
    }
}
