//! The command-wide `load_data` runtime observed through public passes:
//! reuse across transclusion, the reference check, pre-flight, and
//! composition; the emptying rule after commands and lifecycle steps; remote
//! policy and transport caching; and the read set of each evaluation surface
//! and read-side function.
//!
//! Every fixture lives in its own temporary directory and every request is
//! anchored there, so no test reads this repository or the process
//! directory. Commands that write a data file use `git config --file`, which
//! rewrites one key of an INI-style file in place: the result is valid TOML,
//! and `git` is on every host these tests run on.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use darkmatter::effects::EffectEngine;
use darkmatter::lifecycle::md_host::{self, CommandOutput, MdEvent, MdRuntimeState, MdStackContext};
use darkmatter::lifecycle::run::ShellRefusal;
use darkmatter::lifecycle::{LifecycleEffects, LifecycleSignal, LifecycleTiming};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::DataError;
use darkmatter::markdown::compose::expression::{ExpressionError, FileRefFailure};
use darkmatter::markdown::compose::remote::RemoteReadConfig;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions, ShellRuleSet,
};
use darkmatter::markdown::compose::{
    CacheAccessMode, ComposeFailure, ComposeOptions, ComposeReport, ComposeRequest, DataRuntime, SharedFrontmatter,
};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Fixtures ────────────────────────────────────────────────────────────────

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

type Composed = Result<(Markdown, ComposeReport), Box<ComposeFailure>>;

fn compose_file(path: &Path, options: ComposeOptions) -> Composed {
    let request = crate::request_support::request(options.with_source_file(path));
    Markdown::try_from(path).expect("document loads").compose_with_warnings(&request)
}

fn composed(path: &Path, options: ComposeOptions) -> (String, ComposeReport) {
    match compose_file(path, options) {
        Ok((composed, report)) => (composed.content().trim().to_string(), report),
        Err(failure) => panic!("{} failed: {}", path.display(), failure.error),
    }
}

fn composed_text(path: &Path, options: ComposeOptions) -> String {
    composed(path, options).0
}

/// The kind of the file-reference failure in `failure`'s cause chain, raised
/// directly or carried by a `load_data` failure.
fn file_reference_kind(failure: &ComposeFailure) -> FileRefFailure {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&failure.error);
    while let Some(error) = current {
        let expression = error
            .downcast_ref::<ExpressionError>()
            .or_else(|| error.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref));
        match expression.map(ExpressionError::without_callbacks) {
            Some(ExpressionError::FileReference(diagnostic)) => return diagnostic.kind,
            Some(ExpressionError::Data(data)) => {
                if let DataError::FileReference(diagnostic) = &**data {
                    return diagnostic.kind;
                }
            }
            _ => {}
        }
        current = error.source();
    }
    panic!("no file-reference failure in: {}", failure.error)
}

struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
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
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

const OS_YAML: &str = "- macOS\n- Linux\n- Windows\n";

// ── R7: one read per command ────────────────────────────────────────────────

/// Transclusion composes its children concurrently. That concurrent callers
/// for one key wait on the first loader is proven with a barrier by the
/// runtime's own unit test; this proves the composed path shares the entry.
#[test]
fn two_transcluded_children_loading_one_file_share_one_read() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", OS_YAML);
    write(dir.path(), "a.md", "a: {{ as_csv(load_data(\"os.yaml\", \"string[]\")) }}\n");
    write(dir.path(), "b.md", "b: {{ length(load_data(\"os.yaml\", \"string[]\")) }}\n");
    let doc = write(dir.path(), "doc.md", "::file ./a.md\n\n::file ./b.md\n");
    let runtime = DataRuntime::new();
    let text = composed_text(&doc, ComposeOptions::new().with_data_runtime(runtime.clone()));
    assert!(text.contains("a: macOS, Linux, Windows") && text.contains("b: 3"), "{text}");
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.parses, stats.source_hits), (1, 1, 1), "{stats:?}");
}

#[test]
fn two_library_compositions_without_a_runtime_do_not_share_results() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[\"first\"]");
    let doc = write(dir.path(), "doc.md", "{{ as_csv(load_data(\"d.json\", \"string[]\")) }}\n");
    assert_eq!(composed_text(&doc, ComposeOptions::new()), "first");
    write(dir.path(), "d.json", "[\"second\"]");
    assert_eq!(composed_text(&doc, ComposeOptions::new()), "second", "the first run's result was reused");
}

