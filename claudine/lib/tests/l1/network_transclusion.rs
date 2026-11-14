//! Claudine's preparation transcludes URL targets under the captured
//! `network` permission, as `md compose` does: `network: allow` composes a
//! remote `::file`, `::code`, `prologue`, and `epilogue`; the default (`deny`)
//! refuses them without contacting the host; consent never follows a
//! redirect; and shell discovery walks a remote child only when the host is
//! allowed.
//!
//! Every case works in its own temp folder with a temp home, prepares
//! through Claudine's public direct-preparation API, and fetches only from a
//! loopback responder.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use claudine::composition::{
    CompositionError, PrepareOptions, ResolvedCompositionSource, ShellApprovalFailure,
    approve_document_shell, prepare_direct, prepare_inline,
};
use claudine::harness::{CapturedPermissions, ShellApprovalOptions};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};

const CHILD_TEXT: &str = "Remote child.";
const CODE_TEXT: &str = "fn remote() {}";
const PROLOGUE_TEXT: &str = "Remote prologue.";
const EPILOGUE_TEXT: &str = "Remote epilogue.";

/// A loopback HTTP responder that serves fixed routes and counts every
/// request by path, for the rest of the process.
struct Responder {
    base: String,
    hits: Arc<Mutex<HashMap<String, usize>>>,
}

impl Responder {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let base = format!("http://127.0.0.1:{}", listener.local_addr().expect("address").port());
        let hits = Arc::new(Mutex::new(HashMap::new()));
        let (counter, location) = (Arc::clone(&hits), format!("{base}/final.md"));
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
                let response = match route.as_str() {
                    "/hop.md" => {
                        format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    }
                    _ => {
                        let body = match route.as_str() {
                            "/child.md" | "/final.md" => CHILD_TEXT,
                            "/snippet.rs" => CODE_TEXT,
                            "/prologue.md" => PROLOGUE_TEXT,
                            "/epilogue.md" => EPILOGUE_TEXT,
                            "/commands.md" => "::shell echo remote\n",
                            "/fails.md" => "::shell mkdir taken\n",
                            "/bad-block.md" => "::shell-block timeout=bad\nmkdir taken\n::end-block\n",
                            "/unclosed.md" => "::block when=true\nkept\n",
                            "/collection.md" => "::map other \"o => o\"\n- x\n::end-map\n",
                            _ => "",
                        };
                        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                    }
                };
                let _ = (&stream).write_all(response.as_bytes());
            }
        });
        Self { base, hits }
    }

    fn url(&self, route: &str) -> String {
        format!("{}{route}", self.base)
    }

    fn hits(&self, route: &str) -> usize {
        self.hits.lock().unwrap().get(route).copied().unwrap_or(0)
    }
}

/// A home whose `~/.darkmatter.yaml` says `network: <network>` and a work
/// folder beside it.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new(network: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        std::fs::create_dir_all(root.join("work")).unwrap();
        std::fs::write(root.join("home/.darkmatter.yaml"), format!("permissions:\n  network: {network}\n")).unwrap();
        Self { _dir: dir, root }
    }

    /// `text` written as `work/doc.md`, with the request and preparation
    /// options anchored in the work folder.
    fn source(&self, text: &str) -> (ResolvedCompositionSource, ComposeRequest, PrepareOptions) {
        let path = self.root.join("work/doc.md");
        std::fs::write(&path, text).unwrap();
        let snapshot = RequestSnapshot::new(self.root.join("work")).with_home(Some(self.root.join("home")));
        let request =
            ComposeRequest::prepare(ComposeOptions::new().with_source_file(&path), &snapshot).expect("a request");
        let source = ResolvedCompositionSource {
            original_ref: path.display().to_string(),
            resolved_path: path.clone(),
            original_text: text.to_string(),
            markdown: Markdown::try_from(path.as_path()).unwrap(),
        };
        let options = PrepareOptions::new(request.resolution_context().clone());
        (source, request, options)
    }

    /// The prepared prompt, or the error's whole cause chain, lowercased.
    fn prepare(&self, text: &str) -> Result<String, String> {
        let (source, _, options) = self.source(text);
        prepare_direct(&source, options).map(|prepared| prepared.prompt).map_err(|error| {
            let mut chain = String::new();
            let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&error);
            while let Some(error) = current {
                chain.push_str(&error.to_string());
                chain.push('\n');
                current = error.source();
            }
            chain.to_ascii_lowercase()
        })
    }
}

