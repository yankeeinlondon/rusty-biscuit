//! The on-disk literal token `{{!data:v1:…}}` (R2).
//!
//! A token that is an entire frontmatter string decodes once, during
//! frontmatter pass 1, into a data leaf: never evaluated, never a shell
//! candidate, never scanned again. A malformed or misplaced token fails with
//! its authored location under every policy. Each test composes a real
//! document through the public pipeline.

use darkmatter::markdown::compose::expression::ExpressionError;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOptions, ShellExpansionError, collect_shell_commands,
};
use darkmatter::markdown::literal_token::{TokenError, encode, encode_yaml_scalar};
use darkmatter::markdown::{Markdown, MarkdownError, SourceRef};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// Instruction-shaped payloads, a token look-alike, and the empty string.
fn payloads() -> Vec<String> {
    vec![
        "{{ area }}".into(),
        "{{…}}".into(),
        "{{{ area }}}".into(),
        "$(echo X)".into(),
        "see {{ area }} and $(rm -rf x)".into(),
        encode("{{ area }}"),
        String::new(),
    ]
}

/// Records every approval request and allows it once.
#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl Recorder {
    fn commands(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl ShellApprovalHandler for Recorder {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.0.lock().unwrap().push(request.normalized_exact);
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn options(dir: &Path, approvals: Arc<Recorder>) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(dir.join("doc.md"))
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_shell_approval_handler(approvals)
}

fn load(dir: &Path) -> Markdown {
    Markdown::try_from(dir.join("doc.md").as_path()).unwrap()
}

fn field(markdown: &Markdown, key: &str) -> Value {
    markdown.frontmatter().as_map().get(key).cloned().unwrap_or(Value::Null)
}

/// Each payload stored as a token composes to its exact text as a top-level
/// value, as a nested leaf, through a dependent key, in the body, and in a
/// transcluded child, beside authored spans that still evaluate. Nothing is
/// evaluated or asks for approval, and the file keeps its tokens.
#[test]
fn a_decoded_token_is_data_everywhere_it_flows() {
    for payload in payloads() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("child.md"), "Child: {{ note }}\n").unwrap();
        let source = format!(
            "---\narea: claudine\nnote: {token}\nlist:\n  - plain\n  - {{ deep: {token} }}\n\
             copy: \"pre {{{{ note }}}} {{{{ area }}}}\"\n---\n\
             Body: {{{{ area }}}} {{{{ note }}}} {{{{ list[1].deep }}}}\n\n::file ./child.md\n",
            token = encode_yaml_scalar(&payload),
        );
        std::fs::write(dir.path().join("doc.md"), &source).unwrap();
        let approvals = Arc::new(Recorder::default());

        let (composed, _) = load(dir.path())
            .compose_with(&crate::request_support::request(options(dir.path(), approvals.clone())))
            .unwrap_or_else(|error| panic!("{payload:?}: {error}"));

        assert_eq!(field(&composed, "note"), json!(payload), "{payload:?}");
        assert_eq!(field(&composed, "list"), json!(["plain", { "deep": payload }]), "{payload:?}");
        assert_eq!(field(&composed, "copy"), json!(format!("pre {payload} claudine")), "{payload:?}");
        // Composition trims trailing spaces, which the empty payload leaves.
        let body = composed.content();
        let has_line = |line: String| body.lines().any(|seen| seen == line.trim_end());
        assert!(has_line(format!("Body: claudine {payload} {payload}")), "{payload:?}: {body}");
        assert!(has_line(format!("Child: {payload}")), "{payload:?}: {body}");
        assert!(approvals.commands().is_empty(), "{payload:?}: {:?}", approvals.commands());
        assert_eq!(std::fs::read_to_string(dir.path().join("doc.md")).unwrap(), source);
    }
}

/// Loaders keep a token encoded; the public decode path returns the text.
/// Composing the loaded document again decodes the unchanged token again.
#[test]
fn loaders_keep_tokens_and_readers_decode_them() {
    let dir = TempDir::new().unwrap();
    let token = encode("fixed {{ area }} parsing");
    std::fs::write(
        dir.path().join("doc.md"),
        format!("---\nsummary: \"{token}\"\nnested: [\"{token}\"]\n---\nx\n"),
    )
    .unwrap();

    let loaded = load(dir.path());
    assert_eq!(field(&loaded, "summary"), json!(token));
    let decoded = loaded.frontmatter().decoded_literal_tokens().unwrap();
    assert_eq!(decoded["summary"], json!("fixed {{ area }} parsing"));
    assert_eq!(decoded["nested"], json!(["fixed {{ area }} parsing"]));

    for _ in 0..2 {
        let (composed, _) = load(dir.path())
            .compose_with(&crate::request_support::request(options(dir.path(), Arc::default())))
            .unwrap();
        assert_eq!(field(&composed, "summary"), json!("fixed {{ area }} parsing"));
    }
}

