//! A composition's warnings and failure as shared diagnostics
//! (`compose::report_diagnostics`), the records `md compose --output
//! report-json` writes: each failure is classified by its typed error and
//! located where the error knows its authored position, a failed composition
//! keeps the warnings it had reported, and producing the records reads no
//! file, runs no handler, and holds no value.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::ComposeWarning as Warning;
use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
use darkmatter::markdown::compose::expression::{Diagnostic, DiagnosticSource, Severity, UNCERTAIN_CALL_CODE};
use darkmatter::markdown::compose::policy::BlockingPolicy;
use darkmatter::markdown::compose::report_diagnostics::{
    COMPOSE_FAILURE_CODE, FRONTMATTER_PARSE_CODE, INPUT_READ_CODE, UNCODED_WARNING_CODE, error_diagnostics,
    warning_diagnostic,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeFailure, ComposeOptions, ComposeReport};
use darkmatter::markdown::schemas::SCHEMA_TYPE_MISMATCH_CODE;
use darkmatter::markdown::Markdown;
use serde_json::json;
use tempfile::TempDir;

const CATEGORY_3: &str = "---\n$schema:\n  label: string\nlabel: \"7\"\n---\nBody {{ min(label, 5) }}\n";

fn options_in(dir: &Path, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.to_path_buf())
}

fn compose(dir: &Path, name: &str, content: &str, options: impl FnOnce(ComposeOptions) -> ComposeOptions) -> (PathBuf, Result<(Markdown, ComposeReport), Box<ComposeFailure>>) {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    let request = crate::request_support::request_at(dir, options(options_in(dir, &path)));
    let result = Markdown::try_from(path.as_path()).unwrap().compose_with_warnings(&request);
    (path, result)
}

fn failure(dir: &Path, name: &str, content: &str) -> (PathBuf, Box<ComposeFailure>) {
    let (path, result) = compose(dir, name, content, |options| options);
    (path, result.err().unwrap_or_else(|| panic!("{name} fails")))
}

fn located(file: &Path, line: usize, column: usize) -> Option<DiagnosticSource> {
    Some(DiagnosticSource { file: Some(file.to_path_buf()), line, column })
}

