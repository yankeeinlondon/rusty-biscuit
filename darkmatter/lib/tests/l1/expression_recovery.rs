//! `try(expr, fallback)` on every composition surface: which failures
//! recover, which propagate, and what the composed document and its
//! warnings look like afterward.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_file::{FileReferenceError, FileResolutionContext};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{Diagnostic, ExpressionError, FileRefFailure, UNCERTAIN_CALL_CODE};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use darkmatter::markdown::compose::policy::BlockingPolicy;
use darkmatter::markdown::compose::{
    ComposeContext, ComposeFailure, ComposeOptions, ComposeReport, ComposeRequest, ContextGroup, DataRuntime,
};
use tempfile::TempDir;

const UNSUPPORTED: &str = "dm.data.unsupported_format";

// ── Fixtures ────────────────────────────────────────────────────────────────

fn write(dir: &Path, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    std::fs::write(&path, content).expect("write fixture");
    path
}

type Composed = Result<(Markdown, ComposeReport), Box<ComposeFailure>>;

/// Composes `content` as `doc.md` in `dir`, with the request anchored there.
fn compose_in(dir: &Path, content: &str, options: ComposeOptions) -> Composed {
    let path = write(dir, "doc.md", content);
    let request = crate::request_support::request_at(dir, options.with_source_file(&path));
    Markdown::try_from(path.as_path()).expect("document loads").compose_with_warnings(&request)
}

fn rendered(dir: &Path, content: &str, options: ComposeOptions) -> (String, ComposeReport) {
    match compose_in(dir, content, options) {
        Ok((composed, report)) => (composed.content().trim().to_string(), report),
        Err(failure) => panic!("{content:?} failed: {:?}", failure.error),
    }
}

fn failure_of(dir: &Path, content: &str, options: ComposeOptions) -> Box<ComposeFailure> {
    match compose_in(dir, content, options) {
        Ok((composed, _)) => panic!("{content:?} composed: {:?}", composed.content()),
        Err(failure) => failure,
    }
}

/// Every expression error in `failure`'s cause chain, callbacks unwrapped.
fn expression_errors(failure: &ComposeFailure) -> Vec<&ExpressionError> {
    let mut found = Vec::new();
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&failure.error);
    while let Some(error) = current {
        let expression = error
            .downcast_ref::<ExpressionError>()
            .or_else(|| error.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref));
        if let Some(expression) = expression {
            found.push(expression.without_callbacks());
        }
        current = error.source();
    }
    found
}

/// The innermost typed expression error of `failure`, callbacks unwrapped.
fn cause(failure: &ComposeFailure) -> &ExpressionError {
    expression_errors(failure)
        .pop()
        .unwrap_or_else(|| panic!("no expression error in {:?}", failure.error))
}

/// The one record `failure` reports.
fn record(failure: &ComposeFailure) -> Diagnostic {
    let records = error_diagnostics(&failure.error);
    assert_eq!(records.len(), 1, "{records:#?}");
    records.into_iter().next().expect("one record")
}

/// Asserts `failure` reports `code` at the authored `(line, column)` of
/// `doc.md`, and returns the record.
#[track_caller]
fn assert_reported_at(failure: &ComposeFailure, code: &str, (line, column): (usize, usize)) -> Diagnostic {
    let record = record(failure);
    assert_eq!(record.code, code, "{record:#?}");
    let source = record.source.as_ref().expect("a located record");
    assert!(source.file.as_ref().is_some_and(|file| file.ends_with("doc.md")), "{source:?}");
    assert_eq!((source.line, source.column), (Some(line), Some(column)), "{record:#?}");
    record
}

/// Approves every command once and records what it was asked.
#[derive(Default)]
struct Recorder(Mutex<Vec<ShellApprovalRequest>>);

impl Recorder {
    fn commands(&self) -> Vec<String> {
        self.0.lock().unwrap().iter().map(|request| request.normalized_exact.clone()).collect()
    }
}

impl ShellApprovalHandler for Recorder {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.0.lock().unwrap().push(request);
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn shell_options(dir: &Path, recorder: &Arc<Recorder>) -> ComposeOptions {
    ComposeOptions::new()
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_shell_approval_handler(recorder.clone())
}

// ── Every surface ───────────────────────────────────────────────────────────

/// One surface, composed three times with `SRC` naming a present file, a
/// missing one, and an unsupported `.xyz` file that exists. The present and
/// missing runs compose to the given text; the unsupported run fails with
/// `dm.data.unsupported_format` at the authored position instead of
/// composing the fallback.
mod try_recovers_on_every_expression_surface {
    use super::*;