/// The passes `md compose` runs for a file with commands enabled, in its
/// order, all sharing one runtime: the passive reference check, the
/// shell-approval pre-flight walk, and composition.
#[test]
fn the_reference_check_preflight_and_composition_read_once() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", OS_YAML);
    let doc = write(
        dir.path(),
        "doc.md",
        "---\nhosts: \"{{ as_csv(load_data('os.yaml', 'string[]')) }}\"\n---\n\
         {{ hosts }} / {{ length(load_data(\"os.yaml\", \"string[]\")) }}\n",
    );
    let runtime = DataRuntime::new();
    let request = crate::request_support::request(shell_options(dir.path(), &runtime).with_source_file(&doc));
    let md = Markdown::try_from(doc.as_path()).unwrap();

    let check = request.clone().map_options(ComposeOptions::for_reference_check);
    let report = md
        .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&check)))
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(report.is_valid(), "{:?}", report.issues);
    let after_check = runtime.stats();
    assert_eq!(after_check.reads, 1, "the check read the file: {after_check:?}");

    md.compose_preflight_approvals(&request, Some(Arc::new(AllowAll))).unwrap_or_else(|error| panic!("{error}"));
    let after_preflight = runtime.stats();

    let (composed, _) = md.compose_with_warnings(&request).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "macOS, Linux, Windows / 3");
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.parses, stats.binds), (1, 1, 1), "{stats:?}");
    assert!(
        after_preflight.source_hits >= after_check.source_hits && stats.source_hits > after_preflight.source_hits,
        "check {after_check:?}, pre-flight {after_preflight:?}, composition {stats:?}"
    );
}

/// Body interpolation runs before any `::shell`, but a command template's
/// placeholders are filled as each command runs, so the second item's call
/// comes after the first item's command.
#[test]
fn a_shell_step_inside_a_map_body_makes_the_next_item_read_again() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.toml", "[a]\nb = 1\n");
    let doc = write(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  items: number[]\nitems: [10, 20]\n---\n\
         ::map items \"i => i\"\n\
         ::shell git config --file d.toml a.b {{ load_data('d.toml', '{ a: { b: number } }').a.b + i }}\n\n\
         ::end-map\n",
    );
    let runtime = DataRuntime::new();
    let (_, report) = composed(&doc, shell_options(dir.path(), &runtime));
    // 1 + 10, then 11 + 20: the second fill saw the first command's write.
    assert_eq!(read(&dir.path().join("d.toml")).replace("\r\n", "\n"), "[a]\n\tb = 31\n");
    assert_eq!(runtime.stats().reads, 2, "{:?}", runtime.stats());
    assert_eq!(report.read_set, vec![entry(dir.path(), "d.toml")], "a command template's read is recorded");
}

/// Each item's command writes the file and then fails: `git config --get` of
/// a missing key exits 1 after the chain's first segment ran, and
/// `--when-error` keeps the run going.
#[test]
fn a_failed_command_that_wrote_a_file_ends_reuse() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.toml", "[a]\nb = 1\n");
    let doc = write(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  items: number[]\nitems: [10, 20]\n---\n\
         ::map items \"i => i\"\n\
         ::shell --when-error \"failed\" git config --file d.toml a.b \
         {{ load_data('d.toml', '{ a: { b: number } }').a.b + i }} && git config --file d.toml --get no.such\n\n\
         ::end-map\n",
    );
    let runtime = DataRuntime::new();
    let text = composed_text(&doc, shell_options(dir.path(), &runtime));
    assert_eq!(text.matches("failed").count(), 2, "{text}");
    // 1 + 10, then 11 + 20: the second fill saw the failed command's write.
    assert_eq!(read(&dir.path().join("d.toml")).replace("\r\n", "\n"), "[a]\n\tb = 31\n");
    assert_eq!(runtime.stats().reads, 2, "{:?}", runtime.stats());
}

// ── R7: lifecycle steps ─────────────────────────────────────────────────────

struct Silent;

