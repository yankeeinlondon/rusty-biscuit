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

/// A required value the caller declared absent (a missing input the dry run
/// reports itself) is not judged missing, and what reads it stays as
/// written: a span, a `::block` condition, and a frontmatter value derived
/// from it. Independent text composes, and a value supplied after all reads
/// as itself. Its control, an undeclared missing value, still fails the
/// verdict.
#[test]
fn a_value_declared_absent_leaves_only_its_readers_as_written() {
    let dir = TempDir::new().unwrap();
    let text = "---\n$schema:\n  topic: string(required)\nheading: \"About {{ topic }}\"\nowner: Ken\n---\nOwner {{ owner }}.\nTopic {{ topic }}.\nHeading {{ heading }}.\n\n::block when=\"topic == 'x'\"\nx only\n::end-block\n";
    let path = write(dir.path(), "doc.md", text);
    let options = options_in(dir.path(), &path).with_dry_run();
    let session = options.passive_session().cloned().unwrap();
    let missing = DeferralReason::MissingInput { name: "topic".into() };
    session.defer_absent("topic", missing.clone());

    let composed = compose(&path, options).unwrap_or_else(|error| panic!("the dry run failed: {error}"));

    let body = composed.content();
    assert!(body.contains("Owner Ken."), "{body}");
    assert!(body.contains("Topic {{ topic }}."), "{body}");
    assert!(body.contains("Heading {{ heading }}."), "{body}");
    assert!(body.contains("::block when=\"topic == 'x'\""), "{body}");
    let deferrals = session.deferrals();
    assert!(
        deferrals.iter().any(|deferral| deferral.construct == DeferredConstruct::Interpolation && deferral.reason == missing),
        "{deferrals:?}"
    );
    assert!(deferrals.iter().any(|deferral| deferral.key.as_deref() == Some("heading")), "{deferrals:?}");
    assert_eq!(session.absent_value("topic"), Some(missing));

    let control = compose(&path, options_in(dir.path(), &path).with_dry_run()).expect_err("an undeclared absence is judged");
    assert!(matches!(control, MarkdownError::SchemaValidationFailed { .. }), "{control:?}");
}

/// A value declared written at launch (a key the run writes before this
/// composition, or a member such as `seq.outputs`) is unavailable whatever
/// it holds now: a `::block` reading it stays whole as a conditional, and a
/// schema problem with it is set aside as one `SchemaCheck` deferral. The
/// same document with nothing declared is judged on the value it holds.
#[test]
fn a_value_declared_written_is_read_as_known_only_at_launch() {
    let dir = TempDir::new().unwrap();
    let text = "---\ntopic: missing\n$schema:\n  topic: enum(a)\n---\n\n::block when=\"topic == 'a'\"\nGuarded.\n::end-block\n";
    let path = write(dir.path(), "doc.md", text);
    let options = options_in(dir.path(), &path).with_dry_run();
    let session = options.passive_session().cloned().unwrap();
    let written = DeferralReason::WrittenAtLaunch { name: "topic".into() };
    session.defer_written("topic", written.clone());

    let composed = compose(&path, options).unwrap_or_else(|error| panic!("the dry run failed: {error}"));

    let body = composed.content();
    assert!(body.contains("::block when=\"topic == 'a'\""), "{body}");
    assert!(body.contains("Guarded."), "{body}");
    let deferrals = session.deferrals();
    for construct in [DeferredConstruct::BlockCondition, DeferredConstruct::SchemaCheck] {
        assert!(
            deferrals.iter().any(|deferral| deferral.construct == construct && deferral.reason == written),
            "{construct:?}: {deferrals:?}"
        );
    }
    assert_eq!(session.written_value("fm.topic"), Some(written));

    let control = compose(&path, options_in(dir.path(), &path).with_dry_run()).expect_err("an undeclared value is judged");
    assert!(matches!(control, MarkdownError::SchemaValidationFailed { .. }), "{control:?}");
}