    /// `present` is the existing file the control run reads; every data
    /// file holds a one-item list (`loaded.yaml`: `LOADED`, `target.yaml`:
    /// `./loaded.md`).
    fn surface(template: &str, present: (&str, &str), missing: &str, at: (usize, usize)) {
        surface_with(template, present, missing, at, |_| ComposeOptions::new());
    }

    fn surface_with(
        template: &str,
        present: (&str, &str),
        missing: &str,
        at: (usize, usize),
        options: impl Fn(&Path) -> ComposeOptions,
    ) {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "loaded.yaml", "- LOADED\n");
        write(dir.path(), "target.yaml", "- ./loaded.md\n");
        write(dir.path(), "data.xyz", "- LOADED\n");
        write(dir.path(), "loaded.md", "loaded child\n");
        write(dir.path(), "fallback.md", "FALLBACK child\n");
        let source = |file: &str| template.replace("SRC", file);
        for (file, expected) in [present, ("missing.yaml", missing)] {
            let (text, report) = rendered(dir.path(), &source(file), options(dir.path()));
            assert_eq!(text, expected, "{file}");
            assert!(report.warnings.is_empty(), "{file}: {:?}", report.warnings);
        }
        let failure = failure_of(dir.path(), &source("data.xyz"), options(dir.path()));
        assert_reported_at(&failure, UNSUPPORTED, at);
        let data = cause(&failure).data_failure().unwrap_or_else(|| panic!("{:?}", failure.error));
        assert_eq!(data.code(), Some(UNSUPPORTED));
    }

    #[test]
    fn body_interpolation() {
        surface(
            "intro\n\nx {{ try(load_data('SRC', 'string[]')[0], 'FALLBACK') }}\n",
            ("loaded.yaml", "intro\n\nx LOADED"),
            "intro\n\nx FALLBACK",
            (3, 3),
        );
    }

    #[test]
    fn whole_value_frontmatter_pass_one() {
        surface(
            "---\nnote: \"{{ try(load_data('SRC', 'string[]')[0], 'FALLBACK') }}\"\n---\n{{ note }}\n",
            ("loaded.yaml", "LOADED"),
            "FALLBACK",
            (2, 8),
        );
    }

    #[test]
    fn conditional_page_block() {
        // The block's own failure is located at its `::block` opener.
        surface(
            "::block when=\"try(load_data('SRC', 'string[]')[0] != 'LOADED', true)\"\nFALLBACK shown\n::end-block\n\nend\n",
            ("loaded.yaml", "end"),
            "FALLBACK shown\n\nend",
            (1, 1),
        );
    }

    #[test]
    fn transclusion_condition() {
        surface(
            "::file ./fallback.md when=\"try(load_data('SRC', 'string[]')[0] != 'LOADED', true)\"\n\nend\n",
            ("loaded.yaml", "end"),
            "FALLBACK child\n\n\nend",
            (1, 1),
        );
    }

    #[test]
    fn transclusion_target_expression() {
        surface(
            "::file {{ try(load_data('SRC', 'string[]')[0], './fallback.md') }}\n",
            ("target.yaml", "loaded child"),
            "FALLBACK child",
            (1, 8),
        );
    }

    #[test]
    fn inline_map_callback() {
        surface(
            "{{ map([1, 2], n => try(load_data('SRC', 'string[]')[0], 'FALLBACK')) }}\n",
            ("loaded.yaml", "[\"LOADED\",\"LOADED\"]"),
            "[\"FALLBACK\",\"FALLBACK\"]",
            (1, 1),
        );
    }

    #[test]
    fn inline_filter_callback() {
        surface(
            "{{ filter([1, 2], n => try(load_data('SRC', 'string[]')[0] != 'LOADED', n == 2)) }}\n",
            ("loaded.yaml", "[]"),
            "[2]",
            (1, 1),
        );
    }

