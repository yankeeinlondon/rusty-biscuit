//! Composing a document a registered host identifies, as a caller that is
//! not that host (`md compose`): the passive selected-host view in
//! [`ComposeOptions::with_selected_host`], the execution-only lifecycle
//! projection and deferred keys in `md_host`, and the shared stacks run
//! in-process through the recorder fixture.
//!
//! The `relay` host is shaped like Claudine: `composed` → `start`, `failed`
//! → `blocked`, `success`/`failure` of its own, and a `seq` global.

use std::sync::Arc;

use crate::host_fixtures::Fixture;
use crate::host_recorder::record_run;
use darkmatter::config::registration::{HOST_RUNTIME_ONLY_CODE, HostSelection, RegisteredHostProfile, RegisteredHosts};
use darkmatter::lifecycle::md_host;
use darkmatter::lifecycle::run::Finish;
use darkmatter::markdown::compose::expression::{Availability, BindingView, HostGlobals, ScopeId};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Map, Value, json};

const RELAY: &str = "../relay/darkmatter-host.yaml";
const UNDECLARED: &str = "dm.expression.undeclared_property";

/// A fixture with the relay installed and loaded, and its one profile.
fn relay() -> (Fixture, Arc<RegisteredHosts>, Arc<RegisteredHostProfile>) {
    let fixture = Fixture::new();
    fixture.install_relay("relay");
    let hosts = fixture.hosts(&[RELAY]);
    assert!(hosts.diagnostics().is_empty(), "the relay loads: {:?}", hosts.diagnostics());
    let profile = Arc::clone(&hosts.loaded().profiles[0]);
    (fixture, hosts, profile)
}

/// `text` written as `work/doc.md` with a request for it: the relay's
/// registered layer, its selected view, and its deferred keys, as `md
/// compose` prepares them.
fn foreign_request(fixture: &Fixture, hosts: &Arc<RegisteredHosts>, text: &str) -> (Markdown, ComposeRequest) {
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", text);
    let profile = Arc::clone(&hosts.loaded().profiles[0]);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&fixture.path("work"), ""))
        .with_source_file(fixture.path("work/doc.md"))
        .with_trigger_schemas(true)
        .with_exclude_keys(md_host::foreign_deferred_keys(profile.lifecycle().registry()))
        .with_registered_hosts(Arc::clone(hosts))
        .with_selected_host(profile);
    (markdown, ComposeRequest::with_context(options, context).expect("a request"))
}

fn frontmatter_of(markdown: &Markdown) -> Map<String, Value> {
    markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect()
}

fn undeclared(report: &darkmatter::markdown::compose::ComposeReport) -> Vec<String> {
    report.warnings.iter().filter(|warning| warning.code.as_deref() == Some(UNDECLARED)).map(|warning| warning.message.clone()).collect()
}

// ── the projection ──────────────────────────────────────────────────────────

#[test]
fn a_renaming_host_shares_initialize_cancelled_and_finalize_with_md() {
    let (_fixture, _hosts, profile) = relay();

    let shared = md_host::shared_event_keys(profile.lifecycle().registry());

    assert_eq!(shared, ["initialize", "cancelled", "finalize"]);
    assert_eq!(md_host::shared_event_keys(md_host::registry()), ["initialize", "composed", "failed", "cancelled", "finalize"]);
}

#[test]
fn the_execution_projection_keeps_only_the_shared_events() {
    let (_fixture, _hosts, profile) = relay();
    let frontmatter = json!({
        "sequence": [{ "name": "one", "prompt": "hi" }],
        "initialize": [{ "info": "i" }],
        "start": [{ "info": "s" }],
        "composed": [{ "info": "c" }],
        "failed": [{ "info": "f" }],
        "success": [{ "info": "ok" }],
        "finalize": [{ "info": "z" }],
    });

    let projection = md_host::execution_projection(&frontmatter, profile.lifecycle().registry());

    assert_eq!(projection, json!({ "initialize": [{ "info": "i" }], "finalize": [{ "info": "z" }] }));
}

#[test]
fn every_host_event_key_is_deferred_and_composed_and_failed_are_left_to_the_schema() {
    let (_fixture, _hosts, profile) = relay();

    let mut deferred = md_host::foreign_deferred_keys(profile.lifecycle().registry());
    deferred.sort();

    assert_eq!(deferred, ["blocked", "cancelled", "failure", "finalize", "initialize", "start", "success"]);
}

// ── the passive view ────────────────────────────────────────────────────────

