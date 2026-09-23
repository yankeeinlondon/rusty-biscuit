//! `as_markdown(content)` through the library compose entry point (plan
//! Phase 8: AC15, AC16, and the nested portions of AC30/AC32).
//!
//! Every fixture lives in its own temporary directory. No test launches a
//! login shell; the shell commands that run are `echo`/`touch` approved
//! through the pre-approved set.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::remote::RemoteReadConfig;
use darkmatter::markdown::compose::{
    ComposeOperation, ComposeOptions, ComposeReport, ComposeStage, ShellExpansionError,
    TransclusionError,
};
use darkmatter::markdown::{Markdown, MarkdownError};

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn root_document(dir: &Path, relative: &str, content: &str) -> (PathBuf, Markdown) {
    let path = dir.join(relative);
    write(&path, content);
    (path.clone(), Markdown::try_from(path.as_path()).unwrap())
}

/// Stages that keep `name=[value]` probe lines intact.
fn probe_options() -> ComposeOptions {
    ComposeOptions::new().only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::Interpolation,
        ComposeOperation::BlockTransclusion,
    ])
}

fn compose(document: &Markdown, options: ComposeOptions) -> (Markdown, ComposeReport) {
    document
        .compose_with(options)
        .unwrap_or_else(|error| panic!("compose must succeed: {error}"))
}

/// Every `name=[value]` pair on each line, in order.
fn fields(content: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for mut rest in content.lines() {
        while let Some((name, tail)) = rest.split_once("=[") {
            let Some((value, after)) = tail.split_once(']') else { break };
            let name = name.rsplit(' ').next().unwrap_or(name);
            found.push((name.to_string(), value.to_string()));
            rest = after;
        }
    }
    found
}

fn field<'a>(fields: &'a [(String, String)], name: &str) -> &'a str {
    fields
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value.as_str())
        .unwrap_or_else(|| panic!("missing field {name} in {fields:?}"))
}

fn transclusion_error(error: &MarkdownError) -> &TransclusionError {
    match error {
        MarkdownError::Transclusion(inner) => inner,
        other => panic!("expected a typed transclusion error, got: {other:?}"),
    }
}

/// AC15: nested content reads the root's captured `ctx` and resolves
/// `::file` against the root document's directory, even when the call sits in
/// a transcluded document elsewhere.
#[test]
fn nested_content_shares_the_root_context_and_resolution_base() {
    let temp = tempfile::tempdir().unwrap();
    write(&temp.path().join("docs/part.md"), "ROOT-RELATIVE PART\n");
    write(&temp.path().join("docs/sub/part.md"), "CALLER-RELATIVE PART\n");
    write(
        &temp.path().join("docs/sub/child.md"),
        "{{ as_markdown(\"child_host=[{{ ctx.hostname }}]\\n\\n::file ./part.md\") }}\n",
    );
    let (path, document) = root_document(
        temp.path(),
        "docs/root.md",
        "host=[{{ ctx.hostname }}]\n\n\
         {{ as_markdown(\"nested_host=[{{ ctx.hostname }}]\\n\\n::file ./part.md\") }}\n\n\
         ::file ./sub/child.md\n",
    );

    let (composed, _) = compose(&document, probe_options().with_source_file(&path));
    let text = composed.content();
    let values = fields(text);

    assert!(!field(&values, "host").is_empty(), "{text}");
    assert_eq!(field(&values, "nested_host"), field(&values, "host"));
    assert_eq!(field(&values, "child_host"), field(&values, "host"));
    assert_eq!(text.matches("ROOT-RELATIVE PART").count(), 2, "{text}");
    assert!(!text.contains("CALLER-RELATIVE"), "{text}");
    assert!(!text.contains("as_markdown"), "{text}");
}