    /// `note` reads the whole-value command `rev`, so pass 1 defers it and
    /// pass 2 evaluates it after the command ran.
    #[test]
    fn whole_value_frontmatter_pass_two() {
        surface_with(
            "---\nrev: $(git --version)\nnote: \"{{ rev ? try(load_data('SRC', 'string[]')[0], 'FALLBACK') : 'none' }}\"\n---\n{{ note }}\n",
            ("loaded.yaml", "LOADED"),
            "FALLBACK",
            (3, 8),
            |dir| shell_options(dir, &Arc::new(Recorder::default())),
        );
    }

    #[test]
    fn map_region_callback() {
        surface(
            "---\n$schema:\n  items: string[]\nitems: [a]\n---\n::map items \"i => try(load_data('SRC', 'string[]')[0], 'FALLBACK')\"\n- {{ i }}\n::end-map\n",
            ("loaded.yaml", "- LOADED"),
            "- FALLBACK",
            (6, 13),
        );
    }

    #[test]
    fn filter_region_predicate() {
        surface(
            "---\n$schema:\n  items: string[]\nitems: [a, b]\n---\n::filter items \"i => try(load_data('SRC', 'string[]')[0] != 'LOADED', i == 'b')\"\n- {{ i }}\n::end-filter\n\nend\n",
            ("loaded.yaml", "end"),
            "- b\n\nend",
            (6, 16),
        );
    }

    #[test]
    fn region_when_condition() {
        surface(
            "---\n$schema:\n  items: string[]\nitems: [a]\n---\n::map items when=\"try(load_data('SRC', 'string[]')[0] != 'LOADED', true)\"\n- FALLBACK {{ i }}\n::end-map\n\nend\n",
            ("loaded.yaml", "end"),
            "- FALLBACK a\n\nend",
            (6, 1),
        );
    }

    /// A `::shell` template's hole: what it evaluates to is what the
    /// approval handler is asked to run. The program does not exist, so
    /// nothing is launched.
    #[test]
    fn shell_template_expression() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "loaded.yaml", "- LOADED\n");
        write(dir.path(), "data.xyz", "- LOADED\n");
        let source = |file: &str| format!("::shell dm-missing-program {{{{ try(load_data('{file}', 'string[]')[0], 'FALLBACK') }}}}\n");
        for (file, asked) in [("loaded.yaml", "dm-missing-program LOADED"), ("missing.yaml", "dm-missing-program FALLBACK")] {
            let recorder = Arc::new(Recorder::default());
            let failure = failure_of(dir.path(), &source(file), shell_options(dir.path(), &recorder));
            assert!(
                matches!(&failure.error, darkmatter::markdown::MarkdownError::ShellExpansion(error) if matches!(**error, ShellExpansionError::CommandNotFound { .. })),
                "{file}: {:?}",
                failure.error
            );
            assert_eq!(recorder.commands(), [asked], "{file}");
        }
        let recorder = Arc::new(Recorder::default());
        let failure = failure_of(dir.path(), &source("data.xyz"), shell_options(dir.path(), &recorder));
        assert_reported_at(&failure, UNSUPPORTED, (1, 28));
        assert!(recorder.commands().is_empty(), "nothing was offered to run: {:?}", recorder.commands());
    }
}

// ── Surfaces that keep expressions off ──────────────────────────────────────

/// A collection source is a name, not an expression: `try` there is the
/// same grammar error as any other call, and nothing is read.
#[test]
fn try_is_not_enabled_where_expressions_are_disabled() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "loaded.yaml", "- LOADED\n");
    let region = |source: &str| format!("---\n$schema:\n  items: string[]\nitems: [a]\n---\n::map {source}\n- {{{{ i }}}}\n::end-map\n");
    let runtime = DataRuntime::new();
    let options = || ComposeOptions::new().with_data_runtime(runtime.clone());
    let control = failure_of(dir.path(), &region("length(items)"), options());
    let guarded = failure_of(dir.path(), &region("try(load_data('loaded.yaml', 'string[]'), items)"), options());
    assert_eq!(record(&guarded).code, record(&control).code, "{:?}", guarded.error);
    assert!(expression_errors(&guarded).is_empty(), "never evaluated: {:?}", guarded.error);
    assert_eq!(runtime.stats().reads, 0, "{:?}", runtime.stats());

    // A literal `{{{ … }}}` stays text.
    let (text, report) = rendered(dir.path(), "{{{ try(load_data('loaded.yaml', 'string[]'), 'FALLBACK') }}}\n", options());
    assert_eq!(text, "{{ try(load_data('loaded.yaml', 'string[]'), 'FALLBACK') }}");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(runtime.stats().reads, 0, "{:?}", runtime.stats());
}