#[test]
fn analysis_globals_declare_the_host_roots_unavailable_and_leave_md_roots_alone() {
    let (_fixture, _hosts, profile) = relay();

    let globals = profile.analysis_globals().expect("the relay declares `seq`");

    let roots: Vec<&str> = globals.view().globals().map(|global| global.root()).collect();
    assert_eq!(roots, ["seq"], "`err` and `timing` stay md's own");
    let Availability::Unavailable(reason) = globals.view().global("seq").unwrap().availability() else {
        panic!("`seq` has no value outside the relay");
    };
    assert_eq!(reason.code(), HOST_RUNTIME_ONLY_CODE);
}

#[test]
fn a_selected_host_keeps_the_registered_layer_and_a_caller_host_opts_out() {
    let (fixture, hosts, profile) = relay();
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", "---\nsequence: []\n---\n");
    let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&fixture.path("work"), ""))
        .with_registered_hosts(Arc::clone(&hosts));
    let caller = || {
        let view = BindingView::builder(ScopeId::new("caller.host")).build().unwrap();
        HostGlobals::associate(Arc::new(view), []).unwrap()
    };

    let selected = base.clone().with_selected_host(Arc::clone(&profile));
    let replaced = selected.clone().with_host_globals(caller());
    let refused = base.with_host_globals(caller()).with_selected_host(profile);

    assert!(matches!(selected.host_selection_for(&markdown, &context).selection, HostSelection::Registered(_)));
    assert_eq!(selected.selected_host().map(|profile| profile.host()), Some("relay"));
    assert!(matches!(replaced.host_selection_for(&markdown, &context).selection, HostSelection::Md));
    assert!(replaced.selected_host().is_none(), "a caller's own globals replace the passive view");
    assert!(matches!(refused.host_selection_for(&markdown, &context).selection, HostSelection::Md));
    assert!(refused.selected_host().is_none(), "registration never replaces a caller's executable host");
}

// ── composition ─────────────────────────────────────────────────────────────

#[test]
fn a_foreign_body_composes_with_host_events_deferred_and_no_undeclared_host_reads() {
    let (fixture, hosts, _profile) = relay();
    let text = "---\nsequence:\n  - name: one\n    prompt: hi\ntitle: Relay\nstart:\n  - info: \"{{ seq.name }}\"\n---\n# {{ title }}\n";
    let (markdown, request) = foreign_request(&fixture, &hosts, text);

    let (composed, report) = markdown.compose_with_warnings(&request).expect("the body composes");

    assert_eq!(composed.content().trim(), "# Relay");
    assert!(undeclared(&report).is_empty(), "`start` is deferred, never read: {:?}", report.warnings);
}

#[test]
fn a_body_read_of_a_host_global_fails_as_unavailable() {
    let (fixture, hosts, _profile) = relay();
    let (markdown, request) = foreign_request(&fixture, &hosts, "---\nsequence: []\n---\nStep {{ seq.name }}.\n");

    let error = markdown.compose_with_warnings(&request).expect_err("no value is manufactured").error;

    let unavailable = error.unavailable_binding().unwrap_or_else(|| panic!("an unavailable read: {error}"));
    assert_eq!((unavailable.root.as_str(), unavailable.reason.code()), ("seq", HOST_RUNTIME_ONLY_CODE));
}