/// A written member is deferred by what a read selects, not by how it is
/// spelled: with the host's `seq.outputs` declared written, dotted,
/// bracket, parenthesized, and computed-key reads of it stay as written in
/// an interpolation and a `::block` condition, while the same spellings of
/// the static `seq.index` compose. The document's own `seq:` property,
/// read through `fm.` in any spelling, is not the host's and composes too.
#[test]
fn a_written_member_is_deferred_in_every_spelling_of_its_selection() {
    use darkmatter::markdown::compose::expression::{Availability, BindingView, HostGlobals, RuntimeBinding, ScopeId};

    let deferred = ["seq.outputs", "seq['outputs']", "(seq).outputs", "seq['out' + 'puts']", "seq.outputs[0]"];
    let composed = [
        ("seq.index", "2"),
        ("seq['index']", "2"),
        ("(seq).index", "2"),
        ("seq['in' + 'dex']", "2"),
        ("fm.seq.outputs[0]", "static"),
        ("fm['seq'].outputs[0]", "static"),
        ("fm.seq['outputs'][0]", "static"),
    ];
    let mut text = String::from("---\nseq:\n  outputs: [static]\n---\n");
    for read in deferred.iter().chain(composed.iter().map(|(read, _)| read)) {
        text.push_str(&format!("Read <{{{{ {read} }}}}>.\n\n::block when=\"{read} != null\"\nGuarded by {read}.\n::end-block\n\n"));
    }
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", &text);
    let globals = || {
        let view = BindingView::builder(ScopeId::new("host.sequence"))
            .declare("seq", "The step's sequence values.", Availability::Available)
            .build()
            .unwrap();
        HostGlobals::associate(
            std::sync::Arc::new(view),
            [("seq".to_string(), RuntimeBinding::eager(serde_json::json!({ "index": 2, "outputs": [] })))],
        )
        .unwrap()
    };
    let options = options_in(dir.path(), &path).with_host_globals(globals()).with_dry_run();
    let session = options.passive_session().cloned().unwrap();
    let written = DeferralReason::WrittenAtLaunch { name: "seq.outputs".into() };
    session.defer_written("seq.outputs", written.clone());

    let body = compose(&path, options).unwrap_or_else(|error| panic!("the dry run failed: {error}")).content().to_string();

    for read in deferred {
        assert!(body.contains(&format!("Read <{{{{ {read} }}}}>.")), "`{read}` must stay as written: {body}");
        assert!(body.contains(&format!("::block when=\"{read} != null\"")), "`{read}` must stay conditional: {body}");
    }
    for (read, value) in composed {
        assert!(body.contains(&format!("Read <{value}>.")), "`{read}` must compose to `{value}`: {body}");
        assert!(body.contains(&format!("Guarded by {read}.")) && !body.contains(&format!("when=\"{read} != null\"")), "{read}: {body}");
    }
    let deferrals = session.deferrals();
    for construct in [DeferredConstruct::Interpolation, DeferredConstruct::BlockCondition] {
        let count = deferrals.iter().filter(|deferral| deferral.construct == construct && deferral.reason == written).count();
        assert_eq!(count, deferred.len(), "{construct:?}: {deferrals:#?}");
    }

    // The control: nothing declared written, so every spelling composes.
    let control = compose(&path, options_in(dir.path(), &path).with_host_globals(globals()).with_dry_run()).unwrap();
    assert!(!control.content().contains("{{"), "{}", control.content());
}

