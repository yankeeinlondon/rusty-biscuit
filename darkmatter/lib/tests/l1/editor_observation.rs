//! Editor observation of whole-value frontmatter properties
//! (`schemas::slots::observe`): what the editor evaluates, and that its
//! reads are authorized before anything is probed or read.
//!
//! Every context is anchored in a temporary folder whose home pre-approves
//! no read, so only a test's own approvals or `permissions` allow one.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::DataRuntime;
use darkmatter::markdown::compose::expression::data::ObservationKind;
use darkmatter::markdown::compose::expression::iteration::IterationLimits;
use darkmatter::markdown::schemas::slots::SlotSession;
use darkmatter::markdown::schemas::slots::grants::{Grant, GrantDecision, GrantScope, GrantTarget};
use darkmatter::markdown::schemas::slots::observe::{
    ObserveRequest, PendingReason, PropertyObservations, PropertyOutcome, PropertySite, observes_property,
};
use serde_json::{Value, json};

const DATA: &str = "- name: Ada\n  age: 36\n- name: Grace\n  age: 85\n";

/// A canonical temporary folder holding a document whose home's
/// `permissions` are `file_read` (YAML text for the list's items).
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    context: FileResolutionContext,
}

impl Fixture {
    fn new() -> Self {
        Self::with_permissions("[]")
    }

    fn with_permissions(file_read: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let home = root.join(".home");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(".darkmatter.yaml"), format!("permissions:\n    file-read: {file_read}\n")).unwrap();
        let context = FileResolutionContext::from_snapshot(&root, Some(home), std::collections::HashMap::new());
        Self { _dir: dir, root, context }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    fn allow(&self, name: &str) -> Grant {
        Grant { decision: GrantDecision::Allow, target: GrantTarget::File(self.root.join(name)), scope: GrantScope::Once }
    }

    fn session(&self, grants: Vec<Grant>, data: &DataRuntime) -> Arc<SlotSession> {
        Arc::new(SlotSession::for_context(data.clone(), &self.context).with_run_grants(grants))
    }

    /// Observes `sites` (key, expression, eligible) over `seed`.
    fn observe(&self, session: &Arc<SlotSession>, seed: Value, sites: &[(&str, &str, bool)]) -> PropertyObservations {
        let Value::Object(seed) = seed else { panic!("a seed is an object") };
        session.observe_properties(&ObserveRequest {
            document: self.root.join("doc.md"),
            context: self.context.clone(),
            seed,
            sites: sites
                .iter()
                .map(|(key, expression, eligible)| PropertySite {
                    key: (*key).to_string(),
                    expression: (*expression).to_string(),
                    eligible: *eligible,
                })
                .collect(),
            cache_root: Some(self.root.clone()),
            now: SystemTime::now(),
        })
    }

    /// Observes one eligible property `key` = `{{ expression }}` with
    /// `grants`.
    fn one(&self, grants: Vec<Grant>, expression: &str) -> PropertyObservations {
        let data = DataRuntime::new();
        let session = self.session(grants, &data);
        self.observe(&session, json!({ "p": format!("{{{{ {expression} }}}}") }), &[("p", expression, true)])
    }
}

fn observed(observations: &PropertyObservations, key: &str) -> Value {
    match observations.get(key) {
        Some(PropertyOutcome::Observed { value, .. }) => value.clone(),
        other => panic!("{key} was not observed: {other:?}"),
    }
}

fn pending(observations: &PropertyObservations, key: &str) -> PendingReason {
    match observations.get(key) {
        Some(PropertyOutcome::Pending(reason)) => reason.clone(),
        other => panic!("{key} is not pending: {other:?}"),
    }
}

/// Whether any observation names `path`.
fn names(observations: &PropertyObservations, path: &Path) -> bool {
    let entry = biscuit_file::to_portable_string(path);
    observations.observations.iter().any(|observation| observation.entry == entry)
}

fn parse(source: &str) -> simplified_schema::expression::SpannedExpr {
    simplified_schema::expression::parse_spanned(source).unwrap()
}