#[test]
fn each_failure_is_classified_by_its_typed_error() {
    let dir = TempDir::new().unwrap();
    for (name, content, code, position) in [
        ("yaml.md", "---\ntitle: [unclosed\n---\nbody\n", FRONTMATTER_PARSE_CODE, Some((3, 1))),
        ("schema.md", "---\ncount: hello\n$schema:\n  count: number(required)\n---\nbody\n", SCHEMA_TYPE_MISMATCH_CODE, Some((2, 8))),
        ("eval.md", "# Doc\n\n{{ 1 / 0 }}\n", "dm.expression.evaluation_failure", Some((3, 1))),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        // A frontmatter parse failure is found loading the document.
        let error = match Markdown::try_from(path.as_path()) {
            Err(error) => error,
            Ok(_) => failure(dir.path(), name, content).1.error,
        };
        let diagnostics = error_diagnostics(&error);
        assert_eq!(diagnostics.len(), 1, "{name}: {diagnostics:#?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.code, code, "{name}: {diagnostic:#?}");
        assert_eq!(diagnostic.severity, Severity::Error, "{name}");
        assert_eq!(diagnostic.source, position.and_then(|(line, column)| located(&path, line, column)), "{name}");
        assert_eq!((diagnostic.expected_type.as_deref(), diagnostic.actual_type.as_deref()), (None, None), "{name}");
    }
}

/// A rejected call reports exactly the record composition and DMLS share.
#[test]
fn a_rejected_call_reports_its_shared_record() {
    let dir = TempDir::new().unwrap();
    let (_, failure) = failure(dir.path(), "doc.md", &CATEGORY_3.replace("min(label, 5)", "min(true, 5)"));
    let record = failure.error.call_diagnostic().expect("a rejected call");
    assert_eq!(error_diagnostics(&failure.error), vec![record]);
}

/// A foreign error has no Darkmatter type: an I/O error is an input failure,
/// anything else the catch-all, both with the error's message.
#[test]
fn a_foreign_error_takes_a_family_code_and_no_source() {
    let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
    let other = std::fmt::Error;
    for (error, code) in [(&io as &(dyn std::error::Error + 'static), INPUT_READ_CODE), (&other, COMPOSE_FAILURE_CODE)] {
        let diagnostics = error_diagnostics(error);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, code);
        assert_eq!(diagnostics[0].message, error.to_string());
        assert_eq!(diagnostics[0].source, None);
    }
}

/// A warning keeps its shared record; one without a record keeps its code,
/// or takes the uncoded code, and has no source.
#[test]
fn a_warning_keeps_its_record_or_its_code() {
    let dir = TempDir::new().unwrap();
    let (_, result) = compose(dir.path(), "doc.md", CATEGORY_3, |options| options);
    let (_, report) = result.unwrap_or_else(|failure| panic!("composes: {}", failure.error));
    let warning = report.warnings.iter().find(|warning| warning.diagnostic.is_some()).expect("the call warning");
    assert_eq!(&warning_diagnostic(warning), warning.diagnostic.as_deref().unwrap());

    let mut coded = Warning::new("stage", "coded");
    coded.code = Some("dm.context.ctx_override".to_string());
    for (warning, code) in [(coded, "dm.context.ctx_override"), (Warning::new("stage", "plain"), UNCODED_WARNING_CODE)] {
        let diagnostic = warning_diagnostic(&warning);
        assert_eq!(
            diagnostic,
            Diagnostic {
                code: code.to_string(),
                severity: Severity::Warning,
                message: warning.message.clone(),
                source: None,
                expected_type: None,
                actual_type: None,
            }
        );
    }
}

/// The warnings reported before a later failure come back with it, the outer
/// document's first; [`Markdown::compose_with`] is unchanged.
#[test]
fn a_failed_composition_keeps_the_warnings_it_reported() {
    let dir = TempDir::new().unwrap();
    let content = format!("{CATEGORY_3}\n{{{{ 1 / 0 }}}}\n");
    let (path, failure) = failure(dir.path(), "doc.md", &content);
    let codes: Vec<Option<&str>> = failure.warnings.iter().map(|warning| warning.code.as_deref()).collect();
    assert_eq!(codes, [Some(UNCERTAIN_CALL_CODE)], "{:#?}", failure.warnings);
    assert_eq!(error_diagnostics(&failure.error)[0].source, located(&path, 8, 1));

    let request = crate::request_support::request_at(dir.path(), options_in(dir.path(), &path));
    let plain = Markdown::try_from(path.as_path()).unwrap().compose_with(&request).expect_err("same failure");
    assert_eq!(plain.to_string(), failure.error.to_string());
}

/// A policy stop reports the warnings before the stop, then the blocking one,
/// each with its own severity.
#[test]
fn a_policy_stop_reports_every_warning_with_its_own_severity() {
    let dir = TempDir::new().unwrap();
    let content = "---\n$schema:\n  label: string\nlabel: \"7\"\n---\n{{ file_exists(\"\") }} {{ min(label, 5) }}\n";
    let (_, result) = compose(dir.path(), "doc.md", content, |options| {
        options.with_blocking_policy(BlockingPolicy::default().deny_code(UNCERTAIN_CALL_CODE))
    });
    let failure = result.err().expect("the uncertain call blocks");
    let diagnostics = error_diagnostics(&failure.error);
    let codes: Vec<(&str, Severity)> = diagnostics.iter().map(|diagnostic| (diagnostic.code.as_str(), diagnostic.severity)).collect();
    assert_eq!(
        codes,
        [("dm.expression.always_false_call", Severity::Warning), (UNCERTAIN_CALL_CODE, Severity::Warning)],
        "{diagnostics:#?}"
    );
}

/// Criterion 22's report leg: the records of a failed composition and its
/// earlier warnings hold no frontmatter, environment, or file-content value,
/// entering no handler and attempting no network access; producing them again
/// after every file is gone gives the same records, so it reads no file.
#[test]
fn report_diagnostics_hold_no_value_and_read_nothing() {
    const SECRET: &str = "s3cr3t-value";
    let registration = FunctionRegistration::builder("probe", "function(parameters(v: number); returns(number))")
        .build()
        .expect("the test signature compiles");
    let _probe = hooks::register(registration, Arc::new(|_| Ok(json!(1))));

    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("other.md"), format!("---\ntitle: {SECRET}\n---\n{SECRET}\n")).unwrap();
    let content = format!(
        "---\n$schema:\n  label: string\nlabel: \"{SECRET}\"\n---\n\
         {{{{ false ? probe(label) : 0 }}}} {{{{ false ? probe(env.SECRET_VALUE) : 0 }}}} \
         {{{{ is_file_type(\"other.md\") ? 1 : 0 }}}}\n\n{{{{ 1 / 0 }}}}\n"
    );
    hooks::reset_handler_entries();
    let network = darkmatter::effects::network_attempt_count();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, &content).unwrap();
    let mut context = ComposeContext::capture_for_content(dir.path(), "");
    context.env_mut().insert("SECRET_VALUE".to_string(), SECRET.to_string());
    let request = crate::request_support::request_at(dir.path(), ComposeOptions::new_with_context(context).with_source_file(path.clone()));
    let failure = Markdown::try_from(path.as_path()).unwrap().compose_with_warnings(&request).err().expect("the division fails");
    let records = |failure: &ComposeFailure| -> Vec<Diagnostic> {
        failure.warnings.iter().map(warning_diagnostic).chain(error_diagnostics(&failure.error)).collect()
    };
    let before = records(&failure);
    assert!(before.len() >= 2, "the probe warning and the failure: {before:#?}");
    let serialized = serde_json::to_string(&before).unwrap();
    assert!(!serialized.contains(SECRET), "{serialized}");
    assert_eq!(hooks::handler_entries("probe"), 0, "no handler ran");
    assert_eq!(darkmatter::effects::network_attempt_count(), network);

    dir.close().unwrap();
    assert_eq!(records(&failure), before, "nothing was read to produce them");
}
