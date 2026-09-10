#![allow(dead_code)]

pub mod fixture;
pub mod protected_env;

// Re-exported so a call site keeps saying `common::CliProcessFixture`; the
// definitions live in their own file so this shared module stays near its
// current size (the area's 500-line soft-cap report flags `cli/tests`). Not
// every including binary uses every name yet (Phase 6 migrates callers).
#[allow(unused_imports)]
pub use fixture::{
    ChildEnvironment, CliProcessFixture, ConfigurableCommand, EnvironmentOp, GIT_PLUMBING_VARS,
    MdCommandBuilder, checkout_containment_error, copy_tree, git, helper_command,
    minimal_system_path, write, write_executable,
};
#[allow(unused_imports)]
pub use protected_env::{ProtectedClass, protected_class};

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub fn md_file(content: &str) -> tempfile::NamedTempFile {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "{}", content).unwrap();
    tmp
}

pub struct MockHttpResponse {
    pub status: u16,
    pub body: &'static str,
    pub cache_control: Option<&'static str>,
}

pub struct MockHttpServer {
    base_url: String,
    requests: Arc<AtomicUsize>,
    request_messages: Arc<Mutex<Vec<String>>>,
    shutdown: Option<mpsc::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

/// The worker checks for shutdown every 10 ms. The larger public bound allows
/// one in-flight request read to hit its 1 s I/O timeout before the worker
/// joins on a contended CI host.
pub const MOCK_HTTP_SHUTDOWN_BOUND: Duration = Duration::from_secs(2);

impl MockHttpServer {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    pub fn requests(&self) -> Vec<String> {
        self.request_messages.lock().unwrap().clone()
    }

    pub fn shutdown(mut self) {
        self.stop_worker().expect("mock HTTP worker panicked");
    }

    fn stop_worker(&mut self) -> thread::Result<()> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(worker) = self.worker.take() {
            worker.join()?;
        }
        Ok(())
    }
}

impl Drop for MockHttpServer {
    fn drop(&mut self) {
        let _ = self.stop_worker();
    }
}

pub fn mock_http_server(responses: Vec<MockHttpResponse>) -> MockHttpServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let request_count = Arc::clone(&requests);
    let request_messages = Arc::new(Mutex::new(Vec::new()));
    let captured_requests = Arc::clone(&request_messages);
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    listener.set_nonblocking(true).unwrap();

    let worker = thread::spawn(move || {
        let mut responses = VecDeque::from(responses);
        while !responses.is_empty() {
            if shutdown_rx.try_recv().is_ok() {
                break;
            }
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(_) => break,
            };
            let response = responses.pop_front().unwrap();
            // Accepted sockets can inherit the listener's nonblocking mode.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();

            let mut request = Vec::new();
            let mut buf = [0_u8; 1024];
            while request.len() < 16 * 1024 {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(read) => {
                        request.extend_from_slice(&buf[..read]);
                        if request.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            captured_requests
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&request).into_owned());
            request_count.fetch_add(1, Ordering::SeqCst);

            let status_text = match response.status {
                200 => "OK",
                304 => "Not Modified",
                500 => "Internal Server Error",
                _ => "OK",
            };
            let mut headers = format!(
                "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n",
                response.status,
                status_text,
                response.body.len()
            );
            if let Some(cache_control) = response.cache_control {
                headers.push_str(&format!("Cache-Control: {cache_control}\r\n"));
            }
            headers.push_str("\r\n");
            let _ = stream.write_all(headers.as_bytes());
            let _ = stream.write_all(response.body.as_bytes());
        }
    });

    MockHttpServer {
        base_url: format!("http://{addr}"),
        requests,
        request_messages,
        shutdown: Some(shutdown_tx),
        worker: Some(worker),
    }
}

