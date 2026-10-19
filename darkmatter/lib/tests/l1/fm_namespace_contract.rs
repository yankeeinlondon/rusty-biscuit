//! The `fm` frontmatter namespace through public composition.
//!
//! `fm` is the reserved root for the document's own frontmatter; `doc` is an
//! ordinary property name, and so is the retired lazy root `current_env`.
//! Every test composes a real document in a fresh temporary directory and
//! asserts the rendered output (and, where the contract names one, the
//! undeclared-property advisory).

use std::path::{Path, PathBuf};

use std::sync::Arc;

use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeReport, ComposeWarning};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::json;
use tempfile::TempDir;

/// Approves every `$()` command; no test reaches a command branch.
struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// A context capturing only the date/time group, so no test observes the host
/// repository or environment.
fn options(dir: &Path, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_shell(ShellExpansionOptions {
            policy_root: Some(dir.to_path_buf()),
            ..Default::default()
        })
        .with_shell_approval_handler(Arc::new(AllowAll))
}

fn compose_with(
    content: &str,
    configure: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> Result<(String, ComposeReport), MarkdownError> {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", content);
    let options = configure(options(dir.path(), &path));
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, report)| (composed.content().trim_end().to_string(), report))
}

#[track_caller]
fn compose(content: &str) -> (String, ComposeReport) {
    compose_with(content, |options| options).unwrap_or_else(|error| panic!("{content:?} failed: {error}"))
}

#[track_caller]
fn body(content: &str) -> String {
    compose(content).0
}

/// The roots named by the report's undeclared-property advisories.
fn warned_roots(report: &ComposeReport) -> Vec<String> {
    report
        .warnings
        .iter()
        .filter(|warning| warning.code.as_deref() == Some(ComposeWarning::UNDECLARED_PROPERTY_CODE))
        .map(|warning| {
            let rest = warning.message.strip_prefix('`').expect(&warning.message);
            rest[..rest.find('`').unwrap()].to_string()
        })
        .collect()
}

// ── Reads ───────────────────────────────────────────────────────────────────

#[test]
fn bare_fm_is_a_snapshot_of_the_whole_frontmatter() {
    let rendered = body(
        "---\ntitle: X\ncount: 3\nsnap: \"{{ fm }}\"\n---\n\
         {{ is_object(fm) }} {{ fm.count }} {{ has_key(fm, 'title') }} [{{ snap.title }}]\n",
    );
    assert_eq!(rendered, "true 3 true [X]");
}

#[test]
fn qualified_and_bracket_reads_select_members() {
    let rendered = body(
        "---\ntitle: X\nrelease:\n  channel: nested\n\"release.channel\": dotted\n---\n\
         [{{ fm.title }}] [{{ fm['title'] }}] [{{ fm.release.channel }}] [{{ fm['release.channel'] }}]\n",
    );
    assert_eq!(rendered, "[X] [X] [nested] [dotted]");
}

/// `today` is a captured context key; a missing `fm.today` is still absent.
#[test]
fn an_absent_qualified_read_is_null_and_never_falls_back() {
    let rendered = body("---\ntitle: X\n---\n[{{ fm.missing }}] [{{ fm.today }}] {{ is_null(fm.missing) }}\n");
    assert_eq!(rendered, "[] [] true");
}

#[test]
fn a_property_named_fm_is_read_as_fm_fm() {
    let rendered = body("---\nfm: literal\n---\n[{{ fm.fm }}] {{ is_object(fm) }}\n");
    assert_eq!(rendered, "[literal] true");
}

#[test]
fn caller_overrides_are_visible_through_fm() {
    let (rendered, _) = compose_with(
        "---\ntitle: authored\n---\n[{{ fm.title }}] [{{ fm['title'] }}] [{{ fm.extra }}]\n",
        |options| options.with_set_overrides(json!({ "title": "override", "extra": "supplied" })),
    )
    .unwrap();
    assert_eq!(rendered, "[override] [override] [supplied]");
}

// ── Frontmatter interpolation ordering ──────────────────────────────────────

/// `b` is declared before the templated key it reads, so it resolves only if
/// `fm.a` waits for `a` exactly as a bare `a` does.
#[test]
fn a_qualified_read_waits_for_its_templated_key() {
    let rendered = body("---\nb: \"{{ fm.a }}\"\na: \"{{ base }}/spec.md\"\nbase: /root\n---\n[{{ b }}]\n");
    assert_eq!(rendered, "[/root/spec.md]");
}

/// Bare `fm` adds no dependency, so reading it from a templated key does not
/// form a cycle with that key.
#[test]
fn bare_fm_adds_no_dependency() {
    let rendered = body(
        "---\nsnap: \"{{ is_object(fm) }}\"\nkeys: \"{{ has_key(fm, 'title') }}\"\ntitle: X\n---\n\
         {{ snap }} {{ keys }}\n",
    );
    assert_eq!(rendered, "true true");
}

#[test]
fn a_literal_bracket_read_waits_for_its_templated_key() {
    let rendered = body("---\nb: \"{{ fm['a'] }}\"\na: \"{{ base }}-value\"\nbase: root\n---\n[{{ b }}]\n");
    assert_eq!(rendered, "[root-value]");
}

