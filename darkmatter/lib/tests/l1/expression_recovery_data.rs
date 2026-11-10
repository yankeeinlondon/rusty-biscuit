//! `try` over `load_data`: recoverable and nonrecoverable loader failures,
//! the data memo, the read set, remote transport failures, and approval
//! discovery. Recovery never stores the fallback where a later call finds it.
//!
//! Every fixture lives in its own temporary directory and every request is
//! anchored there. Most fallbacks read `fallback.json`, so a test sees from
//! the read set whether the fallback ran.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::DataError;
use darkmatter::markdown::compose::expression::{ExpressionError, FileRefFailure, RemoteReadKind};
use darkmatter::markdown::compose::remote::RemoteReadConfig;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{
    CacheAccessMode, ComposeFailure, ComposeOptions, ComposeReport, ComposeRequest, DataRuntime, RequestSnapshot,
    ShellExpansionError,
};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Fixtures ────────────────────────────────────────────────────────────────

const OS_YAML: &str = "- macOS\n- Linux\n- Windows\n";
const FALLBACK: &str = "load_data('fallback.json', 'string[]')";

fn write(dir: &Path, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// The read-set spelling of `name` in `dir`.
fn entry(dir: &Path, name: &str) -> String {
    let canonical = biscuit_file::canonicalize_simplified(dir).expect("canonical dir");
    biscuit_file::to_portable_string(&canonical.join(name))
}

/// A fresh fixture directory holding `fallback.json`.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "fallback.json", "[\"fallback\"]");
    dir
}

type Composed = Result<(Markdown, ComposeReport), Box<ComposeFailure>>;

fn compose_file(path: &Path, options: ComposeOptions) -> Composed {
    let request = crate::request_support::request(options.with_source_file(path));
    Markdown::try_from(path).expect("document loads").compose_with_warnings(&request)
}

/// Composes `content` as `doc.md` in `dir`, sharing `runtime`.
fn compose_in(dir: &Path, content: &str, runtime: &DataRuntime) -> Composed {
    let doc = write(dir, "doc.md", content);
    compose_file(&doc, ComposeOptions::new().with_data_runtime(runtime.clone()))
}

/// A document rendering `expression`'s value as JSON (wrapped in a list, all
/// `as_json` renders), under `frontmatter` when it is not empty.
fn document(frontmatter: &str, expression: &str) -> String {
    let body = format!("{{{{ as_json([{expression}]) }}}}\n");
    if frontmatter.is_empty() { body } else { format!("---\n{frontmatter}\n---\n{body}") }
}

/// The value of `expression` and the report of its composition.
fn value_in(dir: &Path, runtime: &DataRuntime, frontmatter: &str, expression: &str) -> (Value, ComposeReport) {
    match compose_in(dir, &document(frontmatter, expression), runtime) {
        Ok((composed, report)) => {
            let text = composed.content().trim().to_string();
            let wrapped: Value =
                serde_json::from_str(&text).unwrap_or_else(|error| panic!("{text:?} is not JSON: {error}"));
            (wrapped[0].clone(), report)
        }
        Err(failure) => panic!("{expression} failed: {}", failure.error),
    }
}

fn value_of(dir: &Path, expression: &str) -> (Value, ComposeReport) {
    value_in(dir, &DataRuntime::new(), "", expression)
}

/// The failure of `expression`.
fn failure_in(dir: &Path, runtime: &DataRuntime, frontmatter: &str, expression: &str) -> Box<ComposeFailure> {
    match compose_in(dir, &document(frontmatter, expression), runtime) {
        Ok((composed, _)) => panic!("{expression} composed: {}", composed.content()),
        Err(failure) => failure,
    }
}

fn failure_of(dir: &Path, expression: &str) -> Box<ComposeFailure> {
    failure_in(dir, &DataRuntime::new(), "", expression)
}