/// AC30 (nested portion): file, URL, and in-memory roots keep one document
/// identity and one per-execution nonce across the nested child.
#[test]
fn nested_content_keeps_the_root_document_identity() {
    let temp = tempfile::tempdir().unwrap();
    let body = "root=[{{ ctx.id }}|{{ ctx.sid }}|{{ ctx.self }}|{{ ctx.hash }}]\n\n\
                {{ as_markdown(\"nested=[{{ ctx.id }}|{{ ctx.sid }}|{{ ctx.self }}|{{ ctx.hash }}]\") }}\n";
    let (path, file_document) = root_document(temp.path(), "root.md", body);
    let url = url::Url::parse("https://example.invalid/docs/root.md").unwrap();
    let cases = [
        ("file", file_document, probe_options().with_source_file(&path)),
        ("url", Markdown::from(body), probe_options().with_source_url(url)),
        ("in-memory", Markdown::from(body), probe_options()),
    ];

    for (kind, document, options) in cases {
        let (composed, _) = compose(&document, options);
        let values = fields(composed.content());
        let root = field(&values, "root");
        assert_eq!(field(&values, "nested"), root, "{kind}: {}", composed.content());
        let parts: Vec<&str> = root.split('|').collect();
        assert!(!parts[0].is_empty() && !parts[1].is_empty(), "{kind}: {root}");
        if kind == "file" {
            assert!(parts[2].ends_with("root.md"), "{kind}: {root}");
        } else {
            assert_eq!(&parts[2..], ["", ""], "{kind}: document fields are null: {root}");
        }
    }
}

/// Frontmatter and body calls both compose, and the serialization carries
/// the frontmatter the content authored but none of the request state
/// composition merges into it.
#[test]
fn frontmatter_and_body_calls_return_composed_markdown_with_authored_frontmatter() {
    let temp = tempfile::tempdir().unwrap();
    let (path, document) = root_document(
        temp.path(),
        "root.md",
        "---\nintro: '{{ as_markdown(\"# Intro {{ 1 + 1 }}\") }}'\n---\n\
         intro=[{{ intro }}]\n\n\
         {{ as_markdown(\"---\\ntitle: 'Nested {{ 2 + 2 }}'\\n---\\nnested body {{ leaked }}\") }}\n",
    );
    let options = probe_options()
        .with_source_file(&path)
        .with_external_state(serde_json::json!({ "leaked": "state" }));

    let (composed, _) = compose(&document, options);
    let text = composed.content();

    assert_eq!(
        composed.frontmatter().as_map().get("intro"),
        Some(&serde_json::Value::String("# Intro 2".to_string())),
        "{text}"
    );
    assert!(text.contains("intro=[# Intro 2]"), "{text}");
    assert!(text.contains("---\ntitle: Nested 4\n---\nnested body state"), "{text}");
    assert!(!text.contains("leaked:"), "request state must not serialize: {text}");
}

/// Adversarial arguments: an empty string composes to nothing, while a
/// non-string or null argument is the usual argument-type error, fatal in a
/// document body.
#[test]
fn empty_and_non_string_arguments() {
    let (composed, _) = compose(&Markdown::from("empty=[{{ as_markdown(\"\") }}]\n"), probe_options());
    assert!(composed.content().contains("empty=[]"), "{}", composed.content());

    for (actual, body) in [
        ("number", "number={{ as_markdown(42) }}\n"),
        ("null", "null={{ as_markdown(missing) }}\n"),
    ] {
        let error = Markdown::from(body)
            .compose_with(probe_options())
            .expect_err("a non-string argument fails the document");
        let expected = format!("as_markdown() argument 0: expected string, got {actual}");
        assert!(error.to_string().contains(&expected), "{expected}: {error}");
    }
}

/// Nested diagnostics are tagged with their nested stage, and a nested
/// failure names both the call site and the nested cause.
#[test]
fn nested_diagnostics_keep_their_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let (path, document) = root_document(
        temp.path(),
        "root.md",
        "{{ as_markdown(\"typo={{ ctx.toady }}\") }}\n",
    );
    let (_, report) = compose(&document, probe_options().with_source_file(&path));
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.stage == "as_markdown > interpolation" && warning.message.contains("ctx.toady")),
        "{:?}",
        report.warnings
    );

    // The nested document is a full document too, so an inner evaluation
    // failure is fatal and names both the call site and the inner cause.
    let (path, document) = root_document(
        temp.path(),
        "arity.md",
        "{{ as_markdown(\"strict={{ length(1, 2) }}\") }}\n",
    );
    let error = document
        .compose_with(probe_options().with_source_file(&path))
        .expect_err("a nested evaluation failure fails the request");
    let message = format!("{error}");
    assert!(message.contains("as_markdown(): nested composition failed"), "{message}");
    assert!(message.contains("length"), "{message}");

    let (path, document) = root_document(
        temp.path(),
        "failing.md",
        "before\n\n{{ as_markdown(\"{{ no_such_function() }}\") }}\n",
    );
    let error = document
        .compose_with(probe_options().with_source_file(&path))
        .expect_err("a nested compose error fails the request");
    let message = format!("{error}");
    assert!(message.contains("as_markdown(): nested composition failed"), "{message}");
    assert!(message.contains("no_such_function"), "{message}");
}