/// `fm['release.channel']` names the single key `release.channel`, not
/// `release`.
#[test]
fn a_dotted_literal_bracket_read_waits_for_the_single_key() {
    let rendered = body(
        "---\nlabel: \"{{ fm['release.channel'] }}\"\n\"release.channel\": \"{{ base }}-channel\"\nbase: stable\n---\n\
         [{{ label }}]\n",
    );
    assert_eq!(rendered, "[stable-channel]");
}

/// Self-reference keeps the bare-name cycle handling: the key never becomes
/// ready, so the fallback pass resolves it against its still-absent value
/// (empty) and composition terminates.
#[test]
fn self_reference_through_fm_matches_bare_self_reference() {
    for expression in ["{{ a }}", "{{ fm.a }}", "{{ fm['a'] }}"] {
        let rendered = body(&format!("---\na: \"x{expression}/tail\"\n---\n[{{{{ a }}}}]\n"));
        assert_eq!(rendered, "[x/tail]", "{expression}");
    }
}

// ── Shell tokens ────────────────────────────────────────────────────────────

/// A `fm.<name>` value branch in `$( … )` always reads the property, even
/// when an executable of that name exists on `PATH` (`echo`). The untaken
/// branch is the command that makes the body a shell expression.
#[test]
fn a_shell_token_through_fm_reads_the_property() {
    let rendered = body(
        "---\nflag: false\nbuild: from-property\necho: echo-property\n\
         out: $(flag ? cat README : fm.build)\nshout: $(flag ? cat README : fm.echo)\n---\n\
         [{{ out }}] [{{ shout }}]\n",
    );
    assert_eq!(rendered, "[from-property] [echo-property]");
}

// ── `doc` is an ordinary property ───────────────────────────────────────────

#[test]
fn doc_is_an_ordinary_property() {
    let (rendered, report) = compose(
        "---\ndoc:\n  title: X\n---\n[{{ doc.title }}] {{ is_object(doc) }} {{ has_key(doc, 'title') }} \
         {{ has_key(doc, 'doc') }} [{{ fm.doc.title }}]\n",
    );
    assert_eq!(rendered, "[X] true true false [X]");
    assert!(warned_roots(&report).is_empty(), "{:?}", report.warnings);
}

#[test]
fn an_absent_doc_renders_empty_with_the_undeclared_property_advisory() {
    let (rendered, report) = compose("---\ntitle: X\n---\n[{{ doc.title }}]\n");
    assert_eq!(rendered, "[]");
    assert_eq!(warned_roots(&report), ["doc"]);
}

#[test]
fn fm_and_doc_resolve_independently_in_one_document() {
    let rendered = body(
        "---\ntitle: X\ndoc:\n  title: userland\nfm: literal\n---\n\
         [{{ fm.title }}] [{{ fm['title'] }}] [{{ fm.fm }}] [{{ doc.title }}]\n",
    );
    assert_eq!(rendered, "[X] [X] [literal] [userland]");
}

// ── `current_env` is an ordinary property ───────────────────────────────────

/// Composes `content` with a supplied environment that is empty, so no value
/// can come from the host process.
fn compose_with_empty_env(content: &str) -> (String, ComposeReport) {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", content);
    let mut context = ComposeContext::capture_for_content(dir.path(), "");
    context.env_mut().clear();
    let options = ComposeOptions::new_with_context(context).with_source_file(path.clone());
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, report)| (composed.content().trim_end().to_string(), report))
        .unwrap_or_else(|error| panic!("{content:?} failed: {error}"))
}

#[test]
fn current_env_reads_the_frontmatter_property_not_the_environment() {
    let (rendered, report) =
        compose_with_empty_env("---\ncurrent_env:\n  HOME: userland\n---\n[{{ current_env.HOME }}]\n");
    assert_eq!(rendered, "[userland]");
    assert!(warned_roots(&report).is_empty(), "{:?}", report.warnings);
}

#[test]
fn an_absent_current_env_renders_empty_with_the_undeclared_property_advisory() {
    let (rendered, report) = compose_with_empty_env("---\ntitle: X\n---\n[{{ current_env.HOME }}]\n");
    assert_eq!(rendered, "[]");
    assert_eq!(warned_roots(&report), ["current_env"]);
}

// ── Reserved roots shadow frontmatter ───────────────────────────────────────

#[test]
fn reserved_roots_shadow_same_named_frontmatter_keys() {
    let rendered = body(
        "---\nctx:\n  note: notes\nenv: staging\ncurrent: present\nfm: literal\n---\n\
         {{ is_object(env) }} {{ env == 'staging' }} {{ is_object(fm) }} \
         [{{ fm.ctx.note }}] [{{ fm.env }}] [{{ fm.current }}] [{{ fm.fm }}]\n",
    );
    assert_eq!(rendered, "true false true [notes] [staging] [present] [literal]");
}

/// A `ctx` key is the document's own value under `fm`, never merged with the
/// runtime context it shares a name with.
#[test]
fn fm_ctx_reads_the_document_value_not_the_merged_context() {
    let rendered = body("---\nctx:\n  note: notes\n---\n[{{ fm.ctx.note }}] [{{ fm.ctx.today }}] [{{ is_null(fm.ctx.today) }}]\n");
    assert_eq!(rendered, "[notes] [] [true]");
}