fn codes(failure: &ComposeFailure) -> Vec<String> {
    error_diagnostics(&failure.error).into_iter().map(|record| record.code).collect()
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

fn data_error(failure: &ComposeFailure) -> Arc<DataError> {
    expression_errors(failure)
        .into_iter()
        .find_map(|error| match error {
            ExpressionError::Data(data) => Some(data.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no data error in: {}", failure.error))
}

/// The kind of the file-reference failure in `failure`'s cause chain, raised
/// directly or carried by a `load_data` source failure.
fn file_reference_kind(failure: &ComposeFailure) -> Option<FileRefFailure> {
    expression_errors(failure).into_iter().find_map(|error| match error {
        ExpressionError::FileReference(diagnostic) => Some(diagnostic.kind),
        ExpressionError::Data(data) => match &**data {
            DataError::FileReference(diagnostic) => Some(diagnostic.kind),
            _ => None,
        },
        _ => None,
    })
}

/// `try(<call>, <fallback>)`.
fn guarded(call: &str, fallback: &str) -> String {
    format!("try({call}, {fallback})")
}

/// Asserts the guarded composition recovered cleanly: the fallback's value,
/// its read recorded, and nothing reported for the absorbed cause.
fn assert_recovered(dir: &Path, value: &Value, report: &ComposeReport) {
    assert_eq!(value, &json!(["fallback"]), "the fallback's value");
    assert!(report.read_set.contains(&entry(dir, "fallback.json")), "the fallback ran: {:?}", report.read_set);
    assert!(report.warnings.is_empty(), "the absorbed cause was reported: {:?}", report.warnings);
}

// ── Positive controls and recoveries ────────────────────────────────────────

#[test]
fn valid_source_is_returned() {
    let dir = fixture();
    write(dir.path(), "os.yaml", OS_YAML);
    let (value, report) = value_of(dir.path(), &guarded("load_data('os.yaml', 'string[]')", FALLBACK));
    assert_eq!(value, json!(["macOS", "Linux", "Windows"]));
    assert_eq!(report.read_set, vec![entry(dir.path(), "os.yaml")], "the fallback never ran");
}

#[test]
fn missing_source_recovers() {
    let dir = fixture();
    let call = "load_data('missing.yaml', 'string[]')";
    let control = failure_of(dir.path(), call);
    assert_eq!(file_reference_kind(&control), Some(FileRefFailure::NotFound), "{}", control.error);

    let (value, report) = value_of(dir.path(), &guarded(call, FALLBACK));
    assert_recovered(dir.path(), &value, &report);
}

/// The read guard is a thread-local scope, and a single document's body
/// interpolation runs on the calling thread, so the refused read is the real
/// one at the load site.
#[test]
fn unreadable_source_recovers() {
    let dir = fixture();
    write(dir.path(), "data.yaml", OS_YAML);
    let refuse = || -> simplified_schema::resolve::buffers::ReadGuard {
        Rc::new(|path: &Path| path.file_name() != Some(OsStr::new("data.yaml")))
    };
    let call = "load_data('data.yaml', 'string[]')";
    let control = simplified_schema::resolve::buffers::guarded(refuse(), || failure_of(dir.path(), call));
    assert_eq!(file_reference_kind(&control), Some(FileRefFailure::NotFound), "{}", control.error);
    let diagnostic = expression_errors(&control)
        .into_iter()
        .find_map(ExpressionError::file_reference_diagnostic)
        .unwrap_or_else(|| panic!("no file reference in {}", control.error));
    assert!(diagnostic.source.is_some(), "an I/O failure, not a clean miss: {}", control.error);

    let (value, report) =
        simplified_schema::resolve::buffers::guarded(refuse(), || value_of(dir.path(), &guarded(call, FALLBACK)));
    assert_recovered(dir.path(), &value, &report);
    assert!(report.read_set.contains(&entry(dir.path(), "data.yaml")), "{:?}", report.read_set);
}

#[test]
fn malformed_source_recovers() {
    let dir = fixture();
    write(dir.path(), "bad.yaml", "[macOS, Linux\n");
    write(dir.path(), "bad.json", b"[\"a\", \xff]".as_slice());
    for file in ["bad.yaml", "bad.json"] {
        let call = format!("load_data('{file}', 'string[]')");
        assert_eq!(codes(&failure_of(dir.path(), &call)), ["dm.data.malformed"], "{file}");
        let (value, report) = value_of(dir.path(), &guarded(&call, FALLBACK));
        assert_recovered(dir.path(), &value, &report);
    }
}

#[test]
fn contract_mismatch_recovers() {
    let dir = fixture();
    write(dir.path(), "os.yaml", OS_YAML);
    let call = "load_data('os.yaml', 'number[]')";
    let control = codes(&failure_of(dir.path(), call));
    assert!(!control.is_empty() && control.iter().all(|code| code == "dm.data.contract_mismatch"), "{control:?}");
    let (value, report) = value_of(dir.path(), &guarded(call, FALLBACK));
    assert_recovered(dir.path(), &value, &report);
}

#[test]
fn headerless_malformed_record_recovers() {
    let dir = fixture();
    write(dir.path(), "short.csv", "a,b\n1,2\n3\n");
    let call = "load_data('short.csv', 'tuple([string, string])[]', {header: false})";
    assert_eq!(codes(&failure_of(dir.path(), call)), ["dm.data.malformed"]);
    let (value, report) = value_of(dir.path(), &guarded(call, FALLBACK));
    assert_recovered(dir.path(), &value, &report);
}

// ── Invalid requests propagate (ER-2) ───────────────────────────────────────

/// Asserts `call` under `try` fails exactly as it does unguarded, with
/// `code`, and that the fallback never ran.
fn assert_not_recovered(dir: &Path, frontmatter: &str, call: &str, code: Option<&str>) -> Box<ComposeFailure> {
    let control = failure_in(dir, &DataRuntime::new(), frontmatter, call);
    let runtime = DataRuntime::new();
    let failure = failure_in(dir, &runtime, frontmatter, &guarded(call, FALLBACK));
    assert_eq!(data_error(&failure).code(), code, "{}", failure.error);
    assert_eq!(codes(&failure), codes(&control), "{}", failure.error);
    assert_eq!(data_error(&failure).to_string(), data_error(&control).to_string());
    assert!(!failure.read_set.contains(&entry(dir, "fallback.json")), "the fallback ran: {:?}", failure.read_set);
    assert_eq!(failure.read_set, control.read_set);
    failure
}

#[test]
fn unsupported_format_is_not_recovered() {
    let dir = fixture();
    write(dir.path(), "data.xyz", "[1,");
    let failure =
        assert_not_recovered(dir.path(), "", "load_data('data.xyz', 'string[]')", Some("dm.data.unsupported_format"));
    assert!(matches!(&*data_error(&failure), DataError::UnsupportedFormat { .. }));
}

/// A missing `.xyz` is the same invalid request: no file it could resolve to
/// is readable, so it never reaches `try` as a miss that would select the
/// fallback on every run. Its error names the file and its extension.
#[test]
fn missing_source_with_unsupported_extension_is_not_recovered() {
    let dir = fixture();
    let failure =
        assert_not_recovered(dir.path(), "", "load_data('missing.xyz', 'string[]')", Some("dm.data.unsupported_format"));
    match &*data_error(&failure) {
        DataError::UnsupportedFormat { identity, extension } => {
            assert_eq!(extension.as_deref(), Some("xyz"));
            assert!(identity.to_string().ends_with("missing.xyz"), "{identity}");
        }
        other => panic!("unsupported format: {other:?}"),
    }
    assert!(!dir.path().join("missing.xyz").exists());
}

/// The same `data.xyz`, read as JSON, is malformed, which recovers; so does a
/// missing `.xyz` file read as JSON.
#[test]
fn format_option_makes_unsupported_extension_readable() {
    let dir = fixture();
    write(dir.path(), "data.xyz", "[1,");
    for (file, missing) in [("data.xyz", false), ("missing.xyz", true)] {
        let call = format!("load_data('{file}', 'string[]', {{format: 'json'}})");
        let control = failure_of(dir.path(), &call);
        if missing {
            assert_eq!(file_reference_kind(&control), Some(FileRefFailure::NotFound), "{}", control.error);
        } else {
            assert_eq!(codes(&control), ["dm.data.malformed"], "{}", control.error);
        }
        let (value, report) = value_of(dir.path(), &guarded(&call, FALLBACK));
        assert_recovered(dir.path(), &value, &report);
    }
}

/// The options come from frontmatter, so only execution judges them.
#[test]
fn computed_invalid_options_are_not_recovered() {
    let dir = fixture();
    write(dir.path(), "rows.tsv", "a\tb\n1\t2\n");
    let call = "load_data('rows.tsv', 'tuple([string, string])[]', {delimiter: d})";
    let failure = assert_not_recovered(dir.path(), "d: \";\"", call, Some("dm.data.invalid_options"));
    assert!(matches!(&*data_error(&failure), DataError::InvalidOptions { key: "delimiter", .. }));
}

#[test]
fn headerless_object_rows_are_not_recovered() {
    let dir = fixture();
    write(dir.path(), "rows.csv", "name,port\nweb,8080\n");
    let call = "load_data('rows.csv', '{name: string, port: number}[]', {header: hdr})";
    assert_not_recovered(dir.path(), "hdr: false", call, Some("dm.data.row_shape_conflict"));
}

#[test]
fn missing_contract_import_is_not_recovered() {
    let dir = fixture();
    write(dir.path(), "ports.yaml", "- port: 1\n");
    let failure = assert_not_recovered(dir.path(), "", "load_data('ports.yaml', '{port: Port@./absent.yaml}[]')", None);
    assert!(matches!(&*data_error(&failure), DataError::ContractImport(_)), "{}", failure.error);
    assert!(failure.read_set.contains(&entry(dir.path(), "absent.yaml")), "{:?}", failure.read_set);
}

/// An import that is read but does not export the named type.
#[test]
fn invalid_contract_import_is_not_recovered() {
    let dir = fixture();
    write(dir.path(), "ports.yaml", "- port: 1\n");
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  Other: number\n");
    let failure = assert_not_recovered(dir.path(), "", "load_data('ports.yaml', '{port: Port@./types.yaml}[]')", None);
    assert!(matches!(&*data_error(&failure), DataError::ContractSchema { .. }), "{}", failure.error);
}

#[test]
fn not_a_container_is_not_recovered() {
    let dir = fixture();
    write(dir.path(), "os.yaml", OS_YAML);
    assert_not_recovered(dir.path(), "t: string", "load_data('os.yaml', t)", Some("dm.data.not_a_container"));
}

#[test]
fn this_import_is_not_recovered() {
    let dir = fixture();
    write(dir.path(), "os.yaml", OS_YAML);
    assert_not_recovered(dir.path(), "t: Os@this", "load_data('os.yaml', t)", Some("dm.data.this_import_rejected"));
}

// ── Request checks do not depend on the source existing ─────────────────────

/// The call every case below makes. Its inputs come from frontmatter, so the
/// passive call check cannot judge them and only execution does.
const COMPUTED_CALL: &str = "load_data(src, contract, opts)";

/// Frontmatter declaring and setting the computed inputs of [`COMPUTED_CALL`].
fn computed_request(src: &str, contract: &str, opts: &str) -> String {
    format!("$schema:\n  src: string\n  contract: string\n  opts: unknown\nsrc: {src:?}\ncontract: {contract:?}\nopts: {opts}")
}

/// Asserts the computed request for `src` fails with `code` under `try`,
/// exactly as unguarded, with `file` (where `src` finds it) present and
/// holding `body`, or missing.
fn assert_request_refused(src: &str, file: &str, body: Option<&str>, contract: &str, opts: &str, code: &str) {
    let dir = fixture();
    if let Some(body) = body {
        write(dir.path(), file, body);
    }
    assert_not_recovered(dir.path(), &computed_request(src, contract, opts), COMPUTED_CALL, Some(code));
    assert_eq!(dir.path().join(file).exists(), body.is_some(), "nothing created {file}");
}

/// One `missing` and one `present` test per invalid request: each fails with
/// the same code whether or not its file exists. The format aliases share one
/// applicability check, so one case per canonical format proves the wiring.
macro_rules! invalid_request_fails_whether_or_not_present {
    ($($case:ident: $src:literal at $file:literal = $body:expr, as $contract:literal, opts $opts:literal => $code:literal;)*) => {
        mod invalid_request_fails_whether_or_not_present {
            use super::*;
            $(
                mod $case {
                    use super::*;

                    #[test]
                    fn missing() {
                        assert_request_refused($src, $file, None, $contract, $opts, $code);
                    }

                    #[test]
                    fn present() {
                        assert_request_refused($src, $file, Some($body), $contract, $opts, $code);
                    }
                }
            )*
        }
    };
}

invalid_request_fails_whether_or_not_present! {
    yaml_delimiter: "data.yaml" at "data.yaml" = OS_YAML, as "string[]", opts "{delimiter: \";\"}" => "dm.data.invalid_options";
    json_delimiter: "data.json" at "data.json" = "[\"a\"]", as "string[]", opts "{delimiter: \";\"}" => "dm.data.invalid_options";
    jsonl_delimiter: "data.jsonl" at "data.jsonl" = "\"a\"\n", as "string[]", opts "{delimiter: \";\"}" => "dm.data.invalid_options";
    json5_delimiter: "data.json5" at "data.json5" = "['a']", as "string[]", opts "{delimiter: \";\"}" => "dm.data.invalid_options";
    toml_delimiter: "data.toml" at "data.toml" = "a = \"b\"\n", as "{a: string}", opts "{delimiter: \";\"}" => "dm.data.invalid_options";
    yaml_header: "data.yaml" at "data.yaml" = OS_YAML, as "string[]", opts "{header: false}" => "dm.data.invalid_options";
    tsv_semicolon: "rows.tsv" at "rows.tsv" = "a\tb\n1\t2\n", as "tuple([string, string])[]", opts "{delimiter: \";\"}" => "dm.data.invalid_options";
    csv_headerless_object_rows: "rows.csv" at "rows.csv" = "name,port\nweb,8080\n", as "{name: string, port: string}[]", opts "{header: false}" => "dm.data.row_shape_conflict";
    csv_string_rows: "rows.csv" at "rows.csv" = "a,b\n1,2\n", as "string[]", opts "{}" => "dm.data.row_shape_conflict";
    tsv_string_rows: "rows.tsv" at "rows.tsv" = "a\tb\n1\t2\n", as "string[]", opts "{}" => "dm.data.row_shape_conflict";
    explicit_format: "data.xyz" at "data.xyz" = OS_YAML, as "string[]", opts "{format: yaml, delimiter: \";\"}" => "dm.data.invalid_options";
    recursive_unsupported_extension: "%optional.xyz" at "nested/optional.xyz" = OS_YAML, as "string[]", opts "{}" => "dm.data.unsupported_format";
}

/// The valid counterparts: a missing file selects the fallback, a present
/// one loads.
macro_rules! valid_request_recovers_only_a_miss {
    ($($case:ident: $src:literal at $file:literal = $body:expr, as $contract:literal, opts $opts:literal => $loaded:expr;)*) => {
        mod valid_request_recovers_only_a_miss {
            use super::*;
            $(
                mod $case {
                    use super::*;

                    #[test]
                    fn missing() {
                        let dir = fixture();
                        let frontmatter = computed_request($src, $contract, $opts);
                        let (value, report) = value_in(dir.path(), &DataRuntime::new(), &frontmatter, &guarded(COMPUTED_CALL, FALLBACK));
                        assert_recovered(dir.path(), &value, &report);
                    }

                    #[test]
                    fn present() {
                        let dir = fixture();
                        write(dir.path(), $file, $body);
                        let frontmatter = computed_request($src, $contract, $opts);
                        let (value, report) = value_in(dir.path(), &DataRuntime::new(), &frontmatter, &guarded(COMPUTED_CALL, FALLBACK));
                        assert_eq!(value, $loaded);
                        assert!(!report.read_set.contains(&entry(dir.path(), "fallback.json")), "{:?}", report.read_set);
                    }
                }
            )*
        }
    };
}

valid_request_recovers_only_a_miss! {
    yaml_list: "data.yaml" at "data.yaml" = OS_YAML, as "string[]", opts "{}" => json!(["macOS", "Linux", "Windows"]);
    csv_tuple_rows: "rows.csv" at "rows.csv" = "a;b\n1;2\n", as "tuple([string, string])[]", opts "{delimiter: \";\"}" => json!([["1", "2"]]);
    explicit_format: "data.xyz" at "data.xyz" = OS_YAML, as "string[]", opts "{format: yaml}" => json!(["macOS", "Linux", "Windows"]);
    recursive_supported_extension: "%optional.yaml" at "nested/optional.yaml" = OS_YAML, as "string[]", opts "{}" => json!(["macOS", "Linux", "Windows"]);
}

// ── Environment-interpolated references ─────────────────────────────────────

/// The captured environment the requests below resolve `{{VAR}}` through.
fn reference_env() -> std::collections::HashMap<String, String> {
    std::collections::HashMap::from([
        ("DATA_FILE".to_string(), "optional.yaml".to_string()),
        ("DATA_EXT".to_string(), "yaml".to_string()),
        ("DATA_DIR".to_string(), "nested".to_string()),
    ])
}

/// Composes `expression` in `dir` under [`reference_env`], captured by the
/// request rather than read from the process.
fn compose_with_env(dir: &Path, expression: &str) -> Composed {
    let doc = write(dir, "doc.md", document("", expression));
    let snapshot = RequestSnapshot::new(dir).with_home(biscuit_file::home_dir()).with_env(reference_env());
    let request = ComposeRequest::prepare(ComposeOptions::new().with_source_file(&doc), &snapshot).expect("request");
    Markdown::try_from(doc.as_path()).expect("document loads").compose_with_warnings(&request)
}

/// The literal call `load_data("<src>", "string[]", <opts>)`: a literal
/// argument reaches `FileReference` with its placeholders unexpanded.
fn env_call(src: &str, opts: &str) -> String {
    format!("load_data(\"{src}\", 'string[]', {opts})")
}

/// One `missing` and one `present` test per reference whose filename,
/// extension, or directory comes from the captured environment: a valid
/// request recovers only a miss, an invalid one fails either way.
macro_rules! env_reference_missing_and_present {
    ($($case:ident: $src:literal at $file:literal, opts $opts:literal => $expect:expr;)*) => {
        mod env_reference_missing_and_present {
            use super::*;

            /// `Ok(loaded)` for a valid request, `Err(code)` for an invalid one.
            fn check(src: &str, file: &str, opts: &str, present: bool, expect: Result<Value, &str>) {
                let dir = fixture();
                if present {
                    write(dir.path(), file, "- loaded\n");
                }
                let call = env_call(src, opts);
                let guarded_call = guarded(&call, FALLBACK);
                match (expect, compose_with_env(dir.path(), &guarded_call)) {
                    (Ok(loaded), Ok((composed, report))) => {
                        let text = composed.content().trim().to_string();
                        let value = serde_json::from_str::<Value>(&text).unwrap_or_else(|e| panic!("{text:?}: {e}"))[0].clone();
                        if present {
                            assert_eq!(value, loaded);
                            assert!(!report.read_set.contains(&entry(dir.path(), "fallback.json")), "{:?}", report.read_set);
                        } else {
                            assert_recovered(dir.path(), &value, &report);
                        }
                    }
                    (Err(code), Err(failure)) => {
                        assert_eq!(data_error(&failure).code(), Some(code), "{}", failure.error);
                        let control = compose_with_env(dir.path(), &call).expect_err("unguarded call fails");
                        assert_eq!(codes(&failure), codes(&control), "{}", failure.error);
                        assert!(!failure.read_set.contains(&entry(dir.path(), "fallback.json")), "{:?}", failure.read_set);
                    }
                    (Ok(_), Err(failure)) => panic!("{guarded_call} failed: {}", failure.error),
                    (Err(code), Ok((composed, _))) => panic!("{guarded_call} composed instead of {code}: {}", composed.content()),
                }
                assert_eq!(dir.path().join(file).exists(), present, "nothing created {file}");
            }

            $(
                mod $case {
                    use super::*;

                    #[test]
                    fn missing() {
                        check($src, $file, $opts, false, $expect);
                    }

                    #[test]
                    fn present() {
                        check($src, $file, $opts, true, $expect);
                    }
                }
            )*
        }
    };
}

env_reference_missing_and_present! {
    recursive_env_file: "%{{DATA_FILE}}" at "nested/optional.yaml", opts "{}" => Ok(json!(["loaded"]));
    recursive_env_extension: "%optional.{{DATA_EXT}}" at "nested/optional.yaml", opts "{}" => Ok(json!(["loaded"]));
    recursive_env_file_delimiter: "%{{DATA_FILE}}" at "nested/optional.yaml", opts "{delimiter: ';'}" => Err("dm.data.invalid_options");
    recursive_env_dir: "%{{DATA_DIR}}/optional.yaml" at "nested/optional.yaml", opts "{}" => Ok(json!(["loaded"]));
    env_file: "{{DATA_FILE}}" at "optional.yaml", opts "{}" => Ok(json!(["loaded"]));
    env_extension: "optional.{{DATA_EXT}}" at "optional.yaml", opts "{}" => Ok(json!(["loaded"]));
    recursive_env_file_explicit_format: "%{{DATA_FILE}}" at "nested/optional.yaml", opts "{format: 'yaml'}" => Ok(json!(["loaded"]));
}

// ── The data memo ───────────────────────────────────────────────────────────

const BAD_CALL: &str = "load_data('bad.json', 'string[]')";

#[test]
fn second_unguarded_call_fails_from_memo() {
    let dir = fixture();
    write(dir.path(), "bad.json", "[1,");
    let runtime = DataRuntime::new();
    let (value, _) = value_in(dir.path(), &runtime, "", &guarded(BAD_CALL, "['first']"));
    assert_eq!(value, json!(["first"]));
    let after_guarded = runtime.stats();
    assert_eq!(after_guarded.reads, 1, "{after_guarded:?}");

    let failure = failure_in(dir.path(), &runtime, "", BAD_CALL);
    assert_eq!(codes(&failure), ["dm.data.malformed"], "{}", failure.error);
    assert_eq!(runtime.stats().reads, after_guarded.reads, "the failure was reused: {:?}", runtime.stats());
}

#[test]
fn different_fallbacks_stay_distinct() {
    let dir = fixture();
    write(dir.path(), "bad.json", "[1,");
    let runtime = DataRuntime::new();
    let content = format!(
        "{{{{ as_json([{}]) }}}} {{{{ as_json([{}]) }}}}\n",
        guarded(BAD_CALL, "['a']"),
        guarded(BAD_CALL, "['b']")
    );
    let (composed, _) = compose_in(dir.path(), &content, &runtime).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "[[\"a\"]] [[\"b\"]]");
    assert_eq!(runtime.stats().reads, 1, "{:?}", runtime.stats());
}

/// A source-layer failure (malformed) and a bind-layer failure (mismatch):
/// each later call is answered from its own layer with the failure, which
/// the next `try` absorbs with its own fallback and an unguarded call reports.
#[test]
fn fallback_never_enters_any_memo_layer() {
    let dir = fixture();
    write(dir.path(), "bad.json", "[1,");
    write(dir.path(), "os.yaml", OS_YAML);
    let runtime = DataRuntime::new();
    let mismatch = "load_data('os.yaml', 'number[]')";
    let calls = [guarded(BAD_CALL, "['a']"), guarded(mismatch, "['b']")];
    let first = format!("{{{{ as_json([{}, {}]) }}}}\n", calls[0], calls[1]);
    let (composed, _) = compose_in(dir.path(), &first, &runtime).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "[[\"a\"],[\"b\"]]");
    let warm = runtime.stats();

    let calls = [guarded(BAD_CALL, "['c']"), guarded(mismatch, "['d']")];
    let second = format!("{{{{ as_json([{}, {}]) }}}}\n", calls[0], calls[1]);
    let (composed, _) = compose_in(dir.path(), &second, &runtime).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "[[\"c\"],[\"d\"]]", "each fallback is its own call's");
    let reused = runtime.stats();
    assert_eq!((reused.reads, reused.parses, reused.binds), (warm.reads, warm.parses, warm.binds), "{reused:?}");
    assert!(reused.source_hits > warm.source_hits, "the malformed source was reused: {warm:?} -> {reused:?}");
    assert!(reused.bind_hits > warm.bind_hits, "the failed binding was reused: {warm:?} -> {reused:?}");

    assert_eq!(codes(&failure_in(dir.path(), &runtime, "", BAD_CALL)), ["dm.data.malformed"]);
    let mismatches = codes(&failure_in(dir.path(), &runtime, "", mismatch));
    assert!(!mismatches.is_empty() && mismatches.iter().all(|code| code == "dm.data.contract_mismatch"), "{mismatches:?}");
    let last = runtime.stats();
    assert_eq!((last.reads, last.parses, last.binds), (warm.reads, warm.parses, warm.binds), "{last:?}");
}

#[test]
fn object_mode_failure_does_not_poison_positional_mode() {
    let dir = fixture();
    write(dir.path(), "dup.csv", "a,a\n1,2\n");
    let runtime = DataRuntime::new();
    let object = "load_data('dup.csv', '{a: string}[]')";
    let (value, _) = value_in(dir.path(), &runtime, "", &guarded(object, "['fallback']"));
    assert_eq!(value, json!(["fallback"]));
    let positional = "load_data('dup.csv', 'tuple([string, string])[]', {header: false})";
    let (value, _) = value_in(dir.path(), &runtime, "", positional);
    assert_eq!(value, json!([["a", "a"], ["1", "2"]]));
    assert_eq!(codes(&failure_in(dir.path(), &runtime, "", object)), ["dm.data.malformed"]);
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.parses), (1, 1), "{stats:?}");
}

