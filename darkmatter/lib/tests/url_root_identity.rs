//! AC30 for a URL root: the request composes the text its caller already
//! fetched and never fetches the root again, even though the same request's
//! remote policy allows the host (a remote child is fetched through it to
//! prove the policy is live). The document fields follow the source-kind
//! contract: `ctx.self`, `ctx.last_updated`, and `ctx.hash` are null, while
//! `ctx.id` / `ctx.sid` are drawn for the supplied source and shared with the
//! remote child.
//!
//! `md compose` accepts a path or stdin, so the library entry point is the
//! only boundary a URL root crosses.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::remote::RemoteReadConfig;
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const IDENTITY: &str = "{{ ctx.id }}|{{ ctx.sid }}|{{ ctx.self }}|{{ ctx.last_updated }}|{{ ctx.hash }}";

/// The `name=[value]` probe on the composed text.
fn probe(content: &str, name: &str) -> String {
    let start = content
        .find(&format!("{name}=["))
        .unwrap_or_else(|| panic!("no `{name}` probe in {content}"))
        + name.len()
        + 2;
    let end = start + content[start..].find(']').expect("probe closes");
    content[start..end].to_string()
}

fn is_lowercase_hex(value: &str, len: usize) -> bool {
    value.len() == len && value.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_url_root_composes_from_the_fetched_text_without_refetching_it() {
    let server = MockServer::start().await;
    let root_url = format!("{}/docs/root.md", server.uri());
    let child_url = format!("{}/docs/child.md", server.uri());
    let root_body = format!("root=[{IDENTITY}]\n\n::file {child_url}\n");
    Mock::given(method("GET"))
        .and(path("/docs/root.md"))
        .respond_with(ResponseTemplate::new(200).set_body_string(root_body.clone()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/child.md"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!("child=[{IDENTITY}]\n")))
        .mount(&server)
        .await;

    // The caller fetches the root once; composition receives the text.
    let fetched = reqwest::get(&root_url).await.unwrap().text().await.unwrap();
    assert_eq!(fetched, root_body);

    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Interpolation, ComposeOperation::BlockTransclusion])
        .with_source_url(root_url.parse().unwrap())
        .with_allow_remote_transclusion(true)
        .with_remote_read_config(RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        });
    let document = Markdown::from(fetched.as_str());
    let (composed, _) = tokio::task::spawn_blocking(move || document.compose_with(options))
        .await
        .unwrap()
        .expect("the URL root composes");
    let text = composed.content().to_string();

    let paths: Vec<String> = server
        .received_requests()
        .await
        .expect("recorded requests")
        .iter()
        .map(|request| request.url.path().to_string())
        .collect();
    assert_eq!(
        paths.iter().filter(|path| *path == "/docs/root.md").count(),
        1,
        "the root was fetched by the caller alone: {paths:?}"
    );
    assert_eq!(
        paths.iter().filter(|path| *path == "/docs/child.md").count(),
        1,
        "the remote child proves the policy allowed a fetch: {paths:?}"
    );
    assert_eq!(paths.len(), 2, "{paths:?}");

    let root: Vec<String> = probe(&text, "root").split('|').map(str::to_string).collect();
    let [id, sid, self_path, last_updated, hash] = root.as_slice() else {
        panic!("five identity fields: {root:?}");
    };
    assert!(is_lowercase_hex(id, 16), "ctx.id: {id}");
    assert!(is_lowercase_hex(sid, 64), "ctx.sid: {sid}");
    assert_ne!(id.as_str(), &sid[..16], "ctx.sid is not a re-spelling of ctx.id");
    assert_eq!(
        [self_path.as_str(), last_updated.as_str(), hash.as_str()],
        ["", "", ""],
        "a URL root has no native path, modification time, or on-disk hash: {text}"
    );
    assert_eq!(probe(&text, "child"), probe(&text, "root"), "the remote child composes under the root's identity: {text}");
}