/// Link normalization is root-only: a nested link is normalized exactly
/// once, by the outermost pipeline, for file and in-memory roots alike.
#[test]
fn nested_links_are_normalized_once_at_the_root() {
    let temp = tempfile::tempdir().unwrap();
    let home_link = format!(
        "[home]({})",
        biscuit_file::to_portable_string(&dirs::home_dir().unwrap().join("notes.md"))
    );
    let body = format!("{{{{ as_markdown(\"{home_link}\") }}}}\n");
    let (path, file_document) = root_document(temp.path(), "root.md", &body);

    for (kind, document, options) in [
        ("file", file_document, ComposeOptions::new().with_source_file(&path)),
        ("in-memory", Markdown::from(body.as_str()), ComposeOptions::new()),
    ] {
        let (composed, report) = compose(&document, options.with_perf(true));
        assert_eq!(composed.content().trim(), "[home](~/notes.md)", "{kind}");
        let normalization_runs = report
            .perf
            .as_ref()
            .and_then(|perf| perf.metrics.iter().find(|metric| metric.stage == ComposeStage::LinkNormalization))
            .map_or(0, |metric| metric.calls);
        assert_eq!(normalization_runs, 1, "{kind}");
    }
}

/// AC16: content that composes itself stops at the shared depth limit with the
/// typed transclusion error instead of hanging.
#[test]
fn self_composition_reaches_the_depth_limit() {
    let document = Markdown::from("{{ as_markdown(again) }}\n");
    // `again` is mixed-text state referencing itself, which frontmatter
    // interpolation would reject; excluding it keeps the raw template for each
    // nested level to compose.
    let options = probe_options()
        .with_external_state(serde_json::json!({
            "again": "again {{ as_markdown(again) }}"
        }))
        .with_exclude_keys(["again"]);

    let error = document.compose_with(options).expect_err("self-composition must fail");

    assert!(
        matches!(transclusion_error(&error), TransclusionError::MaxDepthExceeded { max_depth: 16 }),
        "{error:?}"
    );
}

/// AC32: `as_markdown` and `::file` share one budget, so a chain alternating
/// between them fails one level earlier than its transclusions alone would.
#[test]
fn mixed_nesting_consumes_one_depth_budget() {
    let temp = tempfile::tempdir().unwrap();
    write(&temp.path().join("leaf.md"), "LEAF\n");
    let (path, document) = root_document(temp.path(), "root.md", "{{ as_markdown(\"::file ./leaf.md\") }}\n");

    // root (1) -> as_markdown (2) -> leaf.md (3)
    let (composed, _) = compose(
        &document,
        probe_options().with_source_file(&path).with_max_transclusion_depth(3),
    );
    assert!(composed.content().contains("LEAF"), "{}", composed.content());

    let error = document
        .compose_with(probe_options().with_source_file(&path).with_max_transclusion_depth(2))
        .expect_err("the as_markdown node counts against the budget");
    assert!(
        matches!(transclusion_error(&error), TransclusionError::MaxDepthExceeded { max_depth: 2 }),
        "{error:?}"
    );
}

/// Mutual `as_markdown` <-> `::file` recursion is a typed cycle through the
/// shared ancestry.
#[test]
fn mutual_function_and_transclusion_recursion_is_a_cycle() {
    let temp = tempfile::tempdir().unwrap();
    write(&temp.path().join("b.md"), "{{ as_markdown(\"::file ./a.md\") }}\n");
    let (path, document) = root_document(temp.path(), "a.md", "{{ as_markdown(\"::file ./b.md\") }}\n");

    let error = document
        .compose_with(probe_options().with_source_file(&path))
        .expect_err("mutual recursion must fail");

    let TransclusionError::CycleDetected { chain } = transclusion_error(&error) else {
        panic!("expected a cycle: {error:?}");
    };
    assert!(chain.iter().any(|(node, _)| node == Path::new("as_markdown()")), "{chain:?}");
}