/// `md compose`'s passive reference check and composition, sharing one
/// runtime: the check's memoized failure answers composition.
#[test]
fn pipeline_passes_share_the_data_memo() {
    let dir = fixture();
    write(dir.path(), "bad.json", "[1,");
    let doc = write(dir.path(), "doc.md", document("", &guarded(BAD_CALL, "['fallback']")));
    let runtime = DataRuntime::new();
    let request = crate::request_support::request(
        ComposeOptions::new().with_data_runtime(runtime.clone()).with_source_file(&doc),
    );
    let md = Markdown::try_from(doc.as_path()).unwrap();
    let check = request.clone().map_options(ComposeOptions::for_reference_check);
    let report = md
        .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&check)))
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(report.is_valid(), "{:?}", report.issues);
    assert_eq!(runtime.stats().reads, 1, "the check read the file: {:?}", runtime.stats());

    let (composed, _) = md.compose_with_warnings(&request).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "[[\"fallback\"]]");
    let stats = runtime.stats();
    assert_eq!(stats.reads, 1, "{stats:?}");
    assert!(stats.source_hits >= 1, "composition reused the check's failure: {stats:?}");
}

struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// Allows every command, keeping each command line it was asked about.
#[derive(Default)]
struct Recording(Mutex<Vec<String>>);