impl LifecycleEffects for Silent {
    fn stdout(&self, _: &str) {}
    fn stderr(&self, _: LifecycleSignal, _: &str) {}
    fn info(&self, _: &str) {}
    fn warn(&self, _: &str) {}
    fn success(&self, _: &str) {}
    fn message(&self, _: &str) {}
    fn speech(&self, _: &str) {}
    fn effect(&self, _: &str) {}
    fn notification(&self, _: &str) {}
}

impl CommandOutput for Silent {
    fn stdout(&self, _: &str) {}
    fn stderr(&self, _: &str) {}
}

/// Runs `signal`'s stack of `lifecycle` for `source` through the `md` host, in
/// `request`, with shells locked, returning the frontmatter `set` produced.
fn run_event(request: &ComposeRequest, source: &Path, signal: LifecycleSignal, lifecycle: Value) -> Value {
    let config = md_host::parse_lifecycle(&lifecycle, source, None)
        .unwrap_or_else(|error| panic!("{lifecycle} parses: {error}"));
    let frontmatter = SharedFrontmatter::from(serde_json::Map::new());
    let live = Mutex::new(frontmatter.clone());
    let state = MdRuntimeState::default();
    let approved = ShellPreApprovals::new();
    let blacklist = ShellRuleSet::default();
    let root = source.parent().expect("source folder");
    let engine = EffectEngine::builder().mutation_root(root).auto_rehash(false).build();
    let timing = LifecycleTiming::default();
    let never = || false;
    let context = MdStackContext {
        request,
        source_path: source,
        frontmatter: &frontmatter,
        live_frontmatter: &live,
        state: &state,
        approved: &approved,
        user_blacklist: &blacklist,
        effect_engine: &engine,
        effects: &Silent,
        output: &Silent,
        timing: &timing,
        interrupted: &never,
    };
    let name = match signal {
        LifecycleSignal::Initialize => "initialize",
        _ => "composed",
    };
    let event = MdEvent { signal, name, error: None, shells: Err(ShellRefusal::Locked) };
    let outcome = md_host::evaluate(&context, &config, &event);
    assert!(outcome.evaluation_error.is_none() && outcome.action_error.is_none(), "{outcome:?}");
    Value::Object(state.snapshot().into_iter().collect())
}

/// `md compose`'s lifecycle order: the shell-free frontmatter bootstrap,
/// `initialize`, then composition, all in one request.
#[test]
fn a_read_in_the_frontmatter_bootstrap_is_not_reused_after_initialize() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.yaml", "- a\n");
    let doc = write(
        dir.path(),
        "doc.md",
        "---\ncount: \"{{ length(load_data('d.yaml', 'string[]')) }}\"\n---\n{{ count }}\n",
    );
    let runtime = DataRuntime::new();
    let request =
        crate::request_support::request(ComposeOptions::new().with_source_file(&doc).with_data_runtime(runtime.clone()));
    let md = Markdown::try_from(doc.as_path()).unwrap();

    let bootstrap = request.clone().map_options(|options| options.with_deferred_schema_verdict(true).only_frontmatter_surface());
    let (projected, _) = md.compose_with_warnings(&bootstrap).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(projected.frontmatter().as_map().get("count"), Some(&json!(1)));
    assert_eq!(runtime.stats().reads, 1);

    run_event(&request, &doc, LifecycleSignal::Initialize, json!({ "initialize": [{ "append_line": ["d.yaml", "- b"] }] }));

    let (composed, _) = md.compose_with_warnings(&request).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "2", "composition read the file `initialize` changed");
    assert_eq!(runtime.stats().reads, 2, "{:?}", runtime.stats());
}

// ── R8: the read set of each evaluation surface ─────────────────────────────

#[test]
fn a_nested_as_markdown_child_records_its_reads() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "t.md", "# Tee\n");
    let doc = write(dir.path(), "doc.md", "{{ as_markdown(\"{{ markdown_title('t.md') }}\") }}\n");
    let (text, report) = composed(&doc, ComposeOptions::new());
    assert_eq!(text, "Tee");
    assert_eq!(report.read_set, vec![entry(dir.path(), "t.md")]);
}