fn approval_set(document: &Markdown, options: &ComposeOptions) -> HashSet<String> {
    document
        .compose_preflight(options)
        .unwrap_or_else(|error| panic!("preflight must succeed: {error}"))
        .approval_set()
        .into_iter()
        .collect()
}

/// AC32: statically known nested effects appear in preflight, including
/// content in a false page block and an untaken ternary branch, and content
/// passed through a frontmatter value; the approved compose then runs them.
#[test]
fn preflight_discovers_nested_effects_without_composing_them() {
    let temp = tempfile::tempdir().unwrap();
    let sentinel = temp.path().join("nested-ran");
    let sentinel_text = biscuit_file::to_portable_string(&sentinel);
    let (path, document) = root_document(
        temp.path(),
        "root.md",
        &format!(
            "---\npart: '::shell echo from-frontmatter-value'\n---\n\
             {{{{ as_markdown(\"::shell touch {sentinel_text}\") }}}}\n\n\
             ::block when=\"false\"\n{{{{ as_markdown(\"::shell echo unreachable-block\") }}}}\n::end-block\n\n\
             {{{{ false ? as_markdown(\"::shell echo untaken-branch\") : \"\" }}}}\n\n\
             {{{{ as_markdown(part) }}}}\n\n\
             {{{{ as_markdown(\"{{{{ as_markdown(\\\"::shell echo doubly-nested\\\") }}}}\") }}}}\n"
        ),
    );
    let options = ComposeOptions::new().with_source_file(&path);

    let approvals = approval_set(&document, &options);

    for command in [
        format!("touch {sentinel_text}"),
        "echo unreachable-block".to_string(),
        "echo untaken-branch".to_string(),
        "echo from-frontmatter-value".to_string(),
        "echo doubly-nested".to_string(),
    ] {
        assert!(approvals.contains(&command), "missing {command:?} in {approvals:?}");
    }
    assert!(!sentinel.exists(), "preflight must not compose nested content");

    // Execution needs the Unix `touch` and `echo` executables.
    #[cfg(unix)]
    {
        let (composed, _) = compose(&document, options.with_pre_approved_commands(approvals));
        assert!(sentinel.exists(), "the approved nested command runs during compose");
        assert!(composed.content().contains("from-frontmatter-value"), "{}", composed.content());
        assert!(!composed.content().contains("unreachable-block"), "{}", composed.content());
    }
}

/// AC32: a nested shape discovery cannot know fails before any command runs,
/// whether it waits on frontmatter shell expansion or on a shell probe.
///
/// One test per shape: each case is a preflight plus a compose, and every
/// request pays a repository discovery, so all six in one test exceed the
/// slow-test budget.
fn assert_shape_fails_before_execution(name: &str, frontmatter: &str, body: &str, dependency: &str) {
    let temp = tempfile::tempdir().unwrap();
    let sentinel = temp.path().join("frontmatter-ran");
    let sentinel_text = biscuit_file::to_portable_string(&sentinel);
    let (path, document) = root_document(
        temp.path(),
        name,
        &format!("---\nran: \"$(touch {sentinel_text})\"\n{frontmatter}---\n{body}"),
    );
    let options = ComposeOptions::new().with_source_file(&path);

    let error = document.compose_preflight(&options).expect_err(name);
    let MarkdownError::ShellExpansion(inner) = &error else {
        panic!("{name}: expected a shell expansion error, got {error:?}");
    };
    let ShellExpansionError::UnevaluatedDependencyShape { dependency: found, .. } = inner.as_ref() else {
        panic!("{name}: expected a dynamic shape, got {inner:?}");
    };
    assert!(found.contains(dependency), "{name}: {found}");

    let approved: HashSet<String> = [format!("touch {sentinel_text}"), "printf '::shell echo late'".to_string()]
        .into_iter()
        .collect();
    document
        .compose_with(options.with_pre_approved_commands(approved))
        .expect_err(name);
    assert!(!sentinel.exists(), "{name}: a frontmatter command ran before the rejection");
}