// ── Callbacks ───────────────────────────────────────────────────────────────

const THREE_FILES: &str = "---\n$schema:\n  items: string[]\nitems: [loaded.yaml, missing.yaml, other.yaml]\n---\n";

fn three_files() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "loaded.yaml", "- LOADED\n");
    write(dir.path(), "other.yaml", "- OTHER\n");
    dir
}

#[test]
fn whole_map_recovery_replaces_the_result() {
    let dir = three_files();
    let mapped = "map(items, i => load_data(i, 'string[]')[0])";
    let (text, _) = rendered(dir.path(), &format!("{THREE_FILES}{{{{ try({mapped}, ['FALLBACK']) }}}}\n"), ComposeOptions::new());
    assert_eq!(text, "[\"FALLBACK\"]", "the earlier item's value is discarded with the map");

    let failure = failure_of(dir.path(), &format!("{THREE_FILES}{{{{ {mapped} }}}}\n"), ComposeOptions::new());
    let missing = cause(&failure).file_reference_diagnostic().unwrap_or_else(|| panic!("{:?}", failure.error));
    assert_eq!(missing.reference, "missing.yaml");
}

#[test]
fn per_item_recovery_continues() {
    let dir = three_files();
    let (text, report) = rendered(
        dir.path(),
        &format!("{THREE_FILES}{{{{ map(items, i => try(load_data(i, 'string[]')[0], 'FALLBACK')) }}}}\n"),
        ComposeOptions::new(),
    );
    assert_eq!(text, "[\"LOADED\",\"FALLBACK\",\"OTHER\"]");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

/// An unsupported item inside a per-item `try` propagates with the item's
/// position, inline and in a `::map` region.
#[test]
fn propagated_callback_error_keeps_item_position() {
    let dir = three_files();
    write(dir.path(), "data.xyz", "- LOADED\n");
    let front = THREE_FILES.replace("missing.yaml", "data.xyz");
    let inline = format!("{front}{{{{ map(items, i => try(load_data(i, 'string[]')[0], 'FALLBACK')) }}}}\n");
    let failure = failure_of(dir.path(), &inline, ComposeOptions::new());
    let found = assert_reported_at(&failure, UNSUPPORTED, (6, 1));
    assert_eq!(found.iteration_path.to_string(), "map[1]");

    let region = format!("{front}::map items \"i => try(load_data(i, 'string[]')[0], 'FALLBACK')\"\n- {{{{ i }}}}\n::end-map\n");
    let failure = failure_of(dir.path(), &region, ComposeOptions::new());
    let found = assert_reported_at(&failure, UNSUPPORTED, (6, 13));
    assert_eq!(found.iteration_path.to_string(), "items[1]");
}

#[test]
fn fallback_sees_callback_locals_and_captures() {
    let dir = TempDir::new().unwrap();
    let (text, report) = rendered(
        dir.path(),
        "---\ntitle: T\n---\n{{ map(['o'], o => map(['missing.yaml'], i => try(load_data(i, 'string[]'), [o, i, title]))) }}\n",
        ComposeOptions::new(),
    );
    assert_eq!(text, "[[[\"o\",\"missing.yaml\",\"T\"]]]");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn inserted_fallback_text_is_not_rescanned() {
    let dir = TempDir::new().unwrap();
    let (text, report) = rendered(
        dir.path(),
        "---\ntitle: T\nnote: \"{{ try(load_data('missing.yaml', 'string[]'), '{{ title }}') }}\"\n---\n\
         {{ title }} {{ note }} {{ try(load_data('missing.yaml', 'string[]'), '{{ title }}') }}\n",
        ComposeOptions::new(),
    );
    assert_eq!(text, "T {{ title }} {{ title }}");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

// ── Failures `try` never absorbs ────────────────────────────────────────────

/// A fallback whose run is observable: it reads `sentinel.yaml`, which the
/// read set then names.
const SENTINEL: &str = "load_data('sentinel.yaml', 'string[]')";

fn sentinel_read(read_set: &[String]) -> bool {
    read_set.iter().any(|entry| entry.ends_with("sentinel.yaml"))
}

/// Composes `content` with `SENTINEL` spelled out (it must fail) and checks
/// the fallback never ran. A recovered miss in the same directory first
/// shows the sentinel is visible when it does run.
fn propagated(dir: &Path, content: &str, options: impl Fn() -> ComposeOptions) -> Box<ComposeFailure> {
    write(dir, "sentinel.yaml", "- SENTINEL\n");
    let control = format!("{{{{ try(load_data('missing.yaml', 'string[]'), {SENTINEL}) }}}}\n");
    let (composed, report) = compose_in(dir, &control, options()).unwrap_or_else(|failure| panic!("{:?}", failure.error));
    assert_eq!(composed.content().trim(), "[\"SENTINEL\"]");
    assert!(sentinel_read(&report.read_set), "{:?}", report.read_set);

    let failure = failure_of(dir, &content.replace("SENTINEL", SENTINEL), options());
    assert!(!sentinel_read(&failure.read_set), "the fallback ran: {:?}", failure.read_set);
    failure
}

#[test]
fn parse_error_inside_try_is_fatal() {
    let dir = TempDir::new().unwrap();
    for content in ["{{ try(1 +, SENTINEL) }}\n", "{{ try(load_data('missing.yaml', 'string[]'), (1 +)) }}\n"] {
        let failure = propagated(dir.path(), content, ComposeOptions::new);
        assert!(matches!(cause(&failure), ExpressionError::Parse(_)), "{content}: {:?}", failure.error);
    }
}

/// The contract comes from frontmatter, so only binding at run time can
/// reject it. (A number or list contract converts to text and fails as a
/// contract instead; an object cannot convert.)
#[test]
fn runtime_binding_failure_propagates() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "loaded.yaml", "- LOADED\n");
    let failure = propagated(
        dir.path(),
        "---\ncontract: {a: 1}\n---\n{{ try(load_data('loaded.yaml', contract), SENTINEL) }}\n",
        ComposeOptions::new,
    );
    assert!(matches!(cause(&failure), ExpressionError::Call(_)), "{:?}", failure.error);
    assert!(!cause(&failure).is_recoverable_read_failure());
}

#[test]
fn arithmetic_failure_propagates() {
    let dir = TempDir::new().unwrap();
    let failure = propagated(dir.path(), "---\nflag: true\n---\n{{ try(flag + 1, SENTINEL) }}\n", ComposeOptions::new);
    assert!(matches!(cause(&failure), ExpressionError::Arithmetic { op: "Addition" }), "{:?}", failure.error);
}

#[test]
fn iteration_limit_propagates() {
    let dir = TempDir::new().unwrap();
    let content = "{{ try(map([1, 2, 3], n => n), SENTINEL) }}\n";
    let (text, _) = rendered(dir.path(), &content.replace("SENTINEL", "[]"), ComposeOptions::new());
    assert_eq!(text, "[1,2,3]", "control: within the allowance");
    // Three visits fit: the control's own `try` spends none.
    let options = || ComposeOptions::new().with_max_iteration_visits(std::num::NonZeroU64::new(3).unwrap());
    let failure = propagated(dir.path(), &content.replace("3]", "3, 4]"), options);
    assert!(matches!(cause(&failure), ExpressionError::IterationLimit(_)), "{:?}", failure.error);
}

#[test]
fn missing_request_context_is_fatal() {
    let dir = TempDir::new().unwrap();
    let options = || {
        let context = ComposeContext::capture_for_content(dir.path(), "");
        assert!(!context.capture_requirements().contains(ContextGroup::Repo));
        ComposeOptions::new_with_context(context)
    };
    let failure = propagated(dir.path(), "{{ try(ctx.repo_root, SENTINEL) }}\n", options);
    assert!(matches!(cause(&failure), ExpressionError::ContextNotCaptured { .. }), "{:?}", failure.error);
}

/// A `file` parameter refuses a reference that cannot name a path (`~user`,
/// an unsupported scheme): before evaluation when it is written, and at
/// binding when it is computed. Neither reaches the reader, so neither is a
/// file-reference miss.
#[test]
fn malformed_reference_is_fatal() {
    let dir = TempDir::new().unwrap();
    for reference in ["~someone/data.yaml", "ftp://example.invalid/data.yaml"] {
        let literal = format!("{{{{ try(load_data('{reference}', 'string[]'), SENTINEL) }}}}\n");
        let failure = propagated(dir.path(), &literal, ComposeOptions::new);
        assert!(matches!(cause(&failure), ExpressionError::CallCheck(_)), "{reference}: {:?}", failure.error);

        let computed = format!("---\nref: '{reference}'\n---\n{{{{ try(load_data(ref, 'string[]'), SENTINEL) }}}}\n");
        let failure = propagated(dir.path(), &computed, ComposeOptions::new);
        assert!(matches!(cause(&failure), ExpressionError::Call(_)), "{reference}: {:?}", failure.error);
        assert!(cause(&failure).file_reference_diagnostic().is_none());
    }
}

/// A relative reference that leaves the repository is labeled `NotFound`;
/// its retained cause, not the label, keeps it fatal.
#[test]
fn tree_escape_is_fatal() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    write(temp.path(), "outside.yaml", "- OUTSIDE\n");
    write(&repo, "sentinel.yaml", "- SENTINEL\n");
    let doc = write(&repo, "doc.md", "");
    let compose = |content: &str| {
        std::fs::write(&doc, content.replace("SENTINEL", SENTINEL)).unwrap();
        let context = FileResolutionContext::from_snapshot(&repo, None, HashMap::new()).with_repository_root(&repo);
        let request = ComposeRequest::with_context(ComposeOptions::new().with_source_file(&doc), context).unwrap();
        Markdown::try_from(doc.as_path()).unwrap().compose_with_warnings(&request)
    };
    let (control, report) = compose("{{ try(load_data('missing.yaml', 'string[]'), SENTINEL) }}\n").expect("an in-tree miss recovers");
    assert_eq!(control.content().trim(), "[\"SENTINEL\"]");
    assert!(sentinel_read(&report.read_set), "{:?}", report.read_set);

    let failure = compose("{{ try(load_data('../outside.yaml', 'string[]'), SENTINEL) }}\n").expect_err("an escape is fatal");
    assert!(!sentinel_read(&failure.read_set), "{:?}", failure.read_set);
    let diagnostic = cause(&failure).file_reference_diagnostic().unwrap_or_else(|| panic!("{:?}", failure.error));
    assert_eq!(diagnostic.kind, FileRefFailure::NotFound);
    assert!(
        matches!(diagnostic.source.as_deref(), Some(FileReferenceError::RelativeTreeEscape { .. })),
        "{diagnostic:?}"
    );
}