/// A `::map`/`::filter` callback and its options read a written member
/// through the same selection rule: every spelling of `seq.outputs` meets
/// the dry run's unavailable value exactly as the dotted one does (how a
/// collection grades it is its own rule), while the same spellings of the
/// static `seq.index` compose the items.
#[test]
fn a_collection_callback_reads_a_written_member_alike_in_every_spelling() {
    use darkmatter::markdown::compose::expression::{Availability, BindingView, HostGlobals, RuntimeBinding, ScopeId};
    use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;

    let dir = TempDir::new().unwrap();
    let globals = || {
        let view = BindingView::builder(ScopeId::new("host.sequence"))
            .declare("seq", "The step's sequence values.", Availability::Available)
            .build()
            .unwrap();
        HostGlobals::associate(
            std::sync::Arc::new(view),
            [("seq".to_string(), RuntimeBinding::eager(serde_json::json!({ "index": 2, "outputs": [] })))],
        )
        .unwrap()
    };
    // The body, or the codes of the failure, of each site reading `read`.
    let outcome = |read: &str, written: bool| -> Vec<Result<String, Vec<String>>> {
        [
            format!("::filter items \"i => {read}\"\n- {{{{i}}}}\n::end-filter\n"),
            format!("::map items \"i => i\" filter=\"i => {read}\"\n- {{{{i}}}}\n::end-map\n"),
            format!("::map items \"i => i\" when=\"{read}\"\n- {{{{i}}}}\n::end-map\n"),
        ]
        .iter()
        .map(|site| {
            let path = write(dir.path(), "doc.md", &format!("---\n$schema:\n  items: \"number[]\"\nitems: [1, 2]\n---\n{site}"));
            let options = options_in(dir.path(), &path).with_host_globals(globals()).with_dry_run();
            if written {
                let session = options.passive_session().cloned().unwrap();
                session.defer_written("seq.outputs", DeferralReason::WrittenAtLaunch { name: "seq.outputs".into() });
            }
            compose(&path, options)
                .map(|composed| composed.content().replace(read, "READ"))
                .map_err(|error| error_diagnostics(&error).into_iter().map(|diagnostic| diagnostic.code).collect())
        })
        .collect()
    };

    let dotted = outcome("length(seq.outputs) == 0", true);
    assert_ne!(dotted, outcome("length(seq.outputs) == 0", false), "the dotted read must meet the written value");
    for read in ["length(seq['outputs']) == 0", "length((seq).outputs) == 0", "length(seq['out' + 'puts']) == 0"] {
        assert_eq!(outcome(read, true), dotted, "`{read}` must read the written member as the dotted form does");
    }
    for read in ["seq.index == 2", "seq['index'] == 2", "(seq).index == 2"] {
        for site in outcome(read, true) {
            let body = site.unwrap_or_else(|codes| panic!("`{read}` must compose: {codes:?}"));
            assert!(body.contains("- 1") && body.contains("- 2"), "`{read}`: {body}");
        }
    }
}

/// An inclusion whose target reads a value the dry run left as written (a
/// declared-absent input, a `$( … )` value) names no path: the `::file` and
/// `::code` lines stay as written, `prologue`/`epilogue` contribute nothing,
/// and each is a deferred transclusion, never a parse or missing-file
/// failure. Its control, the same document with the value known, includes
/// the files.
#[test]
fn an_inclusion_whose_target_reads_a_deferred_value_is_left_as_written() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.md", "Included.\n");
    write(dir.path(), "a.rs", "fn a() {}\n");
    let text = |topic: &str| {
        format!(
            "---\n$schema:\n  topic: string(required)\n{topic}prologue: ./{{{{ topic }}}}.md\nepilogue: ./{{{{topic}}}}.md\n---\nIntro.\n\n::file ./{{{{ topic }}}}.md\n\n::code ./{{{{topic}}}}.rs\n"
        )
    };
    for topic in ["", "topic: $(echo a)\n"] {
        let path = write(dir.path(), "doc.md", &text(topic));
        let options = options_in(dir.path(), &path).with_dry_run();
        let session = options.passive_session().cloned().unwrap();
        session.defer_absent("topic", DeferralReason::MissingInput { name: "topic".into() });

        let composed = compose(&path, options).unwrap_or_else(|error| panic!("the dry run failed ({topic:?}): {error}"));

        let body = composed.content();
        assert!(body.contains("::file ./{{ topic }}.md\n"), "{body}");
        assert!(body.contains("::code ./{{topic}}.rs"), "{body}");
        assert!(!body.contains("Included."), "{body}");
        let transclusions: Vec<&str> = session
            .deferrals()
            .iter()
            .filter(|deferral| deferral.construct == DeferredConstruct::Transclusion)
            .map(|deferral| deferral.authored.as_str())
            .map(|authored| if authored.starts_with("::") { "directive" } else { "frontmatter" })
            .collect();
        assert_eq!(transclusions, ["directive", "directive", "frontmatter", "frontmatter"], "{topic:?}");
    }

    let path = write(dir.path(), "doc.md", &text("topic: a\n"));
    let composed = compose(&path, options_in(dir.path(), &path).with_dry_run()).unwrap();
    assert_eq!(composed.content().matches("Included.").count(), 3, "{}", composed.content());
    assert!(composed.content().contains("fn a() {}"));
}