#[test]
fn a_shape_pending_frontmatter_shell_expansion_fails_before_execution() {
    assert_shape_fails_before_execution(
        "pending.md",
        "part: \"$(printf '::shell echo late')\"\n",
        "{{ as_markdown(part) }}\n",
        "frontmatter.part",
    );
}

#[test]
fn a_probe_dependent_as_markdown_argument_fails_before_execution() {
    assert_shape_fails_before_execution(
        "probe-arg.md",
        "",
        "{{ as_markdown(can_execute(\"git\") ? \"::shell echo a\" : doc.ran) }}\n",
        "can_execute()",
    );
}

#[test]
fn a_probe_dependent_shell_directive_fails_before_execution() {
    assert_shape_fails_before_execution(
        "probe-shell.md",
        "",
        "::shell echo {{ has_alias(\"ll\") }}\n",
        "has_alias()",
    );
}

#[test]
fn a_probe_dependent_shell_block_fails_before_execution() {
    assert_shape_fails_before_execution(
        "probe-block.md",
        "",
        "::shell-block\necho {{ has_user_function(\"f\") }}\n::end-block\n",
        "has_user_function()",
    );
}

#[test]
fn a_nested_as_markdown_argument_fails_before_execution() {
    assert_shape_fails_before_execution(
        "nested-arg.md",
        "",
        "{{ as_markdown(as_markdown(\"::shell echo inner\")) }}\n",
        "as_markdown()",
    );
}

#[test]
fn a_probe_dependent_frontmatter_shell_value_fails_before_execution() {
    assert_shape_fails_before_execution(
        "probe-frontmatter.md",
        "probe: '$(echo {{ has_builtin_function(\"cd\") }})'\n",
        "body\n",
        "has_builtin_function()",
    );
}

/// A frontmatter `as_markdown` call composes before the root reaches its
/// pre-approved gate, so it runs the gate first: an unapproved body command
/// stops the request before the nested command executes.
#[test]
fn frontmatter_nested_effects_wait_for_the_root_gate() {
    let temp = tempfile::tempdir().unwrap();
    let sentinel = temp.path().join("nested-ran");
    let sentinel_text = biscuit_file::to_portable_string(&sentinel);
    let (path, document) = root_document(
        temp.path(),
        "root.md",
        &format!("---\nintro: '{{{{ as_markdown(\"::shell touch {sentinel_text}\") }}}}'\n---\n::shell echo unapproved\n"),
    );
    let options = ComposeOptions::new().with_source_file(&path);
    let approved: HashSet<String> = [format!("touch {sentinel_text}")].into_iter().collect();

    let error = document
        .compose_with(options.with_pre_approved_commands(approved))
        .expect_err("the unapproved body command must fail the gate");

    let MarkdownError::ShellExpansion(inner) = &error else {
        panic!("expected the typed gate error, got {error:?}");
    };
    assert!(matches!(inner.as_ref(), ShellExpansionError::NotPreApproved { command, .. } if command == "echo unapproved"), "{inner:?}");
    assert!(!sentinel.exists(), "the nested command ran before the gate");
}

/// AC32: nested content cannot widen the calling surface's remote policy.
/// With a host allowed for read-side functions but remote transclusion off,
/// a nested `::file https://…` from frontmatter or body fetches nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nested_content_never_introduces_remote_transclusion() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/remote.md"))
        .respond_with(ResponseTemplate::new(200).set_body_string("REMOTE BODY"))
        .mount(&server)
        .await;
    let url = format!("{}/remote.md", server.uri());
    let temp = tempfile::tempdir().unwrap();

    for (name, content) in [
        ("frontmatter.md", format!("---\nremote: '{{{{ as_markdown(\"::file {url}\") }}}}'\n---\nvalue=[{{{{ remote }}}}]\n")),
        ("body.md", format!("{{{{ as_markdown(\"::file {url}\") }}}}\n")),
    ] {
        let (path, document) = root_document(temp.path(), name, &content);
        let options = probe_options().with_source_file(&path).with_remote_read_config(RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        });
        let result = tokio::task::spawn_blocking(move || document.compose_with(options))
            .await
            .unwrap();
        if let Ok((composed, _)) = &result {
            assert!(!composed.content().contains("REMOTE BODY"), "{name}: {}", composed.content());
        }
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}