/// The specification's worked example: each property is observed, a direct
/// load and a selected arm keep their data source, and the derived row does
/// not.
#[test]
fn the_worked_example_is_observed_with_an_approval() {
    let fixture = Fixture::new();
    fixture.write("data.yaml", DATA);
    let data = DataRuntime::new();
    let session = fixture.session(vec![fixture.allow("data.yaml")], &data);
    let sites = [
        ("direct", "load_data('data.yaml')", true),
        ("additive", "map(load_data('data.yaml'), i => { ...i, color: 'blue' })", true),
        ("conditional", "file_exists('data.yaml') ? load_data('data.yaml') : 'foobar'", true),
    ];
    let seed = json!({
        "direct": "{{ load_data('data.yaml') }}",
        "additive": "{{ map(load_data('data.yaml'), i => { ...i, color: 'blue' }) }}",
        "conditional": "{{ file_exists('data.yaml') ? load_data('data.yaml') : 'foobar' }}",
    });
    let observations = fixture.observe(&session, seed, &sites);
    let rows = json!([{ "name": "Ada", "age": 36 }, { "name": "Grace", "age": 85 }]);
    assert_eq!(observed(&observations, "direct"), rows);
    assert_eq!(
        observed(&observations, "additive"),
        json!([{ "name": "Ada", "age": 36, "color": "blue" }, { "name": "Grace", "age": 85, "color": "blue" }])
    );
    assert_eq!(observed(&observations, "conditional"), rows);
    let load = |key| match observations.get(key) {
        Some(PropertyOutcome::Observed { load, .. }) => load.is_some(),
        _ => false,
    };
    assert!(load("direct") && load("conditional") && !load("additive"));
    // Three properties, one read and one parse of the shared file.
    assert_eq!(data.stats().parses, 1, "{:?}", data.stats());
    assert_eq!(data.stats().reads, 1, "{:?}", data.stats());
    assert!(observations.observations.iter().any(|observation| observation.kind == ObservationKind::File));
    assert!(!observations.exhausted && !observations.canceled);
    assert_eq!(observations.remote_expiry, None);
}

/// Without the file, the conditional takes its fallback and the direct load
/// is a proven miss whose candidate is observed, so creating it refreshes.
#[test]
fn an_approved_missing_file_is_missing_and_its_candidate_is_watched() {
    let fixture = Fixture::new();
    let data = DataRuntime::new();
    let session = fixture.session(vec![fixture.allow("data.yaml")], &data);
    let seed = json!({ "direct": "{{ … }}", "conditional": "{{ … }}" });
    let sites = [
        ("direct", "load_data('data.yaml')", true),
        ("conditional", "file_exists('data.yaml') ? load_data('data.yaml') : 'foobar'", true),
    ];
    let observations = fixture.observe(&session, seed, &sites);
    assert!(matches!(observations.get("direct"), Some(PropertyOutcome::Missing(_))), "{:?}", observations.get("direct"));
    assert_eq!(observed(&observations, "conditional"), json!("foobar"));
    let missing = biscuit_file::to_portable_string(&fixture.root.join("data.yaml"));
    assert!(
        observations
            .observations
            .iter()
            .any(|observation| observation.entry == missing && observation.kind == ObservationKind::MissingProbe),
        "{:?}",
        observations.observations
    );
}

/// A read nothing approves is pending, and nothing about the file is
/// observed: an existing and an absent file give the same answer.
#[test]
fn an_unapproved_read_is_pending_and_probes_nothing() {
    let fixture = Fixture::new();
    let secret = fixture.write("secret.yaml", DATA);
    for name in ["secret.yaml", "absent.yaml"] {
        let observations = fixture.one(Vec::new(), &format!("load_data('{name}')"));
        assert_eq!(pending(&observations, "p"), PendingReason::ReadRefused, "{name}");
        assert!(observations.observations.is_empty(), "{name}: {:?}", observations.observations);
        let observations = fixture.one(Vec::new(), &format!("file_exists('{name}')"));
        assert_eq!(pending(&observations, "p"), PendingReason::ReadRefused, "{name}");
        assert!(observations.observations.is_empty(), "{name}: {:?}", observations.observations);
    }
    assert!(secret.is_file());
}