fn prepared(result: Result<String, String>) -> String {
    result.unwrap_or_else(|failure| panic!("preparation failed: {failure}"))
}

fn refusal(result: Result<String, String>) -> String {
    result.expect_err("preparation refused the remote target")
}

#[test]
fn network_allow_transcludes_a_remote_file() {
    let server = Responder::start();
    let fixture = Fixture::new("allow");

    let prompt = prepared(fixture.prepare(&format!("::file {}\n", server.url("/child.md"))));

    assert!(prompt.contains(CHILD_TEXT), "{prompt}");
}

#[test]
fn network_allow_transcludes_a_remote_code_block() {
    let server = Responder::start();
    let fixture = Fixture::new("allow");

    let prompt = prepared(fixture.prepare(&format!("::code {}\n", server.url("/snippet.rs"))));

    assert!(prompt.contains(&format!("```rs\n{CODE_TEXT}")), "{prompt}");
}

#[test]
fn network_allow_transcludes_a_remote_prologue() {
    let server = Responder::start();
    let fixture = Fixture::new("allow");

    let prompt = prepared(fixture.prepare(&format!("---\nprologue: {}\n---\nBody.\n", server.url("/prologue.md"))));

    let (prologue, body) = (prompt.find(PROLOGUE_TEXT), prompt.find("Body."));
    assert!(prologue.is_some() && prologue < body, "{prompt}");
}

#[test]
fn network_allow_transcludes_a_remote_epilogue() {
    let server = Responder::start();
    let fixture = Fixture::new("allow");

    let prompt = prepared(fixture.prepare(&format!("---\nepilogue: {}\n---\nBody.\n", server.url("/epilogue.md"))));

    let (body, epilogue) = (prompt.find("Body."), prompt.find(EPILOGUE_TEXT));
    assert!(body.is_some() && body < epilogue, "{prompt}");
}

#[test]
fn the_default_refuses_every_remote_target_without_contacting_the_host() {
    let server = Responder::start();
    let fixture = Fixture::new("deny");
    let documents = [
        ("/child.md", format!("::file {}\n", server.url("/child.md"))),
        ("/snippet.rs", format!("::code {}\n", server.url("/snippet.rs"))),
        ("/prologue.md", format!("---\nprologue: {}\n---\nBody.\n", server.url("/prologue.md"))),
        ("/epilogue.md", format!("---\nepilogue: {}\n---\nBody.\n", server.url("/epilogue.md"))),
    ];

    for (route, document) in documents {
        let failure = refusal(fixture.prepare(&document));

        assert!(failure.contains("denied for host '127.0.0.1'"), "{route}: {failure}");
        assert_eq!(server.hits(route), 0, "{route}: a denied host is never contacted");
    }
}

#[test]
fn network_allow_still_refuses_a_redirect() {
    let server = Responder::start();
    let fixture = Fixture::new("allow");

    let failure = refusal(fixture.prepare(&format!("::file {}\n", server.url("/hop.md"))));

    assert!(failure.contains("redirect"), "{failure}");
    assert_eq!(server.hits("/final.md"), 0, "no redirect was followed");
}

/// Shell discovery walks into a remote child only when the host is allowed:
/// under `allow` it finds the child's command (which then needs approval);
/// under `deny` it never contacts the host.
#[test]
fn shell_discovery_reads_a_remote_child_only_under_network_allow() {
    let server = Responder::start();
    let document = format!("::file {}\n", server.url("/commands.md"));

    for network in ["allow", "deny"] {
        let fixture = Fixture::new(network);
        let (source, request, options) = fixture.source(&document);
        let approval = ShellApprovalOptions {
            permissions: CapturedPermissions::of_snapshot(request.config_snapshot()),
            ..Default::default()
        };

        let result = approve_document_shell(&source, &options, &approval, claudine::composition::CompositionMode::ChainedDocument);

        match network {
            "allow" => assert!(
                matches!(
                    result,
                    Err(CompositionError::ShellApprovalUnavailable { failure: ShellApprovalFailure::NoHandler, .. })
                ),
                "the remote child's command needs approval: {result:?}"
            ),
            _ => {
                assert!(result.is_err(), "a denied remote child is not walked: {result:?}");
                assert_eq!(server.hits("/commands.md"), 1, "only the allowed walk contacted the host");
            }
        }
    }
}