fn lifecycle_request(dir: &Path, runtime: &DataRuntime) -> (ComposeRequest, PathBuf) {
    let doc = write(dir, "doc.md", "body\n");
    let options = ComposeOptions::new().with_source_file(&doc).with_data_runtime(runtime.clone());
    (crate::request_support::request(options), doc)
}

#[test]
fn an_initialize_expression_records_its_reads() {
    let dir = TempDir::new().unwrap();
    let runtime = DataRuntime::new();
    let (request, doc) = lifecycle_request(dir.path(), &runtime);
    let set = run_event(
        &request,
        &doc,
        LifecycleSignal::Initialize,
        json!({ "initialize": [{ "set": { "ready": "{{ file_exists('gate.md') }}" } }] }),
    );
    assert_eq!(set, json!({ "ready": false }));
    assert_eq!(runtime.read_set(), vec![entry(dir.path(), "gate.md")]);
}

#[test]
fn a_composed_expression_records_its_reads() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "t.md", "# Tee\n");
    let runtime = DataRuntime::new();
    let (request, doc) = lifecycle_request(dir.path(), &runtime);
    let set = run_event(
        &request,
        &doc,
        LifecycleSignal::Start,
        json!({ "composed": [{ "set": { "title": "{{ markdown_title('t.md') }}" } }] }),
    );
    assert_eq!(set, json!({ "title": "Tee" }));
    assert_eq!(runtime.read_set(), vec![entry(dir.path(), "t.md")]);
}

// ── R8: every read-side function records its file ───────────────────────────

const PAGE: &str = "---\ntitle: Page\n---\nbody\n";

macro_rules! records_its_file {
    ($($name:ident: $file:literal, $content:expr, $call:literal;)+) => {$(
        #[test]
        fn $name() {
            let dir = TempDir::new().unwrap();
            write(dir.path(), $file, $content);
            let doc = write(dir.path(), "doc.md", concat!("{{ as_json([", $call, "]) }}\n"));
            let (_, report) = composed(&doc, ComposeOptions::new());
            assert_eq!(report.read_set, vec![entry(dir.path(), $file)]);
        }
    )+};
}

records_its_file! {
    frontmatter_records_its_file: "p.md", PAGE, "frontmatter('p.md')";
    try_frontmatter_records_its_file: "p.md", PAGE, "try_frontmatter('p.md')";
    file_exists_records_its_file: "p.md", PAGE, "file_exists('p.md')";
    markdown_title_records_its_file: "p.md", PAGE, "markdown_title('p.md')";
    markdown_body_empty_records_its_file: "p.md", PAGE, "markdown_body_empty('p.md')";
    validate_schema_records_its_file: "p.md", PAGE, "validate_schema('p.md')";
    load_data_records_its_file: "d.json", "[\"x\"]", "load_data('d.json', 'string[]')";
}

/// `s.yaml` imports `types.yaml`, which imports `inner.yaml`.
fn schema_chain(dir: &Path) {
    write(dir, "s.yaml", "$schema:\n  title: Title@./types.yaml\n");
    write(dir, "types.yaml", "kind: schema\ntypes:\n  Title: Inner@./inner.yaml\n");
    write(dir, "inner.yaml", "kind: schema\ntypes:\n  Inner: string\n");
}

#[test]
fn validate_schema_records_the_schema_files_it_reads_with_their_imports() {
    let dir = TempDir::new().unwrap();
    schema_chain(dir.path());
    write(dir.path(), "p.md", "---\n$schema: ./s.yaml\ntitle: Page\n---\nbody\n");
    let doc = write(dir.path(), "doc.md", "{{ validate_schema('p.md') }}\n");
    let (text, report) = composed(&doc, ComposeOptions::new());
    assert_eq!(text, "true");
    let mut expected: Vec<String> =
        ["inner.yaml", "p.md", "s.yaml", "types.yaml"].iter().map(|name| entry(dir.path(), name)).collect();
    expected.sort();
    assert_eq!(report.read_set, expected);
}