/// `frontmatter()` and `markdown_title()` read another document's stored
/// tokens as their text, for a property, a nested leaf, and the whole-map form.
/// What they return is data: nothing evaluates, nothing asks for approval, and
/// a malformed token comes back raw rather than failing the expression.
#[test]
fn reading_another_documents_frontmatter_decodes_its_tokens() {
    for payload in payloads() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("log.md"),
            format!(
                "---\ntitle: {token}\nmessage_to_agent: {token}\nitems: [plain, {token}]\n\
                 broken: \"{{{{!data:v1:%%}}}}\"\n---\nx\n",
                token = encode_yaml_scalar(&payload),
            ),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("doc.md"),
            "---\narea: claudine\n\
             message: \"{{ frontmatter('./log.md', 'message_to_agent') }}\"\n\
             items: \"{{ frontmatter('./log.md', 'items') }}\"\n\
             whole: \"{{ frontmatter('./log.md') }}\"\n\
             title: \"{{ markdown_title('./log.md') }}\"\n\
             broken: \"{{ frontmatter('./log.md', 'broken') }}\"\n---\n\
             Msg: {{ message }} in {{ area }}\n",
        )
        .unwrap();
        let approvals = Arc::new(Recorder::default());

        let (composed, _) = load(dir.path())
            .compose_with(&crate::request_support::request(options(dir.path(), approvals.clone())))
            .unwrap_or_else(|error| panic!("{payload:?}: {error}"));

        assert_eq!(field(&composed, "message"), json!(payload), "{payload:?}");
        assert_eq!(field(&composed, "items"), json!(["plain", payload]), "{payload:?}");
        assert_eq!(field(&composed, "whole")["message_to_agent"], json!(payload), "{payload:?}");
        assert_eq!(field(&composed, "whole")["items"], json!(["plain", payload]), "{payload:?}");
        assert_eq!(field(&composed, "title"), json!(payload), "{payload:?}");
        assert_eq!(field(&composed, "broken"), json!("{{!data:v1:%%}}"), "{payload:?}");
        let body = composed.content();
        let expected = format!("Msg: {payload} in claudine");
        assert!(body.lines().any(|line| line == expected.trim_end()), "{payload:?}: {body}");
        assert!(approvals.commands().is_empty(), "{payload:?}: {:?}", approvals.commands());
    }
}

/// A decoded `$( … )` is not a shell candidate for preflight, for approval, or
/// for the pre-approval gate; an authored one beside it still is. The gate also
/// accepts a whole-value `$( … )` an expression produced (Phase 2 R1.2).
#[test]
fn a_decoded_command_is_not_a_shell_candidate() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("doc.md"),
        format!(
            "---\ncmd: {}\nmade: \"{{{{ '$(' + 'echo Z)' }}}}\"\nout: \"$(echo Y)\"\n---\n\
             C={{{{ cmd }}}} M={{{{ made }}}} O={{{{ out }}}}\n",
            encode_yaml_scalar("$(echo X)")
        ),
    )
    .unwrap();

    let collected = collect_shell_commands(&load(dir.path()), &crate::request_support::request(options(dir.path(), Arc::default()))).unwrap();
    assert_eq!(collected.iter().map(|entry| entry.normalized.as_str()).collect::<Vec<_>>(), ["echo Y"]);

    let approvals = Arc::new(Recorder::default());
    let (composed, _) = load(dir.path()).compose_with(&crate::request_support::request(options(dir.path(), approvals.clone()))).unwrap();
    assert!(composed.content().contains("C=$(echo X) M=$(echo Z) O=Y"), "{}", composed.content());
    assert_eq!(approvals.commands(), ["echo Y"]);

    let approved: HashSet<String> = load(dir.path())
        .compose_preflight(&crate::request_support::request(options(dir.path(), Arc::default())))
        .unwrap()
        .approval_set()
        .into_iter()
        .collect();
    assert_eq!(approved, HashSet::from(["echo Y".to_string()]));
    let (gated, _) = load(dir.path())
        .compose_with(&crate::request_support::request(options(dir.path(), Arc::default()).with_pre_approved_commands(approved)))
        .unwrap_or_else(|error| panic!("the gate must accept the document: {error}"));
    assert!(gated.content().contains("C=$(echo X) M=$(echo Z) O=Y"), "{}", gated.content());
}