/// An inline run reports its `prompt`'s own failures at the source file's
/// `prompt` property. A remote child's context has no file path either, but
/// its failure is the child's: it keeps the child's text and line and is
/// never moved onto the source file's `prompt`.
#[test]
fn an_inline_prompt_keeps_a_remote_childs_shell_failure_its_own() {
    let server = Responder::start();
    let fixture = Fixture::new("allow");
    // `mkdir` of an existing directory fails under `sh` and `cmd` alike.
    std::fs::create_dir_all(fixture.root.join("work/taken")).unwrap();
    let text = format!("---\nagent: goose\nprompt: |\n  Update the body.\n\n  ::file {}\n---\n# Body\n", server.url("/fails.md"));
    let (source, _, mut options) = fixture.source(&text);
    options.pre_approved_commands = Some(std::collections::HashSet::from(["mkdir taken".to_string()]).into());

    let error = prepare_inline(&source, options).expect_err("the remote child's command fails");

    let CompositionError::ShellExpansionFailed { error: shell, .. } = &error else {
        panic!("expected a shell failure, got {error:?}");
    };
    assert_eq!(shell.origin(), Some(&darkmatter::markdown::compose::ShellCommandOrigin::Body { line: 1 }), "{shell:?}");
    let context = shell.context().expect("a located failure");
    assert_ne!(context.absolute, source.resolved_path, "{context:?}");
    assert_eq!(&*context.content, "::shell mkdir taken\n", "the child's own text: {context:?}");
}

/// A remote child's block errors are its own too: an inline run of a prompt
/// transcluding it never reports them at the source's `prompt`.
#[test]
fn an_inline_prompt_keeps_a_remote_childs_block_errors_its_own() {
    let server = Responder::start();
    let fixture = Fixture::new("allow");
    for route in ["/bad-block.md", "/unclosed.md"] {
        let text = format!("---\nagent: goose\nprompt: |\n  Update the body.\n\n  ::file {}\n---\n# Body\n", server.url(route));
        let (source, _, options) = fixture.source(&text);

        // Whether the child's error stops the run or is tolerated as a failed
        // transclusion, it is never reported at the source's `prompt`.
        match prepare_inline(&source, options) {
            Err(error @ CompositionError::InlinePromptDirectiveFailed { .. }) => {
                panic!("{route}: a remote child's error was moved to the prompt: {error:?}")
            }
            Err(error) => assert!(!format!("{error:?}").contains("<prompt of"), "{route}: {error:?}"),
            Ok(prepared) => assert!(prepared.prompt.contains("Could not transclude"), "{route}: {}", prepared.prompt),
        }
    }
}

/// A remote child's `::map` failure is its own: Darkmatter names it with its
/// `document` placeholder, never the inline run's prompt identity, so it is
/// never reported at the source's `prompt`.
#[test]
fn an_inline_prompt_keeps_a_remote_childs_collection_failure_its_own() {
    let server = Responder::start();
    let fixture = Fixture::new("allow");
    let text = format!(
        "---\nagent: goose\nprompt: |\n  Update the body.\n\n  ::file {}\n---\n# Body\n",
        server.url("/collection.md")
    );
    let (source, _, options) = fixture.source(&text);

    let error = prepare_inline(&source, options).expect_err("the remote child's collection fails");

    let CompositionError::ComposeFailed(darkmatter::markdown::MarkdownError::Collection(collection)) = &error else {
        panic!("expected the child's own collection failure, got {error:?}");
    };
    let site = collection.site().expect("a located failure");
    let darkmatter::markdown::SourceRef::OnDiskSpan { context, span } = site.source.as_ref() else {
        panic!("expected the child's own span, got {site:?}");
    };
    assert_eq!(context.absolute, std::path::Path::new("document"), "{context:?}");
    assert_eq!(&*context.content, "::map other \"o => o\"\n- x\n::end-map\n", "the child's own text");
    assert_eq!(span.line(), 1, "the child's own line");
}