/// A host's `seq` global, `{ index: 2, outputs: [] }`, as a sequence step
/// binds it.
fn sequence_globals() -> darkmatter::markdown::compose::expression::HostGlobals {
    use darkmatter::markdown::compose::expression::{Availability, BindingView, HostGlobals, RuntimeBinding, ScopeId};
    let view = BindingView::builder(ScopeId::new("host.sequence"))
        .declare("seq", "The step's sequence values.", Availability::Available)
        .build()
        .unwrap();
    HostGlobals::associate(
        std::sync::Arc::new(view),
        [("seq".to_string(), RuntimeBinding::eager(serde_json::json!({ "index": 2, "outputs": [] })))],
    )
    .unwrap()
}

/// What a dry run of `body` under `items: [1, 2]`, with the host's `seq`
/// and `written` declared written at launch, produced: the composed body
/// and the deferrals, or the codes of its failure.
fn collection_dry_run(dir: &Path, body: &str, written: &[&str]) -> Result<(String, Vec<Deferral>), Vec<String>> {
    use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
    let text = format!("---\n$schema:\n  items: \"number[]\"\nitems: [1, 2]\ntitle: Kept\n---\nBefore {{{{ title }}}}.\n\n{body}\nAfter {{{{ title }}}}.\n");
    let path = write(dir, "doc.md", &text);
    let options = options_in(dir, &path).with_host_globals(sequence_globals()).with_dry_run();
    let session = options.passive_session().cloned().unwrap();
    for name in written {
        session.defer_written(name, DeferralReason::WrittenAtLaunch { name: (*name).to_string() });
    }
    compose(&path, options)
        .map(|composed| (composed.content().to_string(), session.deferrals()))
        .map_err(|error| error_diagnostics(&error).into_iter().map(|diagnostic| diagnostic.code).collect())
}

/// Asserts `region` stayed as written and inert beside composed text, with
/// exactly one `Collection` deferral for `reason`.
fn assert_collection_deferred(outcome: Result<(String, Vec<Deferral>), Vec<String>>, region: &str, reason: &DeferralReason) {
    let (body, deferrals) = outcome.unwrap_or_else(|codes| panic!("`{region}` must defer, not fail: {codes:?}"));
    assert!(body.contains(region), "`{region}` must stay as written: {body}");
    assert!(body.contains("Before Kept.") && body.contains("After Kept."), "{body}");
    let collections: Vec<&Deferral> =
        deferrals.iter().filter(|deferral| deferral.construct == DeferredConstruct::Collection).collect();
    assert_eq!(collections.len(), 1, "{deferrals:#?}");
    assert_eq!(&collections[0].reason, reason, "{region}");
    assert_eq!(collections[0].authored, region);
    assert!(collections[0].line.is_some(), "{:?}", collections[0]);
}

