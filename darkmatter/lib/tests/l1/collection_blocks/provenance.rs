//! Runtime failures inside repeated bodies keep their provenance, for both
//! kinds: the authored file, line, and span of the failing site, and the
//! original items of the copy it ran in.
//!
//! Every row composes a real document in its own temporary directory and
//! checks the library twice: the typed error (its located collection failure
//! or its `InIteration` context) and the shared record
//! [`error_diagnostics`] projects for every host.

use std::num::NonZeroU64;
use std::path::Path;
use std::sync::Arc;

use darkmatter::markdown::compose::ComposeOperation;
use darkmatter::markdown::compose::collection_blocks::CollectionKind;
use darkmatter::markdown::compose::expression::Diagnostic;
use darkmatter::markdown::compose::expression::iteration::IterationPosition;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use darkmatter::markdown::{MarkdownError, SourceRef};
use tempfile::TempDir;

use super::support::{closer, opener};
use crate::collection_execution::compose_with_options;

/// One line of 120 `a`s: a value or output larger than every byte limit below.
const LONG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// The request allowance a row runs under.
#[derive(Clone, Copy)]
enum Limit {
    None,
    Visits(u64),
    Bytes(u64),
    /// The body's own authored bytes plus this many: the copy fits and what a
    /// later stage inserts does not.
    BodyBytesPlus(u64),
}

struct Case {
    row: &'static str,
    /// The declared type of `data`.
    schema: &'static str,
    /// `data`'s YAML value; `{LONG}` is replaced by [`LONG`].
    items: &'static str,
    /// The body; `{OPEN}` and `{CLOSE}` are the kind's opener and closer
    /// keywords, and `{LONG}` is [`LONG`]. Its first line is file line 6.
    body: &'static str,
    limit: Limit,
    code: &'static str,
    /// The file line the failure is located at.
    line: usize,
    /// The original item path; a cached replay may be either copy.
    paths: &'static [&'static str],
}

const EVALUATION: &str = "dm.expression.evaluation_failure";
const RUNTIME: &str = r#"["./ok.md", "./missing.md"]"#;