/// `&` names the repository root, and there is none.
#[test]
fn glob_reference_failure_is_fatal() {
    let dir = TempDir::new().unwrap();
    let failure = propagated(dir.path(), "{{ try(find_files('&*.md'), SENTINEL) }}\n", ComposeOptions::new);
    assert!(matches!(cause(&failure), ExpressionError::GlobReference { .. }), "{:?}", failure.error);
}

/// A provider query naming a remote the repository does not configure fails
/// in the provider layer before any request is made.
#[test]
fn provider_failure_is_fatal() {
    let dir = TempDir::new().unwrap();
    gix::init(dir.path()).expect("init a repository");
    let failure = propagated(dir.path(), "{{ try(pr_list({remote: 'nowhere'}), SENTINEL) }}\n", ComposeOptions::new);
    assert!(
        matches!(cause(&failure), ExpressionError::Provider { function, .. } if function == "pr_list"),
        "{:?}",
        failure.error
    );
}

// ── Nested composition ──────────────────────────────────────────────────────

/// A malformed file `try` would recover when read directly is fatal when a
/// nested composition read it.
#[test]
fn nested_composition_data_failure_is_fatal() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "bad.json", "[1,");
    let (text, _) = rendered(dir.path(), "{{ try(load_data('bad.json', 'number[]'), 'direct') }}\n", ComposeOptions::new());
    assert_eq!(text, "direct", "control: read directly, the malformed file recovers");

    let nested = "{{ try(as_markdown(\"{{ load_data('bad.json', 'number[]') }}\"), SENTINEL) }}\n";
    let failure = propagated(dir.path(), nested, ComposeOptions::new);
    let ExpressionError::NestedData(data) = cause(&failure) else {
        panic!("expected nested data, got {:?}", failure.error);
    };
    assert_eq!(data.code(), Some("dm.data.malformed"));
}