/// A collection whose source the run writes before this composition is
/// left whole and inert: its body's `{{ i }}` and `::file` target, which
/// read a name only the expansion binds, are not evaluated or read. A
/// write to an unrelated key leaves the collection to compose.
#[test]
fn a_collection_whose_source_is_written_at_launch_is_left_whole_and_inert() {
    let dir = TempDir::new().unwrap();
    for region in [
        "::map items \"i => i * 10\"\n- {{ i }}\n::file ./{{ i }}.md\n::end-map\n",
        "::filter items \"i => i > 1\"\n- {{ i }}\n::file ./{{ i }}.md\n::end-filter\n",
    ] {
        let reason = DeferralReason::WrittenAtLaunch { name: "items".into() };
        assert_collection_deferred(collection_dry_run(dir.path(), region, &["items"]), region, &reason);

        let control = region.replace("::file ./{{ i }}.md\n", "");
        let (body, _) = collection_dry_run(dir.path(), &control, &["other"]).unwrap_or_else(|codes| panic!("{codes:?}"));
        assert!(!body.contains("::map") && !body.contains("::filter") && body.contains("- 2"), "{body}");
    }
}

/// Every place a collection evaluates an expression (positional transform
/// or predicate, `filter=`, `while=`, `until=`, `sort=`, and `when=`, on
/// both directives) defers the whole collection when the expression reads
/// a written member, and composes when it reads the static one.
#[test]
fn every_collection_option_defers_an_unavailable_read_and_composes_a_static_one() {
    let dir = TempDir::new().unwrap();
    let sites = |read: &str, key: &str| -> Vec<String> {
        let mut sites = vec![
            format!("::map items \"i => {read} ? i : 0\"\n- {{{{ i }}}}\n::end-map\n"),
            format!("::filter items \"i => {read}\"\n- {{{{ i }}}}\n::end-filter\n"),
        ];
        for (directive, callback) in [("map", " \"i => i\""), ("filter", " \"i => true\"")] {
            for option in [
                format!("filter=\"i => {read}\""),
                format!("while=\"i => {read}\""),
                format!("until=\"i => !({read})\""),
                format!("sort=\"i => {key}\""),
                format!("when=\"{read}\""),
            ] {
                sites.push(format!("::{directive} items{callback} {option}\n- {{{{ i }}}}\n::end-{directive}\n"));
            }
        }
        sites
    };
    let written = DeferralReason::WrittenAtLaunch { name: "seq.outputs".into() };
    let deferred = sites("length(seq.outputs) == 0", "length(seq.outputs) + i");
    assert_eq!(deferred.len(), 12);
    for region in &deferred {
        assert_collection_deferred(collection_dry_run(dir.path(), region, &["seq.outputs"]), region, &written);
    }
    for region in sites("seq.index == 2", "seq.index + i") {
        let (body, deferrals) =
            collection_dry_run(dir.path(), &region, &["seq.outputs"]).unwrap_or_else(|codes| panic!("`{region}`: {codes:?}"));
        assert!(body.contains("- 1") && body.contains("- 2") && !body.contains("::end-"), "`{region}`: {body}");
        assert!(deferrals.is_empty(), "`{region}`: {deferrals:?}");
    }
}

/// A dry run defers only what it cannot evaluate: a genuine failure on
/// known data still fails, beside a written member it never reaches.
#[test]
fn a_genuine_collection_failure_on_known_data_still_fails() {
    let dir = TempDir::new().unwrap();
    let cases = [
        ("::map items \"i => i\" sort=\"i => i > 1\"\n- {{ i }}\n::end-map\n", "dm.collection.invalid_sort_key"),
        ("::filter items \"i => true\" sort=\"i => i > 1\"\n- {{ i }}\n::end-filter\n", "dm.collection.invalid_sort_key"),
        ("::filter items \"i => i / 0 > 1\"\n- {{ i }}\n::end-filter\n", "dm.collection.callback_failed"),
    ];
    for (region, code) in cases {
        let codes = collection_dry_run(dir.path(), region, &["seq.outputs"]).map(|(body, _)| body).expect_err(region);
        assert!(codes.iter().any(|found| found == code), "`{region}`: {codes:?}");
    }
}