impl ShellApprovalHandler for Recording {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.0.lock().unwrap().push(request.raw_command);
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// Options that run every command `dir`'s documents ask for, sharing `runtime`.
fn shell_options(dir: &Path, runtime: &DataRuntime) -> ComposeOptions {
    ComposeOptions::new()
        .with_shell(ShellExpansionOptions { policy_root: Some(dir.to_path_buf()), ..Default::default() })
        .with_shell_working_directory(dir)
        .with_shell_approval_handler(Arc::new(AllowAll))
        .with_data_runtime(runtime.clone())
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display())).replace("\r\n", "\n")
}

/// `d.toml`'s `a.b`, or 0 when `d.toml` cannot be loaded.
const GUARDED_B: &str = "try(load_data('d.toml', '{ a: { b: number } }').a.b, 0)";

/// The command creates `d.toml`, which `git config --file` writes as valid
/// TOML; the read after it sees the file the recovered read before it missed.
#[test]
fn executed_command_invalidates_recovered_failure() {
    let dir = fixture();
    let doc = write(
        dir.path(),
        "doc.md",
        format!("value: {{{{ {GUARDED_B} }}}}\n\n::shell git config --file d.toml a.b 5\n\nvalue: {{{{ {GUARDED_B} }}}}\n"),
    );
    let runtime = DataRuntime::new();
    let (composed, _) = compose_file(&doc, shell_options(dir.path(), &runtime))
        .unwrap_or_else(|failure| panic!("{}", failure.error));
    let values: Vec<&str> =
        composed.content().lines().filter_map(|line| line.trim().strip_prefix("value: ")).collect();
    assert_eq!(values, ["0", "5"], "{}", composed.content());
    let stats = runtime.stats();
    assert!(stats.invalidations >= 1 && stats.reads == 1, "{stats:?}");
}