const CASES: &[Case] = &[
    Case {
        row: "item_visit_reservation",
        schema: "string[]",
        items: "[a, b]",
        body: "{OPEN} data\nx\n{CLOSE}\n",
        limit: Limit::Visits(1),
        code: "dm.iteration.visit_limit",
        line: 6,
        paths: &["data[1]"],
    },
    Case {
        row: "authored_body_copy",
        schema: "string[]",
        items: "[a]",
        body: "{OPEN} data\nxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\n{CLOSE}\n",
        limit: Limit::Bytes(1),
        code: "dm.iteration.expansion_limit",
        line: 7,
        paths: &["data[0]"],
    },
    Case {
        row: "interpolation_insertion",
        schema: "string[]",
        items: "[{LONG}]",
        body: "{OPEN} data\n{{i}}\n{CLOSE}\n",
        limit: Limit::Bytes(10),
        code: "dm.iteration.expansion_limit",
        line: 7,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_output_insertion",
        schema: "string[]",
        items: "[a]",
        body: "{OPEN} data\n::shell echo {LONG}\n{CLOSE}\n",
        limit: Limit::BodyBytesPlus(5),
        code: "dm.iteration.expansion_limit",
        line: 7,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_block_output_insertion",
        schema: "string[]",
        items: "[a]",
        body: "{OPEN} data\n::shell-block\necho {LONG}\n::end-block\n{CLOSE}\n",
        limit: Limit::BodyBytesPlus(5),
        code: "dm.iteration.expansion_limit",
        line: 7,
        paths: &["data[0]"],
    },
    Case {
        row: "child_output_insertion",
        schema: "string[]",
        items: "[a]",
        body: "{OPEN} data\n::file ./long.md\n{CLOSE}\n",
        limit: Limit::BodyBytesPlus(5),
        code: "dm.iteration.expansion_limit",
        line: 7,
        paths: &["data[0]"],
    },
    Case {
        row: "cached_child_replay",
        schema: "string[]",
        items: "[a, b]",
        // 2 parent visits and the child's 1 fit; the replay's 1 does not.
        body: "{OPEN} data\n::file ./visits.md\n{CLOSE}\n",
        limit: Limit::Visits(3),
        code: "dm.iteration.visit_limit",
        line: 7,
        paths: &["data[0]", "data[1]"],
    },
    Case {
        row: "repeated_body_interpolation",
        schema: "string[]",
        items: RUNTIME,
        body: "{OPEN} data\n{{ frontmatter(i, 'title') }}\n{CLOSE}\n",
        limit: Limit::None,
        code: EVALUATION,
        line: 7,
        paths: &["data[1]"],
    },
    Case {
        row: "repeated_page_block_gate",
        schema: "string[]",
        items: RUNTIME,
        body: "{OPEN} data\n::block when=\"frontmatter(i, 'title')\"\nx\n::end-block\n{CLOSE}\n",
        limit: Limit::None,
        code: EVALUATION,
        line: 7,
        paths: &["data[1]"],
    },
    Case {
        row: "nested_collection_gate",
        schema: "string[]",
        items: RUNTIME,
        body: "{OPEN} data\n{OPEN} data when=\"frontmatter(i, 'title')\"\ny\n{CLOSE}\n{CLOSE}\n",
        limit: Limit::None,
        code: EVALUATION,
        line: 7,
        paths: &["data[1]"],
    },
    Case {
        row: "repeated_transclusion_target",
        schema: "string[]",
        items: RUNTIME,
        body: "{OPEN} data\n::file {{ frontmatter(i, 'title') }}\n{CLOSE}\n",
        limit: Limit::None,
        code: EVALUATION,
        line: 7,
        paths: &["data[1]"],
    },
    Case {
        row: "repeated_transclusion_condition",
        schema: "string[]",
        items: RUNTIME,
        body: "{OPEN} data\n::file ./ok.md when=\"frontmatter(i, 'title')\"\n{CLOSE}\n",
        limit: Limit::None,
        code: EVALUATION,
        line: 7,
        paths: &["data[1]"],
    },
    Case {
        row: "shell_argument_validation",
        schema: "any[]",
        items: "[null]",
        body: "{OPEN} data\n::shell echo {{ i }}\n{CLOSE}\n",
        limit: Limit::None,
        code: "dm.shell.failure",
        line: 7,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_later_argument_validation",
        schema: "any[]",
        items: "[null]",
        body: "{OPEN} data\n::shell echo first\n::shell echo {{ i }}\n{CLOSE}\n",
        limit: Limit::None,
        code: "dm.shell.failure",
        line: 8,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_block_argument_validation",
        schema: "any[]",
        items: "[null]",
        // Located at the failing command, not the block's opener.
        body: "{OPEN} data\n::shell-block\necho {{ i }}\n::end-block\n{CLOSE}\n",
        limit: Limit::None,
        code: "dm.shell.failure",
        line: 8,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_block_later_command_validation",
        schema: "any[]",
        items: "[null]",
        // Every command is prepared before any runs; the second is located.
        body: "{OPEN} data\n::shell-block\necho first\necho {{ i }}\n::end-block\n{CLOSE}\n",
        limit: Limit::None,
        code: "dm.shell.failure",
        line: 9,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_block_crlf_command_validation",
        schema: "any[]",
        items: "[null]",
        body: "{OPEN} data\r\n::shell-block\r\necho café\r\necho {{ i }}\r\n::end-block\r\n{CLOSE}\r\n",
        limit: Limit::None,
        code: "dm.shell.failure",
        line: 9,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_block_after_multiline_argument_validation",
        schema: "any[]",
        items: r#"[{good: "a\nb", bad: null}]"#,
        body: "{OPEN} data\n::shell-block\necho {{ i.good }}\necho {{ i.bad }}\n::end-block\n{CLOSE}\n",
        limit: Limit::None,
        code: "dm.shell.failure",
        line: 9,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_block_quoted_command_validation",
        schema: "any[]",
        items: "[null]",
        body: "> {OPEN} data\n> ::shell-block\n> echo {{ i }}\n> ::end-block\n> {CLOSE}\n",
        limit: Limit::None,
        code: "dm.shell.failure",
        line: 8,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_block_earlier_command_execution",
        schema: "string[]",
        items: "[a]",
        // The first command fails to start after both were prepared, so the
        // stage must enter it again to run it.
        body: "{OPEN} data\n::shell-block\ndarkmatter-provenance-no-such-program\necho {{ i }}\n::end-block\n{CLOSE}\n",
        limit: Limit::None,
        code: "dm.shell.failure",
        line: 8,
        paths: &["data[0]"],
    },
    Case {
        row: "shell_block_later_command_execution",
        schema: "string[]",
        items: "[a]",
        body: "{OPEN} data\n::shell-block\necho {{ i }}\ndarkmatter-provenance-no-such-program\n::end-block\n{CLOSE}\n",
        limit: Limit::None,
        code: "dm.shell.failure",
        line: 9,
        paths: &["data[0]"],
    },
    Case {
        row: "callback_failure",
        schema: "string[]",
        items: RUNTIME,
        body: "{OPEN} data \"i => frontmatter(i, 'title')\"\nx\n{CLOSE}\n",
        limit: Limit::None,
        code: "dm.collection.callback_failed",
        line: 6,
        paths: &["data[1]"],
    },
];