/// A predicate that meets the unavailable value only at a later item
/// discards the copies earlier items produced: the collection is left
/// whole, never half expanded.
#[test]
fn a_collection_deferred_at_a_later_item_keeps_no_partial_expansion() {
    let dir = TempDir::new().unwrap();
    let region = "::filter items \"i => i == 1 || length(seq.outputs) > 0\"\n- item {{ i }}\n::end-filter\n";
    let written = DeferralReason::WrittenAtLaunch { name: "seq.outputs".into() };
    let outcome = collection_dry_run(dir.path(), region, &["seq.outputs"]);
    if let Ok((body, _)) = &outcome {
        assert!(!body.contains("- item 1"), "{body}");
    }
    assert_collection_deferred(outcome, region, &written);
}

/// A page block inside a collection whose condition reads a written
/// member stays with its directive lines in each copy, as a page block
/// outside a collection does, and the collection around it expands.
#[test]
fn a_page_block_inside_a_collection_defers_like_one_outside() {
    let dir = TempDir::new().unwrap();
    let region = "::map items \"i => i\"\n- item {{ i }}\n::block when=\"length(seq.outputs) > 0\"\nguarded {{ i }}\n::end-block\n::end-map\n";
    let (body, deferrals) = collection_dry_run(dir.path(), region, &["seq.outputs"]).unwrap_or_else(|codes| panic!("{codes:?}"));
    assert!(!body.contains("::map"), "{body}");
    for item in ["1", "2"] {
        assert!(body.contains(&format!("- item {item}\n::block when=\"length(seq.outputs) > 0\"\nguarded {item}\n::end-block")), "{body}");
    }
    let written = DeferralReason::WrittenAtLaunch { name: "seq.outputs".into() };
    assert!(
        deferrals.iter().all(|deferral| deferral.construct == DeferredConstruct::BlockCondition && deferral.reason == written),
        "{deferrals:#?}"
    );
    assert!(!deferrals.is_empty());

    let (body, _) = collection_dry_run(dir.path(), region, &[]).unwrap_or_else(|codes| panic!("{codes:?}"));
    assert!(!body.contains("::block") && !body.contains("guarded"), "{body}");
}

/// An effectful function in a callback is not called; the collection is
/// deferred for the effect, and a pure callback composes.
#[test]
fn an_effectful_callback_defers_its_collection() {
    let dir = TempDir::new().unwrap();
    let region = "::filter items \"i => has_alias('ll')\"\n- {{ i }}\n::end-filter\n";
    let reason = DeferralReason::ShellProbe { function: "has_alias".into() };
    assert_collection_deferred(collection_dry_run(dir.path(), region, &[]), region, &reason);

    let (body, _) = collection_dry_run(dir.path(), &region.replace("has_alias('ll')", "upper('ll') == 'LL'"), &[])
        .unwrap_or_else(|codes| panic!("{codes:?}"));
    assert!(body.contains("- 1") && body.contains("- 2"), "{body}");
}

/// A `::file`/`::code` whose `when=` reads a written member names no
/// decision: the directive stays as written and is a deferred transclusion,
/// never a condition failure, and its static control includes the file.
#[test]
fn a_transclusion_condition_that_reads_a_written_member_is_left_as_written() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.md", "Included.\n");
    let region = "::file ./a.md when=\"length(seq.outputs) > 0\"\n";
    let (body, deferrals) = collection_dry_run(dir.path(), region, &["seq.outputs"]).unwrap_or_else(|codes| panic!("{codes:?}"));
    assert!(body.contains(region.trim_end()) && !body.contains("Included."), "{body}");
    let written = DeferralReason::WrittenAtLaunch { name: "seq.outputs".into() };
    assert!(
        deferrals.iter().any(|deferral| deferral.construct == DeferredConstruct::Transclusion && deferral.reason == written),
        "{deferrals:#?}"
    );

    let control = region.replace("length(seq.outputs) > 0", "seq.index == 2");
    let (body, _) = collection_dry_run(dir.path(), &control, &["seq.outputs"]).unwrap_or_else(|codes| panic!("{codes:?}"));
    assert!(body.contains("Included.") && !body.contains("::file"), "{body}");
}