#[test]
fn validate_schema_records_no_entry_for_an_embedded_schema() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "p.md", "---\n$schema:\n  steps: shell[]@darkmatter:lifecycle\n---\nbody\n");
    let doc = write(dir.path(), "doc.md", "{{ validate_schema('p.md') }}\n");
    let (text, report) = composed(&doc, ComposeOptions::new());
    assert_eq!(text, "true");
    assert_eq!(report.read_set, vec![entry(dir.path(), "p.md")]);
}

#[test]
fn a_load_data_contract_records_its_transitive_imports() {
    let dir = TempDir::new().unwrap();
    schema_chain(dir.path());
    write(dir.path(), "d.json", "[\"x\"]");
    let doc = write(dir.path(), "doc.md", "{{ as_csv(load_data('d.json', 'Title[]@./types.yaml')) }}\n");
    let (text, report) = composed(&doc, ComposeOptions::new());
    assert_eq!(text, "x");
    let mut expected: Vec<String> =
        ["d.json", "inner.yaml", "types.yaml"].iter().map(|name| entry(dir.path(), name)).collect();
    expected.sort();
    assert_eq!(report.read_set, expected);
}

// ── R6/R7: remote reads ─────────────────────────────────────────────────────

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_url_the_reference_check_could_not_read_is_read_by_composition() {
    let server = serve("/data.json", json_body("[\"macOS\", \"Linux\"]")).await;
    let dir = TempDir::new().unwrap();
    let url = format!("{}/data.json", server.uri());
    let doc = write(dir.path(), "doc.md", format!("{{{{ as_csv(load_data(\"{url}\", \"string[]\")) }}}}\n"));
    let runtime = DataRuntime::new();
    let base = ComposeOptions::new().with_data_runtime(runtime.clone()).with_source_file(&doc);
    let check_runtime = runtime.clone();
    let (text, check_reads, check_read_set) = tokio::task::spawn_blocking(move || {
        let md = Markdown::try_from(doc.as_path()).unwrap();
        // As in `md compose`, the check runs before host permission is wired.
        let check = crate::request_support::request(base.clone().for_reference_check());
        md.validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&check)))
            .unwrap_or_else(|error| panic!("{error}"));
        let check_reads = check_runtime.stats();
        let check_read_set = check_runtime.read_set();
        let allowed = base
            .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
            .with_shared_remote_fetch();
        let request = crate::request_support::request(allowed);
        let (composed, _) = md.compose_with_warnings(&request).unwrap_or_else(|failure| panic!("{}", failure.error));
        (composed.content().trim().to_string(), check_reads, check_read_set)
    })
    .await
    .unwrap();
    assert_eq!(check_read_set, vec![url], "the check saw the call");
    assert_eq!(check_reads.reads, 0, "the check deferred the read and memoized nothing");
    assert_eq!(text, "macOS, Linux");
    assert_eq!(requests(&server).await, 1);
    assert_eq!(runtime.stats().reads, 1, "{:?}", runtime.stats());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_denied_host_reaches_neither_the_server_nor_a_seeded_cache() {
    let server = serve("/data.json", json_body("[\"cached\"]")).await;
    let dir = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    let url = format!("{}/data.json", server.uri());
    let doc = write(dir.path(), "doc.md", format!("{{{{ as_csv(load_data(\"{url}\", \"string[]\")) }}}}\n"));
    let cached = |options: ComposeOptions| {
        options.with_cache_root(cache.path()).with_cache_access_mode(CacheAccessMode::ReadWrite)
    };

    // An allowed run seeds a fresh transport-cache entry.
    let (seeded, _) = compose_blocking(doc.clone(), cached(allowing_loopback())).await.unwrap();
    assert_eq!(seeded.content().trim(), "cached");
    assert_eq!(requests(&server).await, 1);

    let failure = compose_blocking(doc.clone(), cached(denying_loopback())).await.unwrap_err();
    assert_eq!(file_reference_kind(&failure), FileRefFailure::RemoteDenied, "{}", failure.error);
    assert_eq!(requests(&server).await, 1, "the denied run made no request");
    assert_eq!(failure.read_set, vec![url], "a denied URL is still an observation");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_computed_url_loads_like_a_literal_one() {
    let server = serve("/data.json", json_body("[\"macOS\", \"Linux\"]")).await;
    let dir = TempDir::new().unwrap();
    let doc = write(
        dir.path(),
        "doc.md",
        format!(
            "---\nbase: \"{}\"\n---\n{{{{ as_csv(load_data(base + \"/data.json\", \"string[]\")) }}}} \
             {{{{ length(load_data(base + \"/data.json\", \"string[]\")) }}}}\n",
            server.uri()
        ),
    );
    let (allowed, report) = compose_blocking(doc.clone(), allowing_loopback()).await.unwrap();
    assert_eq!(allowed.content().trim(), "macOS, Linux 2");
    assert_eq!(report.read_set, vec![format!("{}/data.json", server.uri())]);
    assert_eq!(requests(&server).await, 1, "one fetch for two computed calls");

    let failure = compose_blocking(doc, denying_loopback()).await.unwrap_err();
    assert_eq!(file_reference_kind(&failure), FileRefFailure::RemoteDenied, "{}", failure.error);
    assert_eq!(requests(&server).await, 1, "a denied computed URL makes no request");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_invalid_utf8_response_is_a_transport_failure_without_a_data_location() {
    let server = serve("/data.json", ResponseTemplate::new(200).set_body_bytes(b"[\"a\", \xff]".to_vec())).await;
    let dir = TempDir::new().unwrap();
    let url = format!("{}/data.json", server.uri());
    let doc = write(dir.path(), "doc.md", format!("x {{{{ load_data(\"{url}\", \"string[]\") }}}}\n"));
    let failure = compose_blocking(doc, allowing_loopback()).await.unwrap_err();
    let records = error_diagnostics(&failure.error);
    assert_eq!(records.len(), 1, "{records:#?}");
    // Not `dm.data.malformed`, which a local file's invalid UTF-8 reports.
    assert_eq!(records[0].code, "dm.expression.evaluation_failure");
    assert_eq!(records[0].data_location, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_run_replays_the_cached_response() {
    let server = serve("/data.json", json_body("[\"macOS\"]")).await;
    let dir = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    let url = format!("{}/data.json", server.uri());
    let doc = write(dir.path(), "doc.md", format!("{{{{ as_csv(load_data(\"{url}\", \"string[]\")) }}}}\n"));
    for run in ["first", "second"] {
        let options = allowing_loopback().with_cache_root(cache.path()).with_cache_access_mode(CacheAccessMode::ReadWrite);
        let (composed, _) = compose_blocking(doc.clone(), options).await.unwrap_or_else(|failure| panic!("{run}: {}", failure.error));
        assert_eq!(composed.content().trim(), "macOS", "{run}");
    }
    assert_eq!(requests(&server).await, 1, "the second run replayed the transport cache");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_url_is_fetched_once_per_run_with_the_cache_off() {
    let server = serve("/data.json", json_body("[\"macOS\"]")).await;
    let dir = TempDir::new().unwrap();
    let url = format!("{}/data.json", server.uri());
    let doc = write(
        dir.path(),
        "doc.md",
        format!(
            "---\n$schema:\n  items: string[]\nitems: [a, b, c]\n---\n\
             ::map items \"i => i\"\n- {{{{ as_csv(load_data(\"{url}\", \"string[]\")) }}}}\n::end-map\n"
        ),
    );
    let runtime = DataRuntime::new();
    let options = allowing_loopback().with_cache_access_mode(CacheAccessMode::Off).with_data_runtime(runtime.clone());
    let (composed, _) = compose_blocking(doc, options).await.unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().matches("macOS").count(), 3, "{}", composed.content());
    assert_eq!(requests(&server).await, 1);
    assert_eq!(runtime.stats().reads, 1, "{:?}", runtime.stats());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_url_in_the_read_set_is_normalized() {
    let server = serve("/data.json", json_body("[\"macOS\"]")).await;
    let dir = TempDir::new().unwrap();
    let authority = server.uri().trim_start_matches("http://").to_string();
    let doc = write(
        dir.path(),
        "doc.md",
        format!("{{{{ as_csv(load_data(\"HTTP://{authority}/dir/../data.json?v=1\", \"string[]\")) }}}}\n"),
    );
    let (composed, report) = compose_blocking(doc, allowing_loopback()).await.unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "macOS");
    assert_eq!(report.read_set, vec![format!("http://{authority}/data.json?v=1")]);
}