#[test]
fn the_host_schema_rejects_composed_and_failed() {
    let (fixture, hosts, _profile) = relay();
    let (markdown, request) =
        foreign_request(&fixture, &hosts, "---\nsequence: []\ncomposed:\n  - info: c\nfailed:\n  - info: f\n---\nBody.\n");

    let error = markdown.compose_with_warnings(&request).expect_err("rejected").error;

    let MarkdownError::SchemaValidationFailed { problems, .. } = &error else { panic!("a schema failure: {error:?}") };
    let paths: Vec<&str> = problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/composed", "/failed"], "{problems:#?}");
}

#[test]
fn the_host_schema_rejects_a_host_only_event_md_never_runs() {
    let (fixture, hosts, _profile) = relay();
    let (markdown, request) = foreign_request(&fixture, &hosts, "---\nsequence: []\nstart: 5\nsuccess: 5\n---\nBody.\n");

    let error = markdown.compose_with_warnings(&request).expect_err("rejected").error;

    let MarkdownError::SchemaValidationFailed { problems, .. } = &error else { panic!("a schema failure: {error:?}") };
    let paths: Vec<&str> = problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/start", "/success"], "{problems:#?}");
}

#[test]
fn a_host_event_template_awaiting_its_host_is_set_aside_as_validation_does() {
    let (fixture, hosts, _profile) = relay();
    let (markdown, request) = foreign_request(&fixture, &hosts, "---\nsequence: []\nstart: \"{{ seq.steps }}\"\n---\nBody.\n");

    let (composed, _report) = markdown.compose_with_warnings(&request).expect("the template is the host's to render");

    assert_eq!(composed.frontmatter().as_map()["start"], json!("{{ seq.steps }}"), "never rendered");
}

#[test]
fn a_caller_that_runs_the_events_still_owns_their_verdict() {
    let (fixture, hosts, profile) = relay();
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", "---\nsequence: []\nstart: 5\n---\nBody.\n");
    // The relay's trigger still types `start`; only the selected view is absent.
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&fixture.path("work"), ""))
        .with_source_file(fixture.path("work/doc.md"))
        .with_trigger_schemas(true)
        .with_exclude_keys(md_host::foreign_deferred_keys(profile.lifecycle().registry()))
        .with_registered_hosts(hosts);
    let request = ComposeRequest::with_context(options, context).expect("a request");

    markdown.compose_with_warnings(&request).expect("no selected host: the caller judges `start` when it runs it");
}

// ── the shared stacks, in-process ───────────────────────────────────────────

#[test]
fn a_foreign_lifecycle_runs_only_its_shared_stacks_with_md_bindings() {
    let (fixture, hosts, profile) = relay();
    let text = "---\nsequence: []\ntitle: Relay\ninitialize:\n  - say: hello\n  - info: \"initialize {{ title }}\"\nstart:\n  - info: start\n  - notify: start\nsuccess:\n  - effect: bong\nfinalize:\n  - info: finalize\n---\nBody.\n";
    let (markdown, request) = foreign_request(&fixture, &hosts, text);
    let projection = md_host::execution_projection(&Value::Object(frontmatter_of(&markdown)), profile.lifecycle().registry());
    let config = md_host::parse_lifecycle(&projection, &fixture.path("work/doc.md"), Some(markdown.frontmatter())).unwrap();

    let recorded = record_run(&request, &config, &fixture.path("work/doc.md"), frontmatter_of(&markdown));

    assert_eq!(recorded.end.finish, Finish::Succeeded, "{recorded:?}");
    assert_eq!(recorded.events(), ["initialize", "composed", "finalize"], "{recorded:?}");
    assert_eq!(
        recorded.log,
        [
            "event initialize",
            "warn `md` does not run `say`; skipped it in `initialize`",
            "info initialize Relay",
            "prepare",
            "event composed",
            "write",
            "event finalize",
            "info finalize",
        ],
        "no speech, notification, sound, or host stack: {recorded:?}"
    );
}

#[test]
fn a_shared_stack_reading_a_host_global_fails_as_unavailable() {
    let (fixture, hosts, profile) = relay();
    let text = "---\nsequence: []\ninitialize:\n  - info: \"{{ seq.name }}\"\n---\nBody.\n";
    let (markdown, request) = foreign_request(&fixture, &hosts, text);
    let projection = md_host::execution_projection(&Value::Object(frontmatter_of(&markdown)), profile.lifecycle().registry());
    let config = md_host::parse_lifecycle(&projection, &fixture.path("work/doc.md"), Some(markdown.frontmatter())).unwrap();
    let control = request.clone().map_options(|options| {
        options.with_host_globals(HostGlobals::associate(Arc::new(BindingView::builder(ScopeId::new("md.none")).build().unwrap()), []).unwrap())
    });

    let recorded = record_run(&request, &config, &fixture.path("work/doc.md"), frontmatter_of(&markdown));
    let without_host = record_run(&control, &config, &fixture.path("work/doc.md"), frontmatter_of(&markdown));

    assert_eq!(recorded.end.finish, Finish::Failed, "{recorded:?}");
    let message = recorded.end.error.as_ref().map(|error| error.msg.clone()).unwrap_or_default();
    assert!(message.contains("reads the global `seq`, which is unavailable") && message.contains(HOST_RUNTIME_ONLY_CODE), "{message}");
    assert!(!recorded.log.iter().any(|line| line == "prepare" || line == "write"), "nothing composes: {recorded:?}");
    assert_eq!(without_host.end.finish, Finish::Succeeded, "without the view `seq` is an absent property: {without_host:?}");
}