/// A nested child's warning that the policy blocks stops the document even
/// though `try` wraps the nested call. `min(label, 5)` over a string
/// declaration is an uncertain call: a warning, not a failure.
#[test]
fn pending_nested_blocking_diagnostic_stays_fatal() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", "---\n$schema:\n  label: string\nlabel: \"7\"\n---\nchild {{ min(label, 5) }}\n");
    write(dir.path(), "sentinel.yaml", "- SENTINEL\n");
    let content = format!("{{{{ try(as_markdown('::file ./child.md'), {SENTINEL}) }}}}\n");
    let (text, report) = rendered(dir.path(), &content, ComposeOptions::new());
    assert_eq!(text, "child 5", "control: the warning alone is no failure");
    assert!(report.warnings.iter().any(|warning| warning.code.as_deref() == Some(UNCERTAIN_CALL_CODE)), "{:?}", report.warnings);

    let deny = ComposeOptions::new().with_blocking_policy(BlockingPolicy::default().deny_code(UNCERTAIN_CALL_CODE));
    let failure = failure_of(dir.path(), &content, deny);
    let darkmatter::markdown::MarkdownError::BlockedDiagnostics { blocking, .. } = &failure.error else {
        panic!("expected blocked diagnostics, got {:?}", failure.error);
    };
    assert_eq!(blocking[0].code.as_deref(), Some(UNCERTAIN_CALL_CODE), "{blocking:#?}");
    assert!(!sentinel_read(&failure.read_set), "the fallback ran: {:?}", failure.read_set);
}