/// A lexical exclusion refuses before any probe, and a link whose target is
/// excluded is refused by the physical check even though its own name is
/// allowed.
#[test]
fn lexical_and_physical_exclusions_refuse_before_reading() {
    let fixture = Fixture::with_permissions("[\"./*.yaml\", \"!./secret.yaml\"]");
    fixture.write("secret.yaml", DATA);
    fixture.write("open.yaml", DATA);
    let observations = fixture.one(Vec::new(), "load_data('secret.yaml')");
    assert_eq!(pending(&observations, "p"), PendingReason::ReadRefused);
    assert!(!names(&observations, &fixture.root.join("secret.yaml")), "{:?}", observations.observations);
    // Control: the configuration allows a sibling.
    let observations = fixture.one(Vec::new(), "load_data('open.yaml')");
    assert_eq!(observed(&observations, "p")[0]["name"], json!("Ada"));
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(fixture.root.join("secret.yaml"), fixture.root.join("link.yaml")).unwrap();
        let observations = fixture.one(Vec::new(), "load_data('link.yaml')");
        assert_eq!(pending(&observations, "p"), PendingReason::ReadRefused);
        assert!(!observations.observations.iter().any(|observation| observation.kind == ObservationKind::File));
    }
}

/// A missing file, an unreadable one, and an unauthorized one are three
/// different outcomes.
#[test]
fn missing_unreadable_and_unauthorized_reads_differ() {
    let fixture = Fixture::new();
    let missing = fixture.one(vec![fixture.allow("gone.yaml")], "load_data('gone.yaml')");
    assert!(matches!(missing.get("p"), Some(PropertyOutcome::Missing(_))));
    let refused = fixture.one(Vec::new(), "load_data('gone.yaml')");
    assert_eq!(pending(&refused, "p"), PendingReason::ReadRefused);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let locked = fixture.write("locked.yaml", DATA);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        // A superuser reads through the mode, so the case proves nothing there.
        if std::fs::read(&locked).is_err() {
            let unreadable = fixture.one(vec![fixture.allow("locked.yaml")], "load_data('locked.yaml')");
            assert_eq!(pending(&unreadable, "p"), PendingReason::ReadUnavailable);
        }
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}

/// `try` recovers a proven miss, never a refusal; an unselected branch's
/// miss is never evaluated.
#[test]
fn recovery_turns_only_a_proven_miss_into_the_fallback() {
    let fixture = Fixture::new();
    let recovered = fixture.one(vec![fixture.allow("gone.yaml")], "try(load_data('gone.yaml'), 'fallback')");
    assert_eq!(observed(&recovered, "p"), json!("fallback"));
    let refused = fixture.one(Vec::new(), "try(load_data('gone.yaml'), 'fallback')");
    assert_eq!(pending(&refused, "p"), PendingReason::ReadRefused);
    let unselected = fixture.one(Vec::new(), "false ? load_data('gone.yaml') : 'other'");
    assert_eq!(observed(&unselected, "p"), json!("other"));
    assert!(unselected.observations.is_empty());
}

/// An effectful function, a runtime-only binding, and a name the document
/// does not hold leave the property pending, never `null`.
#[test]
fn unsupported_operations_and_runtime_bindings_are_pending() {
    let fixture = Fixture::new();
    let reason = pending(&fixture.one(Vec::new(), "as_markdown('x.md') || load_data('a.yaml')"), "p");
    assert!(matches!(reason, PendingReason::Unsupported(_)), "{reason:?}");
    let reason = pending(&fixture.one(Vec::new(), "find_files('*.yaml') || load_data('a.yaml')"), "p");
    assert!(matches!(reason, PendingReason::Unsupported(_)), "{reason:?}");
    let reason = pending(&fixture.one(Vec::new(), "load_data(ctx.repo_root)"), "p");
    assert!(matches!(reason, PendingReason::RuntimeOnly(_)), "{reason:?}");
    let reason = pending(&fixture.one(Vec::new(), "load_data(host_value)"), "p");
    assert_eq!(reason, PendingReason::RuntimeOnly("host_value".to_string()));
}

