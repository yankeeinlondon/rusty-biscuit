//! A dry run composes passively (`ComposeOptions::with_dry_run`): nothing
//! with an effect happens, the work that needs one keeps its authored text,
//! and everything independent of it still composes. Each test proves one
//! effect never happened (a marker file, a request count, a cache folder)
//! and that its control, the same document composed for real, does it.

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::passive::{Deferral, DeferralReason, DeferredConstruct};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ShellExpansionError};
use darkmatter::markdown::{Markdown, MarkdownError};
use tempfile::TempDir;

/// What a dry run composed and what it deferred.
struct DryRun {
    body: String,
    frontmatter: serde_json::Value,
    deferrals: Vec<Deferral>,
}

fn options_in(dir: &Path, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_config_snapshot(crate::request_support::shell_config(dir, &["prefix touch", "prefix echo"]))
        .with_shell_working_directory(dir)
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

fn compose(path: &Path, options: ComposeOptions) -> Result<Markdown, MarkdownError> {
    Markdown::try_from(path).unwrap().compose_with(&crate::request_support::request(options)).map(|(composed, _)| composed)
}

fn dry_run(dir: &Path, text: &str) -> DryRun {
    let path = write(dir, "doc.md", text);
    let options = options_in(dir, &path).with_dry_run();
    let session = options.passive_session().cloned().expect("a dry run has a session");
    let composed = compose(&path, options).unwrap_or_else(|error| panic!("the dry run failed: {error}"));
    DryRun {
        body: composed.content().to_string(),
        frontmatter: serde_json::Value::Object(composed.frontmatter().as_map().iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        deferrals: session.deferrals(),
    }
}

fn constructs(run: &DryRun) -> Vec<DeferredConstruct> {
    run.deferrals.iter().map(|deferral| deferral.construct).collect()
}

/// The exit-gate document: one `$( … )` value. Its own key, a key that reads
/// it, the spans that read either, and the block whose condition reads it
/// stay as written; the independent value, span, and block compose.
const ONE_COMMAND: &str = "---\nbranch: $(touch marker)\ntitle: \"Work on {{ branch }}\"\nowner: Ken\n---\nOwner {{ owner }}.\nBranch {{ branch }}.\nTitle {{ title }}.\nMember {{ fm.branch.name }}.\n\n::block when=\"branch == 'main'\"\nmain only\n::end-block\n\n::block when=\"owner == 'Ken'\"\nken only\n::end-block\n";

#[test]
fn one_command_value_defers_only_what_reads_it() {
    let dir = TempDir::new().unwrap();
    let run = dry_run(dir.path(), ONE_COMMAND);

    assert!(!dir.path().join("marker").exists(), "a dry run ran the command");
    assert!(run.body.contains("Owner Ken."), "{}", run.body);
    assert!(run.body.contains("Branch {{ branch }}."), "{}", run.body);
    assert!(run.body.contains("Title {{ title }}."), "{}", run.body);
    assert!(run.body.contains("Member {{ fm.branch.name }}."), "{}", run.body);
    assert!(run.body.contains("::block when=\"branch == 'main'\"\nmain only\n::end-block"), "{}", run.body);
    assert!(run.body.contains("ken only") && !run.body.contains("owner == 'Ken'"), "{}", run.body);
    assert_eq!(run.frontmatter["branch"], "$(touch marker)");
    assert_eq!(run.frontmatter["title"], "Work on {{ branch }}");
    assert_eq!(run.frontmatter["owner"], "Ken");

    let command = &run.deferrals[0];
    assert_eq!(command.construct, DeferredConstruct::FrontmatterValue);
    assert_eq!(command.key.as_deref(), Some("branch"));
    assert_eq!(command.reason, DeferralReason::Command { command: "touch marker".into() });
    assert_eq!(command.line, Some(2));
    assert!(
        run.deferrals.iter().any(|deferral| deferral.key.as_deref() == Some("title")
            && matches!(&deferral.reason, DeferralReason::DependsOn { name } if name == "branch")),
        "{:#?}",
        run.deferrals
    );
    let condition = run.deferrals.iter().find(|deferral| deferral.construct == DeferredConstruct::BlockCondition).unwrap();
    assert_eq!(condition.authored, "branch == 'main'");
    assert_eq!(condition.line, Some(11));
    let spans: Vec<&str> = run
        .deferrals
        .iter()
        .filter(|deferral| deferral.construct == DeferredConstruct::Interpolation)
        .map(|deferral| deferral.authored.as_str())
        .collect();
    for span in ["{{ branch }}", "{{ title }}", "{{ fm.branch.name }}"] {
        assert!(spans.contains(&span), "{span}: {spans:?}");
    }

    // Control: composed for real, the command runs and every read resolves.
    let path = dir.path().join("doc.md");
    let composed = compose(&path, options_in(dir.path(), &path)).unwrap();
    assert!(dir.path().join("marker").exists());
    assert!(composed.content().contains("Title Work on ."), "{}", composed.content());
}

#[test]
fn a_body_directive_and_a_shell_block_stay_as_written() {
    let dir = TempDir::new().unwrap();
    let text = "Before {{ 1 + 1 }}.\n\n::shell touch marker-line\n\n::shell-block\ntouch marker-block\n::end-block\n\nAfter.\n";
    let run = dry_run(dir.path(), text);
    assert!(!dir.path().join("marker-line").exists() && !dir.path().join("marker-block").exists());
    assert!(run.body.contains("Before 2."), "{}", run.body);
    assert!(run.body.contains("::shell touch marker-line"), "{}", run.body);
    assert!(run.body.contains("::shell-block\ntouch marker-block\n::end-block"), "{}", run.body);
    assert_eq!(constructs(&run), [DeferredConstruct::ShellDirective, DeferredConstruct::ShellBlock]);
    assert_eq!(run.deferrals[0].reason, DeferralReason::Command { command: "touch marker-line".into() });
    assert_eq!(run.deferrals[0].line, Some(3));
    assert_eq!(run.deferrals[1].reason, DeferralReason::Command { command: "touch marker-block".into() });
    assert_eq!(run.deferrals[1].line, Some(5));
}

#[test]
fn an_effectful_function_is_deferred_and_a_pure_one_still_evaluates() {
    let dir = TempDir::new().unwrap();
    let text = "Alias {{ has_alias(\"ll\") }}.\nPing {{ ping(\"127.0.0.1\") }}.\nUpper {{ upper(\"x\") }}.\n";
    let run = dry_run(dir.path(), text);
    assert!(run.body.contains("Alias {{ has_alias(\"ll\") }}."), "{}", run.body);
    assert!(run.body.contains("Ping {{ ping(\"127.0.0.1\") }}."), "{}", run.body);
    assert!(run.body.contains("Upper X."), "{}", run.body);
    let reasons: Vec<&DeferralReason> = run.deferrals.iter().map(|deferral| &deferral.reason).collect();
    assert_eq!(reasons.len(), 2, "{reasons:?}");
    assert!(reasons.contains(&&DeferralReason::ShellProbe { function: "has_alias".into() }), "{reasons:?}");
    assert!(reasons.contains(&&DeferralReason::NetworkProbe { function: "ping".into() }), "{reasons:?}");
}

#[test]
fn a_callback_and_a_fallback_never_turn_a_deferred_value_into_a_value() {
    let dir = TempDir::new().unwrap();
    let text = "---\nbranch: $(touch marker)\nvalues: [1, 2]\n---\nMapped {{ map(values, n => branch) }}.\nFallback {{ branch || \"none\" }}.\nDoubled {{ map(values, n => n * 2) }}.\n";
    let run = dry_run(dir.path(), text);
    assert!(!dir.path().join("marker").exists());
    assert!(run.body.contains("Mapped {{ map(values, n => branch) }}."), "{}", run.body);
    assert!(run.body.contains("Fallback {{ branch || \"none\" }}."), "{}", run.body);
    assert!(run.body.contains("Doubled [2,4]."), "{}", run.body);
}

#[test]
fn a_transcluded_child_defers_its_own_command_and_the_parent_composes() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", "---\nstamp: $(touch marker-child)\n---\nChild {{ stamp }} and {{ 2 * 3 }}.\n");
    let run = dry_run(dir.path(), "Parent.\n\n::file ./child.md\n");
    assert!(!dir.path().join("marker-child").exists());
    assert!(run.body.contains("Parent."), "{}", run.body);
    assert!(run.body.contains("Child {{ stamp }} and 6."), "{}", run.body);
    assert!(
        run.deferrals.iter().any(|deferral| deferral.key.as_deref() == Some("stamp")),
        "{:#?}",
        run.deferrals
    );
}

#[test]
fn a_blacklisted_command_is_still_refused() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\nvalue: $(rm -rf /)\n---\nbody\n");
    let error = compose(&path, options_in(dir.path(), &path).with_dry_run()).expect_err("the blacklist refuses");
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    let mut blacklisted = false;
    while let Some(cause) = current {
        if let Some(ShellExpansionError::Blacklisted { .. }) = cause.downcast_ref::<ShellExpansionError>() {
            blacklisted = true;
        }
        if let Some(MarkdownError::ShellExpansion(inner)) = cause.downcast_ref::<MarkdownError>()
            && matches!(**inner, ShellExpansionError::Blacklisted { .. })
        {
            blacklisted = true;
        }
        current = cause.source();
    }
    assert!(blacklisted, "{error:?}");
}