/// Baseline JSON fixture comparison helpers.
///
/// These helpers load the byte-for-byte reference JSON fixtures captured
/// for `md validate refs --json` and `md graph --json` (the public
/// library serde shape pinned during the CLI Atheist feature) and
/// normalize away environment-specific values (temp directory paths and
/// the FNV-1a reference-id hashes that derive from them) so tests can
/// compare the structure of CLI JSON output against the captured
/// baselines without depending on a specific temp-directory location.
///
/// `md validate refs --json` and `md graph --json` share a single
/// library serde contract: `ReferenceValidationReport` serializes via
/// its `Serialize` impl (review-2 finding #2 closed the last
/// CLI-local JSON drift). The baseline fixtures pin that contract.
pub mod baseline {
    use std::path::PathBuf;

    /// Returns the directory holding the baseline JSON fixtures.
    pub fn dir() -> PathBuf {
        let features_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("features");
        let active = features_dir
            .join("2026-06-17-cli-atheist")
            .join("baseline")
            .join("json");
        if active.exists() {
            return active;
        }

        features_dir
            .join("_completed")
            .join("2026-06-17-cli-atheist")
            .join("baseline")
            .join("json")
    }

    /// Loads and parses a baseline JSON fixture by filename (e.g.
    /// `"validate_refs_local.json"`).
    pub fn load_json(name: &str) -> serde_json::Value {
        let path = dir().join(name);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read baseline {}: {}", path.display(), e));
        serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("failed to parse baseline {}: {}", path.display(), e))
    }

    /// Normalizes a JSON value for byte-for-byte comparison by replacing
    /// environment-specific values with stable placeholders.
    ///
    /// - Every string in `paths_to_redact` is replaced with `<TMP>` (used
    ///   for both the raw temp-dir path and its canonicalized form).
    /// - Any `reference_id`-shaped string (`<16-hex>:<digits>:<digits>`)
    ///   has its hash prefix replaced with `<HASH>` so that
    ///   path-dependent FNV-1a hashes do not affect the comparison.
    pub fn normalize(value: serde_json::Value, paths_to_redact: &[&str]) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                serde_json::Value::String(normalize_string(&s, paths_to_redact))
            }
            serde_json::Value::Array(items) => serde_json::Value::Array(
                items
                    .into_iter()
                    .map(|v| normalize(v, paths_to_redact))
                    .collect(),
            ),
            serde_json::Value::Object(map) => serde_json::Value::Object(
                map.into_iter()
                    .map(|(k, v)| (k, normalize(v, paths_to_redact)))
                    .collect(),
            ),
            other => other,
        }
    }

    fn normalize_string(s: &str, paths_to_redact: &[&str]) -> String {
        // Reference IDs have the shape `<16-hex>:<digits>:<digits>`. The
        // hex prefix is the FNV-1a hash of the source-file path, which is
        // not stable across temp directories.
        let mut parts = s.splitn(3, ':');
        let (first, second, third) = (parts.next(), parts.next(), parts.next());
        if let (Some(first), Some(second), Some(third)) = (first, second, third)
            && first.len() == 16
            && !first.is_empty()
            && first.bytes().all(|b| b.is_ascii_hexdigit())
            && !second.is_empty()
            && second.bytes().all(|b| b.is_ascii_digit())
            && !third.is_empty()
            && third.bytes().all(|b| b.is_ascii_digit())
        {
            return format!("<HASH>:{second}:{third}");
        }

        let mut result = s.to_string();
        for path in paths_to_redact {
            if !path.is_empty() {
                result = result.replace(path, "<TMP>");
            }
        }
        if result.contains("<TMP>") {
            result = result.replace('\\', "/");
        }
        result
    }

    /// Produces the path strings that must be redacted when comparing
    /// against a baseline: both the raw temp dir and its canonicalized
    /// form (macOS resolves `/tmp` → `/private/tmp`), plus the original
    /// baseline paths used during fixture capture.
    pub fn paths_to_redact(temp_dir: &std::path::Path) -> Vec<String> {
        let mut paths = Vec::new();
        let raw = temp_dir.to_string_lossy().to_string();
        if !raw.is_empty() {
            paths.push(raw);
        }
        if let Ok(canon) = std::fs::canonicalize(temp_dir) {
            let canon_str = canon.to_string_lossy().to_string();
            if !canon_str.is_empty() {
                paths.push(canon_str);
            }
        }
        // The baseline fixtures were captured under `/tmp/dm-baseline`
        // (and its macOS canonical form `/private/tmp/dm-baseline`).
        // Redact both so the baseline side normalizes to `<TMP>` too.
        paths.push("/tmp/dm-baseline".to_string());
        paths.push("/private/tmp/dm-baseline".to_string());

        // Redact longest paths first. `normalize_string` replaces substrings in
        // order, so a shorter path that is a prefix of a longer one (e.g.
        // `/tmp/dm-baseline` inside `/private/tmp/dm-baseline`, or a raw temp
        // dir inside its `/private`-canonicalized form) would otherwise match
        // first and leave a stray `/private` prefix. That artifact happens to
        // align with macOS's canonicalized temp paths, so the mismatch is
        // invisible on macOS but fails on Linux (`<TMP>/x` vs `/private<TMP>/x`).
        paths.sort_by_key(|p| std::cmp::Reverse(p.len()));
        paths
    }
}