/// A bare name reads a literal property or another whole-value property
/// evaluated first; `fm.NAME` of a template, and properties that read each
/// other, are pending.
#[test]
fn properties_read_literals_and_other_properties_dependency_first() {
    let fixture = Fixture::new();
    fixture.write("data.yaml", DATA);
    let data = DataRuntime::new();
    let session = fixture.session(vec![fixture.allow("data.yaml")], &data);
    let seed = json!({
        "file": "{{ 'data' + '.yaml' }}",
        "rows": "{{ load_data(file) }}",
        "literal": "data.yaml",
        "plain": "{{ load_data(literal) }}",
        "authored": "{{ load_data(fm.file) }}",
        "a": "{{ [b, load_data('data.yaml')] }}",
        "b": "{{ a }}",
    });
    let sites = [
        ("file", "'data' + '.yaml'", false),
        ("rows", "load_data(file)", true),
        ("plain", "load_data(literal)", true),
        ("authored", "load_data(fm.file)", true),
        ("a", "[b, load_data('data.yaml')]", true),
        ("b", "a", false),
    ];
    let observations = fixture.observe(&session, seed, &sites);
    assert_eq!(observed(&observations, "file"), json!("data.yaml"));
    assert_eq!(observed(&observations, "rows")[1]["name"], json!("Grace"));
    assert_eq!(observed(&observations, "plain")[0]["age"], json!(36));
    assert_eq!(pending(&observations, "authored"), PendingReason::Dependency("file".to_string()));
    assert_eq!(pending(&observations, "a"), PendingReason::Cycle);
    let keys: Vec<&str> = observations.properties.iter().map(|property| property.key.as_str()).collect();
    assert_eq!(keys, ["file", "rows", "plain", "authored", "a", "b"], "document order");
}

/// Exhausting the shared allowance stops the pass without asking anyone:
/// the property that ran out and every later one are pending.
#[test]
fn an_exhausted_allowance_stops_the_pass() {
    let fixture = Fixture::new();
    fixture.write("data.yaml", DATA);
    let data = DataRuntime::new();
    let limits = IterationLimits::new(std::num::NonZeroU64::new(3).unwrap(), std::num::NonZeroU64::new(4 * 1024 * 1024).unwrap());
    let session = Arc::new(
        SlotSession::for_context(data.clone(), &fixture.context)
            .with_run_grants(vec![fixture.allow("data.yaml")])
            .with_slot_limits(limits),
    );
    let seed = json!({ "big": "{{ … }}", "later": "{{ … }}" });
    let sites = [
        ("big", "map(load_data('data.yaml'), i => map([1, 2, 3], j => j))", true),
        ("later", "load_data('data.yaml')", true),
    ];
    let observations = fixture.observe(&session, seed, &sites);
    assert_eq!(pending(&observations, "big"), PendingReason::Exhausted);
    assert_eq!(pending(&observations, "later"), PendingReason::Exhausted);
    assert!(observations.exhausted);
}

/// A source larger than the remaining byte allowance is read only one byte
/// past it, and stops the pass.
#[test]
fn a_source_past_the_byte_allowance_stops_the_pass() {
    let fixture = Fixture::new();
    fixture.write("data.yaml", DATA);
    let data = DataRuntime::new();
    let limits = IterationLimits::new(std::num::NonZeroU64::new(25_000).unwrap(), std::num::NonZeroU64::new(16).unwrap());
    let session = Arc::new(
        SlotSession::for_context(data.clone(), &fixture.context)
            .with_run_grants(vec![fixture.allow("data.yaml")])
            .with_slot_limits(limits),
    );
    let observations = fixture.observe(&session, json!({ "p": "{{ … }}" }), &[("p", "load_data('data.yaml')", true)]);
    assert_eq!(pending(&observations, "p"), PendingReason::Exhausted);
    assert!(observations.exhausted);
    assert_eq!(data.stats().parses, 0, "nothing past the allowance is parsed");
}