// ── Warnings ────────────────────────────────────────────────────────────────

const UNDECLARED: &str = "dm.expression.undeclared_property";

fn undeclared(report: &[darkmatter::markdown::compose::ComposeWarning], name: &str) -> bool {
    report.iter().any(|warning| warning.code.as_deref() == Some(UNDECLARED) && warning.message.contains(name))
}

/// A warning the protected expression reported before its read failed, and
/// one elsewhere in the document, both stay in the report, and a policy that
/// blocks the code still stops the document.
#[test]
fn unrelated_warning_survives_recovery() {
    let dir = TempDir::new().unwrap();
    let content = "{{ try([inside, load_data('missing.yaml', 'string[]')], 'FALLBACK') }} {{ outside }}\n";
    let (text, report) = rendered(dir.path(), content, ComposeOptions::new());
    assert_eq!(text, "FALLBACK");
    assert!(undeclared(&report.warnings, "inside"), "{:?}", report.warnings);
    assert!(undeclared(&report.warnings, "outside"), "{:?}", report.warnings);

    let deny = ComposeOptions::new().with_blocking_policy(BlockingPolicy::default().deny_code(UNDECLARED));
    let failure = failure_of(dir.path(), content, deny);
    assert!(matches!(failure.error, darkmatter::markdown::MarkdownError::BlockedDiagnostics { .. }), "{:?}", failure.error);
}

#[test]
fn undeclared_property_warning_inside_try_remains() {
    let dir = TempDir::new().unwrap();
    let front = "---\n$schema:\n  title: string\ntitle: Hello\n---\n";
    let (text, report) = rendered(dir.path(), &format!("{front}{{{{ try(title, 'x') }}}}\n"), ComposeOptions::new());
    assert_eq!(text, "Hello");
    assert!(report.warnings.is_empty(), "control: {:?}", report.warnings);

    let (text, report) = rendered(dir.path(), &format!("{front}{{{{ try(titel, 'x') }}}}\n"), ComposeOptions::new());
    assert_eq!(text, "", "an absent property is null, a success");
    assert!(undeclared(&report.warnings, "titel"), "{:?}", report.warnings);
}

/// `try` absorbs one read, not the file: the same missing file read without
/// `try` later in the document still stops it, at that read.
#[test]
fn independent_reference_to_missing_file_still_fails() {
    let dir = TempDir::new().unwrap();
    let guarded = "{{ try(load_data('missing.yaml', 'string[]'), 'FALLBACK') }}\n";
    let (text, _) = rendered(dir.path(), guarded, ComposeOptions::new());
    assert_eq!(text, "FALLBACK");

    let failure = failure_of(dir.path(), &format!("{guarded}\n{{{{ load_data('missing.yaml', 'string[]') }}}}\n"), ComposeOptions::new());
    let diagnostic = cause(&failure).file_reference_diagnostic().unwrap_or_else(|| panic!("{:?}", failure.error));
    assert_eq!((diagnostic.kind, diagnostic.reference.as_str()), (FileRefFailure::NotFound, "missing.yaml"));
    let source = record(&failure).source.expect("located");
    assert_eq!((source.line, source.column), (Some(3), Some(1)));
}