// ── The read set ────────────────────────────────────────────────────────────

/// The read set of `call` unguarded (a failure or a success) and under a
/// `try` whose fallback reads nothing.
fn read_sets(dir: &Path, call: &str) -> (Vec<String>, Vec<String>) {
    let unguarded = match compose_in(dir, &document("", call), &DataRuntime::new()) {
        Ok((_, report)) => report.read_set,
        Err(failure) => failure.read_set,
    };
    let (_, report) = value_of(dir, &guarded(call, "[]"));
    (unguarded, report.read_set)
}

#[test]
fn guarded_read_set_equals_unguarded() {
    let dir = fixture();
    write(dir.path(), "os.yaml", OS_YAML);
    write(dir.path(), "bad.json", "[1,");
    for call in [
        "load_data('os.yaml', 'string[]')",
        "load_data('os.yaml', 'number[]')",
        "load_data('bad.json', 'string[]')",
        "load_data('missing.yaml', 'string[]')",
        "load_data('absent/missing.yaml', 'string[]')",
    ] {
        let (unguarded, guarded) = read_sets(dir.path(), call);
        assert_eq!(guarded, unguarded, "{call}");
    }
}

#[test]
fn resolved_failed_source_stays_in_read_set() {
    let dir = fixture();
    write(dir.path(), "bad.json", "[1,");
    write(dir.path(), "os.yaml", OS_YAML);
    for (call, file) in [(BAD_CALL, "bad.json"), ("load_data('os.yaml', 'number[]')", "os.yaml")] {
        let (_, report) = value_of(dir.path(), &guarded(call, "[]"));
        assert_eq!(report.read_set, vec![entry(dir.path(), file)], "{call}");
    }
}