pub mod layout {
    use biscuit_terminal::terminal::Terminal;
    use clap::Parser;
    use darkmatter::layout::{DarkmatterPage, PageComponent};
    use darkmatter_cli::Cli;
    use darkmatter_cli::render::apply_cli_layout_flags;
    use renderable::layout::{Alignment, Edges, Length, TargetValue, Width};

    pub fn parse_cli(args: &[&str]) -> Cli {
        let mut full = vec!["md"];
        full.extend_from_slice(args);
        Cli::try_parse_from(full).expect("CLI args must parse")
    }

    pub fn resolved_page(args: &[&str]) -> DarkmatterPage {
        let cli = parse_cli(args);
        let term = Terminal::new_optimistic(120);
        apply_cli_layout_flags(DarkmatterPage::new(&term), &cli)
    }

    pub fn tv_cells(tv: &TargetValue<Length>) -> u16 {
        match tv {
            TargetValue::Universal(Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
            _ => 0,
        }
    }

    pub fn alignment_for(page: &DarkmatterPage, component: PageComponent) -> Alignment {
        page.component_policy(component)
            .map(|p| p.layout.alignment)
            .unwrap_or_default()
    }

    #[derive(Debug, PartialEq)]
    pub enum TestFill {
        Full,
        Pad(Length),
        Indent(Length),
        Max(Length),
        Explicit(Length),
    }

    pub fn fill_for(page: &DarkmatterPage, component: PageComponent) -> TestFill {
        match page.component_policy(component) {
            None => TestFill::Full,
            Some(p) => {
                let l = &p.layout;
                if l.width == Width::Auto && l.max_width.is_none() && l.padding == Edges::default()
                {
                    TestFill::Full
                } else if l.width == Width::Auto
                    && l.max_width.is_none()
                    && l.padding != Edges::default()
                {
                    if l.padding.top == TargetValue::universal(Length::Zero)
                        && l.padding.bottom == TargetValue::universal(Length::Zero)
                        && l.padding.left == l.padding.right
                    {
                        TestFill::Pad(tv_length(&l.padding.left))
                    } else {
                        TestFill::Indent(tv_length(&l.padding.left))
                    }
                } else if let Some(max_width) = &l.max_width
                    && l.width == Width::Auto
                {
                    TestFill::Max(tv_length(max_width))
                } else if matches!(l.width, Width::Fixed(_)) {
                    TestFill::Explicit(width_length(&l.width))
                } else {
                    TestFill::Full
                }
            }
        }
    }

    pub fn tv_length(tv: &TargetValue<Length>) -> Length {
        match tv {
            TargetValue::Universal(l) => l.clone(),
            _ => Length::Zero,
        }
    }

    pub fn width_length(w: &Width) -> Length {
        match w {
            Width::Fixed(tv) => tv_length(tv),
            _ => Length::Zero,
        }
    }

    pub fn style_prop_fixture() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("example-docs")
            .join("rendering")
            .join("style-prop.md")
    }
}

// Only the `level2_*` targets (gated on `terminal-tests`) use this module, and it
// depends on the optional `biscuit-test-harness` crate.
#[cfg(feature = "terminal-tests")]
pub mod level2;