/// Which whole-value expressions the editor observes: a `load_data` call
/// whose contract is missing, `null`, or not a literal.
#[test]
fn observes_property_follows_the_contract_argument() {
    assert!(observes_property(&parse("load_data('a.yaml')")));
    assert!(observes_property(&parse("load_data('a.yaml', null)")));
    assert!(observes_property(&parse("load_data('a.yaml', null, { header: false })")));
    assert!(observes_property(&parse("load_data('a.yaml', fm.contract)")));
    assert!(observes_property(&parse("map(load_data('a.yaml'), i => { ...i, c: 1 })")));
    assert!(observes_property(&parse("{ ...first(load_data('a.yaml')), c: 1 }")));
    assert!(observes_property(&parse("map(x, i => load_data(i))")));
    assert!(observes_property(&parse("loadData('a.yaml')")), "an alias is the same function");
    assert!(!observes_property(&parse("load_data('a.yaml', 'string[]')")));
    assert!(!observes_property(&parse("load_data('a.yaml', ('string[]'))")));
    assert!(!observes_property(&parse("file_exists('a.yaml')")));
    assert!(!observes_property(&parse("'text'")));
}

/// Two properties loading one file through the observation's one runtime
/// parse it once.
#[test]
fn properties_share_one_parse() {
    let fixture = Fixture::new();
    fixture.write("data.yaml", DATA);
    let data = DataRuntime::new();
    let session = fixture.session(vec![fixture.allow("data.yaml")], &data);
    let observations = fixture.observe(
        &session,
        json!({ "a": "{{ … }}", "b": "{{ … }}" }),
        &[("a", "load_data('data.yaml')", true), ("b", "first(load_data('data.yaml'))", true)],
    );
    assert_eq!(observed(&observations, "b")["name"], json!("Ada"));
    assert_eq!(data.stats().parses, 1);
}

/// Each evaluated `load_data` occurrence keeps its distinct returned values
/// at its authored span: once for a call outside a callback, once per item
/// (deduplicated) for a call in a callback body, overflowing past eight.
#[test]
fn each_load_occurrence_keeps_what_it_returned() {
    let fixture = Fixture::new();
    fixture.write("data.yaml", DATA);
    let names: String = (0..10).map(|index| format!("- n{index}\n")).collect();
    fixture.write("names.yaml", &names);
    for index in 0..10 {
        fixture.write(&format!("n{index}.yaml"), &format!("- {index}\n"));
    }
    let mut grants = vec![fixture.allow("data.yaml"), fixture.allow("names.yaml")];
    grants.extend((0..10).map(|index| fixture.allow(&format!("n{index}.yaml"))));

    let expression = "map(load_data('data.yaml'), i => { ...i, color: 'blue' })";
    let observations = fixture.one(grants.clone(), expression);
    let property = observations.properties.iter().find(|property| property.key == "p").unwrap();
    let [call] = property.calls.as_slice() else { panic!("one call: {:?}", property.calls) };
    assert_eq!(&expression[call.span.start..call.span.end], "load_data('data.yaml')");
    assert_eq!(call.values, [json!([{ "name": "Ada", "age": 36 }, { "name": "Grace", "age": 85 }])]);
    assert!(!call.overflow);
    assert_eq!(call.call, parse("load_data('data.yaml')").erase());

    // A call in a callback body is one occurrence with a value per item:
    // ten distinct values keep eight and overflow.
    let expression = "map(load_data('names.yaml'), n => load_data(n + '.yaml'))";
    let observations = fixture.one(grants, expression);
    let property = observations.properties.iter().find(|property| property.key == "p").unwrap();
    assert_eq!(property.calls.len(), 2, "{:?}", property.calls);
    let inner = &property.calls[1];
    assert_eq!(&expression[inner.span.start..inner.span.end], "load_data(n + '.yaml')");
    assert_eq!(inner.values.len(), 8);
    assert!(inner.overflow);
    assert!(!property.calls[0].overflow);
}