/// A miss below directories that do not exist is recorded as the probed
/// candidate. No entry is the canonical spelling of that absent file, which
/// differs from the probed one where the temp directory sits behind a
/// symbolic link (macOS's `/var`).
#[test]
fn unresolved_miss_gets_no_canonical_entry() {
    let dir = fixture();
    let name = "absent/deeper/missing.yaml";
    let (value, report) = value_of(dir.path(), &guarded(&format!("load_data('{name}', 'string[]')"), "[]"));
    assert_eq!(value, json!([]));
    assert!(!report.read_set.is_empty(), "the probe is a dependency");
    let canonical = entry(dir.path(), name);
    for recorded in &report.read_set {
        assert!(!Path::new(recorded).exists(), "{recorded} names nothing that exists");
        assert!(recorded.ends_with(name), "{recorded}");
        let probed = biscuit_file::to_portable_string(&dir.path().join(name));
        if probed != canonical {
            assert_ne!(recorded, &canonical, "an absent file was given a canonical identity");
        }
    }
}

#[test]
fn executed_fallback_reads_are_recorded() {
    let dir = fixture();
    let call = "load_data('missing.yaml', 'string[]')";
    let (unguarded, _) = read_sets(dir.path(), call);
    let (_, report) = value_of(dir.path(), &guarded(call, FALLBACK));
    let mut expected = unguarded;
    expected.push(entry(dir.path(), "fallback.json"));
    expected.sort();
    assert_eq!(report.read_set, expected);
}

#[test]
fn unexecuted_fallback_adds_no_reads() {
    let dir = fixture();
    write(dir.path(), "os.yaml", OS_YAML);
    let runtime = DataRuntime::new();
    let (value, report) = value_in(dir.path(), &runtime, "", &guarded("load_data('os.yaml', 'string[]')", FALLBACK));
    assert_eq!(value, json!(["macOS", "Linux", "Windows"]));
    assert_eq!(report.read_set, vec![entry(dir.path(), "os.yaml")]);
    assert_eq!(runtime.read_set(), vec![entry(dir.path(), "os.yaml")]);
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.parses, stats.binds), (1, 1, 1), "{stats:?}");
}

// ── Remote reads ────────────────────────────────────────────────────────────

async fn serve(route: &str, body: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path(route)).respond_with(body).mount(&server).await;
    server
}

fn json_body(text: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).insert_header("cache-control", "max-age=3600").set_body_string(text)
}

fn allowing_loopback() -> ComposeOptions {
    ComposeOptions::new()
        .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
}

/// A run that allows only `example.com`, so `127.0.0.1` is refused.
fn denying_loopback() -> ComposeOptions {
    ComposeOptions::new()
        .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["example.com".into()], ..Default::default() })
}

async fn compose_blocking(doc: PathBuf, options: ComposeOptions) -> Composed {
    tokio::task::spawn_blocking(move || compose_file(&doc, options)).await.unwrap()
}

async fn requests(server: &MockServer) -> usize {
    server.received_requests().await.unwrap_or_default().len()
}