/// Approves every command once, as an interactive user would.
struct Approve;

impl ShellApprovalHandler for Approve {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn write(root: &Path, name: &str, content: &str) {
    std::fs::write(root.join(name), content).unwrap();
}

/// The error composing the document of `case` for `kind` raised.
fn fail(case: &Case, kind: CollectionKind) -> MarkdownError {
    fail_with_document(case, kind).1
}

/// The document of `case` for `kind`, and the error composing it raised.
fn fail_with_document(case: &Case, kind: CollectionKind) -> (String, MarkdownError) {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    write(&root, "ok.md", "---\ntitle: ./ok.md\n---\nok\n");
    write(&root, "long.md", &format!("{LONG}\n"));
    write(&root, "visits.md", "---\n$schema:\n  d: string[](required)\nd: [z]\n---\n::map d\n{{i}}\n::end-map\n");
    let fill = |text: &str| {
        text.replace("{OPEN}", &opener(kind)).replace("{CLOSE}", &closer(kind)).replace("{LONG}", LONG)
    };
    let body = fill(case.body);
    let document = format!("---\n$schema:\n  data: {}(required)\ndata: {}\n---\n{body}", case.schema, fill(case.items));
    let body_bytes = body.lines().skip(1).collect::<Vec<_>>();
    let body_bytes = body_bytes[..body_bytes.len() - 1].iter().map(|line| line.len() as u64 + 1).sum::<u64>();
    let limit = |value: u64| NonZeroU64::new(value).unwrap();
    let outcome = compose_with_options(&root, &document, |options| {
        let options = options
            .disable(ComposeOperation::Cleanup)
            .with_shell_policy_root(&root)
            .with_shell_working_directory(&root)
            .with_shell_approval_handler(Arc::new(Approve));
        match case.limit {
            Limit::None => options,
            Limit::Visits(visits) => options.with_max_iteration_visits(limit(visits)),
            Limit::Bytes(bytes) => options.with_max_expansion_bytes(limit(bytes)),
            Limit::BodyBytesPlus(extra) => options.with_max_expansion_bytes(limit(body_bytes + extra)),
        }
    });
    match outcome {
        Ok(output) => panic!("{} ({kind:?}) composed:\n{output}", case.row),
        Err(error) => (document, error),
    }
}

/// The authored line and item path the typed error itself carries.
fn typed_provenance(error: &MarkdownError) -> (Option<usize>, String) {
    let line = |source: Option<&SourceRef>| match source {
        Some(SourceRef::OnDiskSpan { span, .. }) => Some(span.line()),
        _ => None,
    };
    match error {
        MarkdownError::InIteration(failure) => (line(failure.location.as_deref()), failure.iteration_path.to_string()),
        MarkdownError::Collection(inner) => {
            (line(inner.site().map(|site| site.source.as_ref())), inner.iteration_path().to_string())
        }
        other => panic!("no provenance on {other:?}"),
    }
}

fn the_record(error: &MarkdownError) -> Diagnostic {
    let records = error_diagnostics(error);
    assert_eq!(records.len(), 1, "one record per failure: {records:#?}");
    records.into_iter().next().unwrap()
}

fn check(name: &str, kind: CollectionKind) {
    let case = CASES.iter().find(|case| case.row == name).expect("a row of CASES");
    let (document, error) = fail_with_document(case, kind);
    let what = format!("{name} ({kind:?}): {error:?}");

    let (typed_line, typed_path) = typed_provenance(&error);
    assert_eq!(typed_line, Some(case.line), "typed line, {what}");
    assert!(case.paths.contains(&typed_path.as_str()), "typed path {typed_path}, {what}");

    let record = the_record(&error);
    assert_eq!(record.code, case.code, "{what}");
    let source = record.source.as_ref().unwrap_or_else(|| panic!("record source, {what}"));
    assert_eq!(source.file.as_deref().and_then(Path::file_name), Some("doc.md".as_ref()), "{what}");
    assert_eq!(source.line, case.line, "record line, {what}");
    let span = record.span.clone().unwrap_or_else(|| panic!("record span, {what}"));
    let span_line = document[..span.start].matches('\n').count() + 1;
    assert_eq!(span_line, case.line, "record span {span:?}, {what}");
    if case.code == "dm.shell.failure" {
        let line_start = document.split_inclusive('\n').take(case.line - 1).map(str::len).sum::<usize>();
        let physical = document[line_start..].split('\n').next().unwrap().trim_end_matches('\r');
        let line = physical.strip_prefix("> ").unwrap_or(physical);
        let prefix = physical.len() - line.len();
        let start = line_start + prefix;
        assert_eq!(source.column, prefix + 1, "command column, {what}");
        assert_eq!(span.start, start, "command byte start, {what}");
        assert!(span.end > start && span.end <= start + line.len(), "command extent {span:?}, {what}");
        assert!(line.starts_with(&document[span]), "authored command slice, {what}");
    }
    assert_eq!(record.iteration_path.to_string(), typed_path, "record path, {what}");
    assert!(record.message.ends_with(&format!(" at {typed_path}")), "{}: {what}", record.message);
}

macro_rules! provenance_rows {
    ($($name:ident),+ $(,)?) => {
        const ROWS: &[&str] = &[$(stringify!($name)),+];
        mod map {
            use darkmatter::markdown::compose::collection_blocks::CollectionKind;
            $( #[test] fn $name() { super::check(stringify!($name), CollectionKind::Map); } )+
        }
        mod filter {
            use darkmatter::markdown::compose::collection_blocks::CollectionKind;
            $( #[test] fn $name() { super::check(stringify!($name), CollectionKind::Filter); } )+
        }
    };
}

provenance_rows!(
    item_visit_reservation,
    authored_body_copy,
    interpolation_insertion,
    shell_output_insertion,
    shell_block_output_insertion,
    child_output_insertion,
    cached_child_replay,
    repeated_body_interpolation,
    repeated_page_block_gate,
    nested_collection_gate,
    repeated_transclusion_target,
    repeated_transclusion_condition,
    shell_argument_validation,
    shell_later_argument_validation,
    shell_block_argument_validation,
    shell_block_later_command_validation,
    shell_block_crlf_command_validation,
    shell_block_after_multiline_argument_validation,
    shell_block_quoted_command_validation,
    shell_block_earlier_command_execution,
    shell_block_later_command_execution,
    callback_failure,
);

#[test]
fn every_case_has_a_generated_test_and_every_test_a_case() {
    let cases: Vec<_> = CASES.iter().map(|case| case.row).collect();
    assert_eq!(cases, ROWS);
}

#[test]
fn the_shared_record_carries_typed_frames_and_the_authored_span() {
    let case = CASES.iter().find(|case| case.row == "callback_failure").unwrap();
    for kind in [CollectionKind::Map, CollectionKind::Filter] {
        let error = fail(case, kind);
        let record = the_record(&error);
        let [frame] = record.iteration_path.frames() else { panic!("{record:?}") };
        assert_eq!((frame.collection.as_str(), &frame.position), ("data", &IterationPosition::Index(1)));
        let span = record.span.clone().expect("the callback's authored span");
        assert!(span.start > 0 && span.start < span.end, "{span:?}");
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["iteration_path"], serde_json::json!([{ "collection": "data", "position": 1 }]), "{kind:?}");
        assert_eq!(json["span"]["start"], span.start, "{kind:?}");
    }
}

#[test]
fn a_child_failure_under_a_repeated_file_joins_the_parent_and_child_paths() {
    for kind in [CollectionKind::Map, CollectionKind::Filter] {
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        write(&root, "plain.md", "plain\n");
        write(
            &root,
            "bad.md",
            "---\n$schema:\n  d: string[](required)\nd: [z]\n---\n::map d \"x => frontmatter('./missing.md', 'title')\"\n{{x}}\n::end-map\n",
        );
        let document = format!(
            "---\n$schema:\n  data: string[](required)\ndata: [./plain.md, ./bad.md]\n---\n{} data\n::file {{{{ i }}}}\n{}\n",
            opener(kind),
            closer(kind)
        );
        let error = compose_with_options(&root, &document, |options| options).expect_err("the child fails");
        let record = the_record(&error);
        assert_eq!(record.code, "dm.collection.callback_failed", "{kind:?}: {error:?}");
        assert_eq!(record.iteration_path.to_string(), "data[1] → d[0]", "{kind:?}: {record:?}");
        let source = record.source.as_ref().expect("the child's own site");
        assert_eq!(source.file.as_deref().and_then(Path::file_name), Some("bad.md".as_ref()), "{kind:?}");
        assert_eq!(source.line, 6, "{kind:?}");
    }
}

#[test]
fn a_static_failure_is_reported_once_with_no_iteration() {
    for kind in [CollectionKind::Map, CollectionKind::Filter] {
        let document = format!("---\nnames: [a, b]\n---\n{} names\n- {{{{i}}}}\n{}\n", opener(kind), closer(kind));
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let error = compose_with_options(&root, &document, |options| options).expect_err("an untyped source");
        let records = error_diagnostics(&error);
        assert_eq!(records.len(), 1, "{kind:?}: {records:#?}");
        assert!(records[0].iteration_path.is_empty(), "{kind:?}: {records:?}");
        assert!(!records[0].message.contains(" at names["), "{kind:?}: {records:?}");
        assert!(!matches!(error, MarkdownError::InIteration(_)), "{kind:?}");
    }
}
