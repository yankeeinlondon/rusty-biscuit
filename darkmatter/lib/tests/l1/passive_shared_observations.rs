//! Dry-run compositions whose sessions share one
//! [`PassiveObservations`](darkmatter::markdown::compose::passive::PassiveObservations)
//! observe each source once (L1): a host checking one invocation in several
//! passes gives each pass [`PassiveSession::sharing_observations`], and a URL
//! or file they all read is read once. Sharing never widens a read: a pass
//! whose `network` setting refuses a host gets its refusal, with no request,
//! even after another pass fetched that URL. Sessions that share nothing
//! each read for themselves.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::passive::PassiveSession;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};

use crate::network_approval::Responder;

/// A home whose `.darkmatter.yaml` sets `network`, and a work folder
/// beside it.
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

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join("work").join(name);
        std::fs::write(&path, text).expect("write");
        path
    }

    /// `path` composed as a dry run recording into `session`: the body, or
    /// the failure's text.
    fn compose(&self, path: &Path, session: PassiveSession) -> Result<String, String> {
        let snapshot = RequestSnapshot::new(self.root.join("work")).with_home(Some(self.root.join("home")));
        let options = ComposeOptions::new()
            .with_source_file(path)
            .with_allow_remote_transclusion(true)
            .with_passive_session(session);
        let request = ComposeRequest::prepare(options, &snapshot).expect("a request");
        Markdown::try_from(path)
            .expect("markdown")
            .compose_with(&request)
            .map(|(composed, _)| composed.content().to_string())
            .map_err(|error| error.to_string())
    }
}

/// A document reading `server`'s data, child, and schema slot.
fn every_reader(home: &Home, server: &Responder) -> PathBuf {
    home.write(
        "schema.yaml",
        &format!("kind: schema\n$schema:\n    os: \"enum({{{{ load_data(\\\"{}\\\", \\\"string[]\\\") }}}})\"\n", server.url("/os.yaml")),
    );
    home.write(
        "doc.md",
        &format!(
            "---\n$schema: ./schema.yaml\nos: macOS\n---\nData {{{{ load_data('{}', 'string[]') }}}}.\n\n::file {}\n",
            server.url("/data.json"),
            server.url("/child.md"),
        ),
    )
}

#[test]
fn passes_sharing_observations_fetch_each_url_once() {
    let server = Responder::start();
    let home = Home::new("allow");
    let doc = every_reader(&home, &server);
    let first = PassiveSession::new();

    for session in [first.clone(), first.sharing_observations(), first.sharing_observations()] {
        let body = home.compose(&doc, session).expect("composed");
        assert!(body.contains("Data [\"a\",\"b\"].") && body.contains("Remote child."), "{body}");
    }

    assert_eq!(server.requests(), 3, "one request for each of the three URLs");
}

#[test]
fn sessions_sharing_nothing_each_fetch() {
    let server = Responder::start();
    let home = Home::new("allow");
    let doc = every_reader(&home, &server);

    for _ in 0..2 {
        home.compose(&doc, PassiveSession::new()).expect("composed");
    }

    assert_eq!(server.requests(), 6);
}

/// A URL one pass fetched under `allow` is not served to a pass whose
/// configuration says `deny`: that pass gets its refusal and sends nothing.
#[test]
fn a_shared_fetch_never_serves_a_pass_whose_policy_refuses_the_host() {
    let server = Responder::start();
    let allowed = Home::new("allow");
    let denied = Home::new("deny");
    let text = format!("Data {{{{ load_data('{}', 'string[]') }}}}.\n", server.url("/data.json"));
    let first = PassiveSession::new();

    let fetched = allowed.compose(&allowed.write("doc.md", &text), first.clone()).expect("composed");
    let refused = denied.compose(&denied.write("doc.md", &text), first.sharing_observations());

    assert!(fetched.contains("Data [\"a\",\"b\"]."), "{fetched}");
    let error = refused.expect_err("the deny pass refuses the read");
    assert!(error.contains("remote read denied for host '127.0.0.1'"), "{error}");
    assert_eq!(server.requests(), 1);
}

/// A local file is read once by passes sharing observations.
#[test]
fn passes_sharing_observations_read_a_local_file_once() {
    let home = Home::new("deny");
    home.write("rows.json", "[\"a\", \"b\"]");
    let doc = home.write("doc.md", "Data {{ load_data('./rows.json', 'string[]') }}.\n");
    let first = PassiveSession::new();

    for session in [first.clone(), first.sharing_observations()] {
        assert!(home.compose(&doc, session).expect("composed").contains("Data [\"a\",\"b\"]."));
    }

    assert_eq!(first.observations().data().stats().reads, 1);
    assert!(first.observations().same_observations(first.sharing_observations().observations()));
}