/// A document whose only content is `try(load_data(url), fallback)` as JSON.
fn remote_doc(dir: &Path, url: &str) -> PathBuf {
    write(dir, "doc.md", document("", &guarded(&format!("load_data('{url}', 'string[]')"), FALLBACK)))
}

/// The guarded call on `url` composes to the fallback; the same call
/// unguarded fails.
async fn assert_transport_recovers(url: &str) {
    let dir = fixture();
    let control = write(dir.path(), "control.md", document("", &format!("load_data('{url}', 'string[]')")));
    let failure = compose_blocking(control, allowing_loopback().with_cache_access_mode(CacheAccessMode::Off))
        .await
        .expect_err("the unguarded read fails");
    assert!(
        expression_errors(&failure).iter().any(|error| matches!(error, ExpressionError::RemoteRead { .. })),
        "{}",
        failure.error
    );

    let doc = remote_doc(dir.path(), url);
    let (composed, report) = compose_blocking(doc, allowing_loopback().with_cache_access_mode(CacheAccessMode::Off))
        .await
        .unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "[[\"fallback\"]]");
    assert!(report.read_set.contains(&url.to_string()), "{:?}", report.read_set);
    assert!(report.read_set.contains(&entry(dir.path(), "fallback.json")), "{:?}", report.read_set);
}

mod allowed_transport_failure_recovers {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_500() {
        let server = serve("/data.json", ResponseTemplate::new(500)).await;
        assert_transport_recovers(&format!("{}/data.json", server.uri())).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invalid_utf8_body() {
        let server = serve("/data.json", ResponseTemplate::new(200).set_body_bytes(b"[\"a\", \xff]".to_vec())).await;
        assert_transport_recovers(&format!("{}/data.json", server.uri())).await;
    }

    /// A port bound and released at once has no listener.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn connection_refused() {
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        assert_transport_recovers(&format!("http://127.0.0.1:{port}/data.json")).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn denied_host_is_fatal_with_warm_cache_and_memo() {
    let server = serve("/data.json", json_body("[\"cached\"]")).await;
    let dir = fixture();
    let cache = TempDir::new().unwrap();
    let url = format!("{}/data.json", server.uri());
    let doc = remote_doc(dir.path(), &url);
    let runtime = DataRuntime::new();
    let warmed = |options: ComposeOptions| {
        options
            .with_cache_root(cache.path())
            .with_cache_access_mode(CacheAccessMode::ReadWrite)
            .with_data_runtime(runtime.clone())
    };

    let (seeded, _) = compose_blocking(doc.clone(), warmed(allowing_loopback())).await.unwrap();
    assert_eq!(seeded.content().trim(), "[[\"cached\"]]");
    assert_eq!(requests(&server).await, 1);
    let warm = runtime.stats();

    let failure = compose_blocking(doc, warmed(denying_loopback())).await.unwrap_err();
    assert_eq!(file_reference_kind(&failure), Some(FileRefFailure::RemoteDenied), "{}", failure.error);
    assert_eq!(requests(&server).await, 1, "the denied run made no request");
    assert_eq!(failure.read_set, vec![url], "the fallback never ran");
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.source_hits, stats.bind_hits), (warm.reads, warm.source_hits, warm.bind_hits));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn denied_url_stays_in_read_set() {
    let server = serve("/data.json", json_body("[\"x\"]")).await;
    let dir = fixture();
    let url = format!("{}/data.json", server.uri());
    let failure = compose_blocking(remote_doc(dir.path(), &url), denying_loopback()).await.unwrap_err();
    assert_eq!(file_reference_kind(&failure), Some(FileRefFailure::RemoteDenied), "{}", failure.error);
    assert_eq!(failure.read_set, vec![url]);
    assert_eq!(requests(&server).await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redirect_to_denied_host_is_fatal() {
    // The target is a loopback server reached as `localhost`, a host the run
    // does not allow.
    let target = serve("/data.json", json_body("[\"target\"]")).await;
    let target_url = format!("http://localhost:{}/data.json", target.address().port());
    let origin =
        serve("/data.json", ResponseTemplate::new(302).insert_header("location", target_url.as_str())).await;
    let dir = fixture();
    let url = format!("{}/data.json", origin.uri());
    let failure = compose_blocking(remote_doc(dir.path(), &url), allowing_loopback().with_cache_access_mode(CacheAccessMode::Off))
        .await
        .unwrap_err();
    let message = failure.error.to_string();
    assert!(message.contains("blocked by fetch policy"), "{message}");
    assert_eq!(requests(&target).await, 0, "the redirect was not followed");
    assert_eq!(failure.read_set, vec![url], "the fallback never ran");
}

/// Serves `first` to the first request on `route` and `then` to every later one.
async fn serve_then(route: &str, first: ResponseTemplate, then: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    let hits = AtomicUsize::new(0);
    let responder = move |_: &wiremock::Request| {
        if hits.fetch_add(1, Ordering::SeqCst) == 0 { first.clone() } else { then.clone() }
    };
    Mock::given(method("GET")).and(path(route)).respond_with(responder).mount(&server).await;
    server
}

/// Loopback allowed, reading through the transport cache at `cache` under the
/// default (fallback) freshness mode.
fn through_cache(cache: &Path) -> ComposeOptions {
    allowing_loopback().with_cache_root(cache).with_cache_access_mode(CacheAccessMode::ReadWrite)
}

/// A stale cached body covers only a transport failure: a revalidation that
/// is refused (a redirect to a host the run does not allow) is a policy
/// error, so `try` propagates it and the cached value is never published.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn warm_cache_never_hides_a_refused_redirect() {
    let target = serve("/data.json", json_body("[\"target\"]")).await;
    let target_url = format!("http://localhost:{}/data.json", target.address().port());
    let origin = serve_then(
        "/data.json",
        ResponseTemplate::new(200).insert_header("cache-control", "max-age=0").set_body_string("[\"loaded\"]"),
        ResponseTemplate::new(302).insert_header("location", target_url.as_str()),
    )
    .await;
    let dir = fixture();
    let cache = TempDir::new().unwrap();
    let url = format!("{}/data.json", origin.uri());
    let doc = remote_doc(dir.path(), &url);

    let (seeded, _) = compose_blocking(doc.clone(), through_cache(cache.path())).await.unwrap();
    assert_eq!(seeded.content().trim(), "[[\"loaded\"]]", "the warm-up control loaded and cached the body");

    let failure = compose_blocking(doc, through_cache(cache.path())).await.unwrap_err();
    let kinds: Vec<RemoteReadKind> = expression_errors(&failure)
        .into_iter()
        .filter_map(|error| match error {
            ExpressionError::RemoteRead { kind, .. } => Some(*kind),
            _ => None,
        })
        .collect();
    assert_eq!(kinds.first(), Some(&RemoteReadKind::Policy), "{}", failure.error);
    assert!(failure.error.to_string().contains("blocked by fetch policy"), "{}", failure.error);
    assert_eq!(requests(&origin).await, 2, "the stale entry was revalidated");
    assert_eq!(requests(&target).await, 0, "the redirect was not followed");
    assert_eq!(failure.read_set, vec![url], "the fallback never ran");
}

/// The transport control for `warm_cache_never_hides_a_refused_redirect`: an
/// HTTP 500 on revalidation still serves the stale cached body.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn warm_cache_covers_a_transport_failure() {
    let origin = serve_then(
        "/data.json",
        ResponseTemplate::new(200).insert_header("cache-control", "max-age=0").set_body_string("[\"loaded\"]"),
        ResponseTemplate::new(500),
    )
    .await;
    let dir = fixture();
    let cache = TempDir::new().unwrap();
    let url = format!("{}/data.json", origin.uri());
    let doc = remote_doc(dir.path(), &url);

    compose_blocking(doc.clone(), through_cache(cache.path())).await.unwrap();
    let (stale, report) =
        compose_blocking(doc, through_cache(cache.path())).await.unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(stale.content().trim(), "[[\"loaded\"]]", "the stale body was served");
    assert_eq!(requests(&origin).await, 2, "the stale entry was revalidated");
    assert!(!report.read_set.contains(&entry(dir.path(), "fallback.json")), "{:?}", report.read_set);
}

/// As in `md compose`, the check runs before host permission is wired: it
/// defers the read, and composition, once allowed, loads the real data.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reference_check_deferral_loads_remote_data_at_composition() {
    let server = serve("/data.json", json_body("[\"macOS\", \"Linux\"]")).await;
    let dir = fixture();
    let url = format!("{}/data.json", server.uri());
    let doc = write(dir.path(), "doc.md", document("", &guarded(&format!("load_data('{url}', 'string[]')"), "[]")));
    let runtime = DataRuntime::new();
    let base = ComposeOptions::new()
        .with_source_file(&doc)
        .with_cache_access_mode(CacheAccessMode::Off)
        .with_data_runtime(runtime.clone());
    let check_runtime = runtime.clone();
    let (text, check_stats) = tokio::task::spawn_blocking(move || {
        let md = Markdown::try_from(doc.as_path()).unwrap();
        let check = crate::request_support::request(base.clone().for_reference_check());
        md.validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&check)))
            .unwrap_or_else(|error| panic!("{error}"));
        let check_stats = check_runtime.stats();
        let allowed = base
            .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
            .with_shared_remote_fetch();
        let (composed, _) = md
            .compose_with_warnings(&crate::request_support::request(allowed))
            .unwrap_or_else(|failure| panic!("{}", failure.error));
        (composed.content().trim().to_string(), check_stats)
    })
    .await
    .unwrap();
    assert_eq!(check_stats.reads, 0, "the check deferred the read and memoized nothing: {check_stats:?}");
    assert_eq!(text, "[[\"macOS\",\"Linux\"]]", "the fallback was not frozen");
    assert_eq!(requests(&server).await, 1);
}