/// Triple braces show the token spelling as authored text, converted once and
/// never decoded; a backslash-escaped spelling is text too.
#[test]
fn escaped_spellings_are_text() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("doc.md"),
        "---\nshown: \"{{{!data:v1:YQ}}}\"\n---\nA={{ shown }} B=\\{{!data:v1:YQ}}\n",
    )
    .unwrap();
    let (composed, _) = load(dir.path()).compose_with(&crate::request_support::request(options(dir.path(), Arc::default()))).unwrap();
    assert_eq!(field(&composed, "shown"), json!("{{!data:v1:YQ}}"));
    assert!(composed.content().contains(r"A={{!data:v1:YQ}} B=\{{!data:v1:YQ}}"), "{}", composed.content());
}

/// Every malformed or misplaced token fails with a typed cause, its receiving
/// key, and its one-based authored line and column, whatever `fail_fast` says.
#[test]
fn malformed_tokens_report_their_location() {
    struct Case {
        source: &'static str,
        key: Option<&'static str>,
        construct: &'static str,
        position: (usize, usize),
        cause: TokenError,
    }
    let cases = [
        Case {
            source: "---\ntitle: \"{{!data:v2:YQ}}\"\n---\nx\n",
            key: Some("title"),
            construct: "{{!data:v2:YQ}}",
            position: (2, 9),
            cause: TokenError::UnsupportedVersion { version: "v2".into() },
        },
        Case {
            source: "---\ntitle: \"{{!data:v1:YQ\"\n---\nx\n",
            key: Some("title"),
            construct: "{{!data:v1:YQ",
            position: (2, 9),
            cause: TokenError::Unterminated,
        },
        Case {
            source: "---\ntitle: \"see {{!data:v1:YQ}}\"\n---\nx\n",
            key: Some("title"),
            construct: "{{!data:v1:YQ}}",
            position: (2, 13),
            cause: TokenError::Embedded,
        },
        Case {
            source: "---\ntitle: \" {{!data:v1:YQ}}\"\n---\nx\n",
            key: Some("title"),
            construct: "{{!data:v1:YQ}}",
            position: (2, 10),
            cause: TokenError::Embedded,
        },
        Case {
            source: "---\nnotes:\n  - ok\n  - \"{{!data:v1:Y}}\"\n---\nx\n",
            key: Some("notes"),
            construct: "{{!data:v1:Y}}",
            position: (4, 6),
            cause: TokenError::InvalidPayload {
                detail: "Invalid input length: 1".into(),
            },
        },
        Case {
            source: "---\ntitle: t\n---\nbody {{!data:v1:YQ}} here\n",
            key: None,
            construct: "{{!data:v1:YQ}}",
            position: (4, 6),
            cause: TokenError::Embedded,
        },
        Case {
            source: "---\ntitle: t\n---\nbody {{ 'a {{!data:v1:YQ}}' }}\n",
            key: None,
            construct: "{{ 'a {{!data:v1:YQ}}' }}",
            position: (4, 6),
            cause: TokenError::Embedded,
        },
    ];

    for case in cases {
        for fail_fast in [false, true] {
            let dir = TempDir::new().unwrap();
            std::fs::write(dir.path().join("doc.md"), case.source).unwrap();
            let error = load(dir.path())
                .compose_with(&crate::request_support::request(options(dir.path(), Arc::default()).with_fail_fast(fail_fast)))
                .map(|(composed, _)| composed.content().to_string())
                .expect_err(case.source);
            let MarkdownError::Interpolation { key, source, cause, .. } = &error else {
                panic!("{}: expected a typed interpolation error, got {error:?}", case.source);
            };
            assert_eq!(key.as_deref(), case.key, "{}", case.source);
            let ExpressionError::MalformedLiteralToken(reason) = cause.as_ref() else {
                panic!("{}: expected a malformed token, got {cause:?}", case.source);
            };
            assert_eq!(reason, &case.cause, "{}", case.source);
            let SourceRef::OnDiskSpan { context, span } = source.as_ref() else {
                panic!("{}: expected the authored span, got {source:?}", case.source);
            };
            assert_eq!(&context.content[span.range()], case.construct, "{}", case.source);
            assert_eq!((span.line(), span.column()), case.position, "{}", case.source);
            assert!(
                error.to_string().contains("literal token") || cause.to_string().contains("literal token"),
                "{error}"
            );
        }
    }
}

/// Lenient callers cannot keep a token: preflight collection fails on one in
/// the body instead of approving the raw text (spike S3 E9).
#[test]
fn preflight_fails_on_a_body_token() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("doc.md"), "::shell echo {{!data:v1:YQ}}\n").unwrap();
    let error = collect_shell_commands(&load(dir.path()), &crate::request_support::request(options(dir.path(), Arc::default())))
        .expect_err("a token in a body command is malformed");
    assert!(error.to_string().contains("literal token"), "{error}");
}