/// The same for a schema slot: the slot gate of the `deny` pass refuses
/// the URL before the shared runtime is consulted.
#[test]
fn a_shared_slot_fetch_never_serves_a_pass_whose_policy_refuses_the_host() {
    let server = Responder::start();
    let allowed = Home::new("allow");
    let denied = Home::new("deny");
    let first = PassiveSession::new();
    let slot = |home: &Home| {
        home.write(
            "schema.yaml",
            &format!("kind: schema\n$schema:\n    os: \"enum({{{{ load_data(\\\"{}\\\", \\\"string[]\\\") }}}})\"\n", server.url("/os.yaml")),
        );
        home.write("doc.md", "---\n$schema: ./schema.yaml\nos: macOS\n---\nBody.\n")
    };

    allowed.compose(&slot(&allowed), first.clone()).expect("composed");
    let refused = denied.compose(&slot(&denied), first.sharing_observations());

    let error = refused.expect_err("the deny pass refuses the slot read");
    assert!(error.contains("deny"), "{error}");
    assert_eq!(server.requests(), 1);
}

/// How many times each of `names` in `home`'s work folder is opened while
/// `run` runs.
#[cfg(feature = "work-counters")]
fn opens_during(home: &Home, names: &[&str], run: impl FnOnce()) -> Vec<usize> {
    let opens = |name: &str| simplified_schema::work_counters::source_opens(&home.root.join("work").join(name));
    let before: Vec<usize> = names.iter().map(|name| opens(name)).collect();
    run();
    names.iter().zip(before).map(|(name, before)| opens(name) - before).collect()
}

/// A document reading every kind of local source: a `$schema` file, a
/// `prologue`/`epilogue`, a `::file` body inclusion, and a `::code` target.
#[cfg(feature = "work-counters")]
fn every_local_reader(home: &Home, name: &str, include: &str) -> PathBuf {
    home.write("s.yaml", "$schema:\n  topic: string\n");
    home.write("pro.md", "Prologue.\n");
    home.write("epi.md", "Epilogue.\n");
    home.write("inc.md", "Included.\n");
    home.write("other.md", "Other.\n");
    home.write("snippet.txt", "let x = 1;\n");
    home.write(
        name,
        &format!(
            "---\n$schema: ./s.yaml\ntopic: hello\nprologue: ./pro.md\nepilogue: ./epi.md\n---\nAbout {{{{ topic }}}}.\n\n::file ./{include}\n\n::code ./snippet.txt\n"
        ),
    )
}

#[cfg(feature = "work-counters")]
const LOCAL_SOURCES: [&str; 5] = ["s.yaml", "pro.md", "epi.md", "inc.md", "snippet.txt"];

/// R3-F4.1–.3: passes sharing observations open each local source once, and
/// every pass still composes it with its own values.
#[cfg(feature = "work-counters")]
#[test]
fn passes_sharing_observations_open_each_local_source_once() {
    let home = Home::new("deny");
    let doc = every_local_reader(&home, "doc.md", "inc.md");
    let first = PassiveSession::new();

    let opens = opens_during(&home, &LOCAL_SOURCES, || {
        for session in [first.clone(), first.sharing_observations(), first.sharing_observations()] {
            let body = home.compose(&doc, session).expect("composed");
            for text in ["Prologue.", "About hello.", "Included.", "let x = 1;", "Epilogue."] {
                assert!(body.contains(text), "{text}: {body}");
            }
        }
    });

    assert_eq!(opens, [1; 5], "{LOCAL_SOURCES:?}");
}

/// Sessions that share nothing each observe the sources for themselves:
/// one open per composition.
#[cfg(feature = "work-counters")]
#[test]
fn sessions_sharing_nothing_each_open_the_local_sources() {
    let home = Home::new("deny");
    let doc = every_local_reader(&home, "doc.md", "inc.md");

    let opens = opens_during(&home, &LOCAL_SOURCES, || {
        for _ in 0..2 {
            home.compose(&doc, PassiveSession::new()).expect("composed");
        }
    });

    assert_eq!(opens, [2; 5], "{LOCAL_SOURCES:?}");
}

/// A different inclusion is a different source: each is opened once, and
/// each document composes its own.
#[cfg(feature = "work-counters")]
#[test]
fn distinct_local_sources_are_each_opened_once() {
    let home = Home::new("deny");
    let first_doc = every_local_reader(&home, "first.md", "inc.md");
    let second_doc = every_local_reader(&home, "second.md", "other.md");
    let first = PassiveSession::new();

    let opens = opens_during(&home, &["inc.md", "other.md", "s.yaml"], || {
        let one = home.compose(&first_doc, first.clone()).expect("composed");
        let two = home.compose(&second_doc, first.sharing_observations()).expect("composed");
        assert!(one.contains("Included.") && !one.contains("Other."), "{one}");
        assert!(two.contains("Other.") && !two.contains("Included."), "{two}");
    });

    assert_eq!(opens, [1, 1, 1]);
}