// ── Approval discovery ──────────────────────────────────────────────────────

/// Pre-flight walks the command while `d.toml` is still missing: the guarded
/// read does not stop the walk, and the command is asked about as its
/// template, not as bytes a discovery-time fallback chose. Composition runs
/// the first command, reads `d.toml`, and runs `r.v 5`.
#[test]
fn approval_discovery_recovers_local_miss_without_freezing() {
    let dir = fixture();
    let doc = write(
        dir.path(),
        "doc.md",
        format!("::shell git config --file d.toml a.b 5\n\n::shell git config --file result.toml r.v {{{{ {GUARDED_B} }}}}\n"),
    );
    let runtime = DataRuntime::new();
    let request = crate::request_support::request(shell_options(dir.path(), &runtime).with_source_file(&doc));
    let md = Markdown::try_from(doc.as_path()).unwrap();
    let recording = Arc::new(Recording::default());
    md.compose_preflight_approvals(&request, Some(recording.clone())).unwrap_or_else(|error| panic!("{error}"));
    let asked = recording.0.lock().unwrap().clone();
    assert!(asked.iter().any(|command| command.ends_with(&format!("r.v {{{{ {GUARDED_B} }}}}"))), "{asked:?}");
    assert!(!asked.iter().any(|command| command.ends_with("r.v 0")), "a fallback was frozen: {asked:?}");
    assert!(!dir.path().join("d.toml").exists(), "pre-flight ran nothing");

    md.compose_with_warnings(&request).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(read(&dir.path().join("result.toml")), "[r]\n\tv = 5\n", "the discovery value was not frozen");
}

/// The transclusion target recovers to the fallback document during the
/// walk, and that document's templated command joins the approval batch.
#[test]
fn approval_discovery_keeps_fallback_requirements_visible() {
    let dir = fixture();
    write(dir.path(), "fallback.md", "::shell git config --file out.toml k.v {{ 1 + 1 }}\n");
    let doc = write(
        dir.path(),
        "doc.md",
        "::file {{ try(load_data('target.json', '{ file: string }').file, 'fallback.md') }}\n",
    );
    let runtime = DataRuntime::new();
    let request = crate::request_support::request(shell_options(dir.path(), &runtime).with_source_file(&doc));
    let md = Markdown::try_from(doc.as_path()).unwrap();
    let recording = Arc::new(Recording::default());
    md.compose_preflight_approvals(&request, Some(recording.clone())).unwrap_or_else(|error| panic!("{error}"));
    let asked = recording.0.lock().unwrap().clone();
    assert!(asked.iter().any(|command| command.contains("out.toml k.v")), "{asked:?}");

    md.compose_with_warnings(&request).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(read(&dir.path().join("out.toml")), "[k]\n\tv = 2\n");
}