/// With no rule approving the command and no approval handler, a dry run
/// defers it rather than failing for want of an approval: it asks nothing.
#[test]
fn an_unapproved_command_is_deferred_without_asking() {
    for (label, text) in [
        ("frontmatter", "---\nvalue: $(touch marker)\n---\nbody\n"),
        ("body", "body\n\n::shell touch marker\n"),
    ] {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "doc.md", text);
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
            .with_source_file(path.clone())
            .with_shell_working_directory(dir.path())
            .with_dry_run();
        let session = options.passive_session().cloned().unwrap();
        compose(&path, options).unwrap_or_else(|error| panic!("{label}: {error}"));
        assert!(!dir.path().join("marker").exists(), "{label}");
        assert_eq!(session.deferrals().len(), 1, "{label}");
    }
}

/// A call the passive call check proves wrong still refuses the document,
/// and nothing runs before it.
#[test]
fn a_rejected_call_still_refuses_a_dry_run_before_any_effect() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\nvalue: $(touch marker)\n---\nLength {{ length(1, 2, 3) }}.\n");
    let error = compose(&path, options_in(dir.path(), &path).with_dry_run()).expect_err("the call check refuses");
    assert!(error.to_string().contains("length"), "{error}");
    assert!(!dir.path().join("marker").exists());
}

mod remote {
    use super::*;
    use darkmatter::markdown::compose::remote::RemoteReadConfig;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn server() -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/data.json"))
            .respond_with(ResponseTemplate::new(200).insert_header("cache-control", "max-age=3600").set_body_string("[\"a\"]"))
            .mount(&server)
            .await;
        server
    }

    fn remote_options(dir: &Path, doc: &Path, host: &str) -> ComposeOptions {
        options_in(dir, doc)
            .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec![host.into()], ..Default::default() })
            .with_cache_root(dir.join("cache"))
    }

    async fn compose_blocking(doc: PathBuf, options: ComposeOptions) -> Result<Markdown, MarkdownError> {
        tokio::task::spawn_blocking(move || compose(&doc, options)).await.unwrap()
    }

    /// An allowed host is fetched once, as the real run would, but a dry run
    /// persists nothing: the cache folder the real run creates never appears.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_allowed_read_is_fetched_without_a_persistent_store() {
        let server = server().await;
        let dir = TempDir::new().unwrap();
        let doc = write(dir.path(), "doc.md", &format!("Data {{{{ load_data('{}/data.json', 'string[]') }}}}.\n", server.uri()));

        let composed = compose_blocking(doc.clone(), remote_options(dir.path(), &doc, "127.0.0.1").with_dry_run()).await.unwrap();
        assert!(composed.content().contains("Data [\"a\"]."), "{}", composed.content());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert!(!dir.path().join("cache").exists(), "a dry run wrote the remote cache");

        // Control: the real run writes the cache.
        compose_blocking(doc.clone(), remote_options(dir.path(), &doc, "127.0.0.1")).await.unwrap();
        assert!(dir.path().join("cache").exists());
    }

    /// A denied host keeps its typed refusal and sends no request.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_denied_read_keeps_its_refusal_and_sends_nothing() {
        let server = server().await;
        let dir = TempDir::new().unwrap();
        let doc = write(dir.path(), "doc.md", &format!("Data {{{{ load_data('{}/data.json', 'string[]') }}}}.\n", server.uri()));
        let error = compose_blocking(doc.clone(), remote_options(dir.path(), &doc, "example.com").with_dry_run())
            .await
            .expect_err("a denied host is refused");
        assert!(error.to_string().contains("127.0.0.1"), "{error}");
        assert_eq!(server.received_requests().await.unwrap().len(), 0);
    }

    /// A remote transclusion follows the same policy, and a command in the
    /// fetched child is deferred.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_include_is_fetched_and_its_command_deferred() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/child.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("---\nstamp: $(touch marker)\n---\nRemote {{ stamp }}.\n"))
            .mount(&server)
            .await;
        let dir = TempDir::new().unwrap();
        let doc = write(dir.path(), "doc.md", &format!("Parent.\n\n::file {}/child.md\n", server.uri()));
        let options = remote_options(dir.path(), &doc, "127.0.0.1").with_allow_remote_transclusion(true).with_dry_run();
        let session = options.passive_session().cloned().unwrap();
        let composed = compose_blocking(doc, options).await.unwrap();
        assert!(composed.content().contains("Remote {{ stamp }}."), "{}", composed.content());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert!(!dir.path().join("marker").exists() && !dir.path().join("cache").exists());
        assert!(session.deferrals().iter().any(|deferral| deferral.key.as_deref() == Some("stamp")));
    }
}

/// A value a dry run deferred keeps its command text, which no schema type
/// admits; the verdict leaves it unjudged, since no run resolved it, while
/// an independent value is still judged.
#[test]
fn a_deferred_value_is_never_judged_by_its_schema() {
    let dir = TempDir::new().unwrap();
    let run = dry_run(dir.path(), "---\n$schema:\n  count: number\ncount: $(echo 3)\n---\nbody\n");
    assert_eq!(run.frontmatter["count"], "$(echo 3)");
    assert_eq!(constructs(&run), [DeferredConstruct::FrontmatterValue]);

    let path = write(dir.path(), "wrong.md", "---\n$schema:\n  count: number\nother: $(echo 3)\ncount: abc\n---\nbody\n");
    let error = compose(&path, options_in(dir.path(), &path).with_dry_run()).expect_err("an independent value is judged");
    assert!(matches!(error, MarkdownError::SchemaValidationFailed { .. }), "{error:?}");
}
