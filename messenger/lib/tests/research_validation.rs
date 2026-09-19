//! Targeted behavior of `messenger::research` (feature `research`): the load
//! stage, eligibility, coverage summaries, overrides, and assessment reuse.
//!
//! Corpus-wide rule coverage lives in `research_corpus.rs`; these tests pin
//! one behavior each, usually by writing a variant of a real fixture into a
//! temporary workspace that carries copies of the shipped schemas.
#![cfg(feature = "research")]

use std::fs;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use messenger::research::assess::{AssessmentState, evaluate};
use messenger::research::canonical::{record_fingerprint, schema_fingerprint, text_fingerprint};
use messenger::research::model::{
    Date, ImplementationStatus, MAX_REFRESH_INTERVAL_DAYS, Mappings, PlatformDocument, PlatformId, Roster, Stage,
    StaleReason, Unit,
};
use messenger::research::project::{AcceptedDocument, CatalogInputs, project};
use messenger::research::publish::SchemaEntry;
use messenger::research::validate::{ConstraintEligibility, CoverageState, IneligibleReason};
use messenger::research::{
    Context, Diagnostic, Loaded, Loader, ResearchError, Rule, Scope, ValidatedDocument, Workspace,
    coverage_summary, validate_document, validate_fleet, validate_overrides, validate_roster,
};
use serde_json::Value;
use tempfile::TempDir;

fn lib_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn repo_root() -> PathBuf {
    lib_dir().parent().and_then(Path::parent).expect("repository root").to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    lib_dir().join("tests/fixtures/research").join(name)
}

fn repo_loader() -> Loader {
    Loader::new(Workspace::new(repo_root()).expect("absolute root"))
}

fn shipped_roster() -> Roster {
    let loaded = repo_loader()
        .load_roster(&repo_root().join("messenger/docs/platforms.yaml"))
        .expect("roster");
    loaded.record.expect("typed roster")
}

fn today() -> Date {
    Date::parse("2026-09-17").expect("date")
}

fn rules(diagnostics: &[Diagnostic]) -> Vec<Rule> {
    diagnostics.iter().map(|d| d.rule).collect()
}

fn show(diagnostics: &[Diagnostic]) -> String {
    diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n  ")
}

fn fragment(roster: &Roster) -> Context<'_> {
    Context { roster: Some(roster), scope: Scope::Fragment }
}

fn validated(loader: &Loader, path: &Path, roster: &Roster) -> ValidatedDocument {
    let result = validate_document(&loader.load_document(path).expect("load"), &fragment(roster));
    assert!(result.diagnostics.is_empty(), "{}:\n  {}", path.display(), show(&result.diagnostics));
    result.validated.expect("validated")
}

/// A throwaway repository holding copies of the shipped schemas at their
/// real layout, so variants bind the same contract as the real files.
struct TempWorkspace {
    dir: TempDir,
    loader: Loader,
}

const SCHEMAS: &[&str] = &[
    "messenger/docs/platforms.schema.yaml",
    "messenger/docs/research/platforms/_schema.yaml",
    "messenger/docs/research/platforms/_types.yaml",
    "messenger/docs/research/platforms/_overrides.schema.yaml",
    "messenger/docs/research/implementation/_schema.yaml",
];

impl TempWorkspace {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        for schema in SCHEMAS {
            let target = dir.path().join(schema);
            fs::create_dir_all(target.parent().expect("parent")).expect("mkdir");
            fs::copy(repo_root().join(schema), target).expect("copy schema");
        }
        let loader = Loader::new(Workspace::new(dir.path()).expect("absolute tempdir"));
        Self { dir, loader }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, content).expect("write");
        path
    }

    /// Writes a platform document variant of a fixture: `$schema` is rebound
    /// to the temporary contract and each `(from, to)` edit must apply.
    fn document(&self, name: &str, source: &str, edits: &[(&str, &str)]) -> PathBuf {
        let text = rebind(&fs::read_to_string(fixture(source)).expect("fixture"), "./_schema.yaml");
        self.write(&format!("messenger/docs/research/platforms/{name}"), &apply(text, edits))
    }
}

fn rebind(text: &str, schema: &str) -> String {
    text.lines()
        .map(|line| if line.starts_with("$schema:") { format!("$schema: {schema}") } else { line.to_string() })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn apply(mut text: String, edits: &[(&str, &str)]) -> String {
    for (from, to) in edits {
        assert!(text.contains(from), "edit target not found: {from}");
        text = text.replacen(from, to, 1);
    }
    text
}

// ---- load stage --------------------------------------------------------------

#[test]
fn a_quoted_schema_version_is_unsupported_even_though_the_schema_coerces_it() {
    let workspace = TempWorkspace::new();
    let path = workspace.document("quoted.md", "contract/minimal-valid.md", &[("schema_version: 1", "schema_version: '1'")]);
    let loaded = workspace.loader.load_document(&path).expect("load");
    assert_eq!(rules(&loaded.diagnostics), vec![Rule::Version], "{}", show(&loaded.diagnostics));
    assert_eq!(loaded.diagnostics[0].pointer, "/schema_version");
    assert!(loaded.record.is_none(), "an unsupported version is never parsed best-effort");
    assert_eq!(loaded.frontmatter["platform_id"], "discord", "the file stays inspectable");
}

#[test]
fn strict_scalars_reject_authored_type_mismatches_at_their_pointer() {
    let workspace = TempWorkspace::new();
    // A quoted number, a numeric condition operand, and a float version.
    let cases = [
        ("contract/constraints-field.md", ("  value: 2000", "  value: '2000'"), "/constraints/0/value"),
        ("contract/constraints-bridge-conditional.md", ("    min: 0.12.0", "    min: 12"), "/constraints/0/applies_when/0/min"),
        ("contract/versions-stable-preview.md", ("latest_stable: v10", "latest_stable: 10.0"), "/api_versions/0/latest_stable"),
    ];
    for (source, edit, pointer) in cases {
        let text = fs::read_to_string(fixture(source)).expect("fixture");
        if !text.contains(edit.0) {
            panic!("{source} no longer contains {:?}", edit.0);
        }
        let path = workspace.document("scalars.md", source, &[edit]);
        let loaded = workspace.loader.load_document(&path).expect("load");
        assert_eq!(rules(&loaded.diagnostics), vec![Rule::StrictScalars], "{source}: {}", show(&loaded.diagnostics));
        assert!(loaded.diagnostics[0].pointer.starts_with(pointer), "{source}: {}", loaded.diagnostics[0].pointer);
    }
}

#[test]
fn an_unknown_top_level_key_is_rejected_by_rule() {
    let workspace = TempWorkspace::new();
    let path = workspace.document("extra.md", "contract/minimal-valid.md", &[("agent: fixture", "agent: fixture\nnotes: stray")]);
    let loaded = workspace.loader.load_document(&path).expect("load");
    assert!(loaded.diagnostics.iter().any(|d| d.rule == Rule::TopLevel && d.pointer == "/notes"), "{}", show(&loaded.diagnostics));
    assert!(loaded.record.is_none());
}

#[test]
fn the_schema_must_be_the_contract_for_the_file_kind() {
    let workspace = TempWorkspace::new();
    let unbound = workspace.document("unbound.md", "contract/minimal-valid.md", &[("$schema: ./_schema.yaml\n", "")]);
    let wrong = workspace.document("wrong.md", "contract/minimal-valid.md", &[("$schema: ./_schema.yaml", "$schema: ./_overrides.schema.yaml")]);
    for path in [unbound, wrong] {
        let loaded = workspace.loader.load_document(&path).expect("load");
        assert_eq!(rules(&loaded.diagnostics), vec![Rule::SchemaBinding], "{}", show(&loaded.diagnostics));
        assert!(loaded.record.is_none());
    }
}

/// The shipped platform documents are unmigrated legacy prose until the
/// Phase 7 baseline; the real loader reports that instead of skipping them.
#[test]
fn shipped_legacy_documents_are_reported_as_unbound() {
    let loader = repo_loader();
    for platform in PlatformId::ALL {
        let path = repo_root().join(format!("messenger/docs/research/platforms/{platform}.md"));
        let loaded = loader.load_document(&path).expect("load");
        if loaded.frontmatter.get("schema_version").is_some() {
            continue;
        }
        assert_eq!(rules(&loaded.diagnostics), vec![Rule::SchemaBinding], "{platform}");
        assert_eq!(loaded.path.as_str(), format!("messenger/docs/research/platforms/{platform}.md"));
    }
}

#[test]
fn unreadable_files_are_errors_with_repository_relative_paths() {
    let workspace = TempWorkspace::new();
    let missing = workspace.root().join("messenger/docs/research/platforms/absent.md");
    match workspace.loader.load_document(&missing) {
        Err(ResearchError::Io { path, .. }) => assert_eq!(path.as_str(), "messenger/docs/research/platforms/absent.md"),
        other => panic!("expected an I/O error, got {other:?}"),
    }

    let broken = workspace.write("messenger/docs/research/platforms/broken.md", "---\nschema_version: [1\n---\n");
    match workspace.loader.load_document(&broken) {
        Err(ResearchError::Frontmatter { path, message }) => {
            assert_eq!(path.as_str(), "messenger/docs/research/platforms/broken.md");
            let root = workspace.root().display().to_string();
            assert!(!message.contains(&root), "host path leaked: {message}");
        }
        other => panic!("expected a frontmatter error, got {other:?}"),
    }

    let outside = tempfile::tempdir().expect("tempdir");
    let foreign = outside.path().join("x.md");
    fs::write(&foreign, "---\n---\n").expect("write");
    assert!(matches!(
        workspace.loader.load_document(&foreign),
        Err(ResearchError::OutsideWorkspace { .. })
    ));
    assert!(matches!(Workspace::new("relative/root"), Err(ResearchError::RelativeRoot { .. })));
}

/// Every positive document survives typed load → serialize → schema
/// validation → deserialize unchanged: the Rust vocabulary is the schema's.
/// All positive documents bind the same shipped schema, so it resolves once.
fn assert_round_trips(dir: &str, min: usize) {
    fn strip_nulls(value: &mut Value) {
        match value {
            Value::Object(map) => {
                map.retain(|_, v| !v.is_null());
                map.values_mut().for_each(strip_nulls);
            }
            Value::Array(items) => items.iter_mut().for_each(strip_nulls),
            _ => {}
        }
    }
    let loader = repo_loader();
    let mut files: Vec<PathBuf> = fs::read_dir(fixture(dir))
        .expect("read dir")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    files.sort();
    assert!(files.len() >= min, "{dir} shrank to {}", files.len());
    let markdown = Markdown::try_from(files[0].as_path()).expect("markdown");
    let effective = DarkmatterSchemas::new()
        .effective_for(&markdown)
        .expect("schema")
        .expect("bound schema");
    for path in files {
        let loaded: Loaded<PlatformDocument> = loader.load_document(&path).expect("load");
        let record = loaded.record.expect("typed record");
        let mut first = serde_json::to_value(&record).expect("serialize");
        let second = serde_json::to_value(&record).expect("serialize again");
        assert_eq!(first, second, "serialization is deterministic");
        strip_nulls(&mut first);
        let report = effective.validate(&first);
        assert!(report.valid, "{}: serialized DTO fails its schema: {:?}", path.display(), report.problems);
        let reread: PlatformDocument = serde_json::from_value(first).expect("deserialize");
        assert_eq!(reread, record, "{}", path.display());
    }
}

#[test]
fn contract_documents_round_trip_through_the_schema() {
    assert_round_trips("contract", 38);
}

#[test]
fn interaction_documents_round_trip_through_the_schema() {
    assert_round_trips("interaction", 12);
}

#[test]
fn diagnostic_documents_round_trip_through_the_schema() {
    assert_round_trips("diagnostics", 6);
}

// ---- eligibility ---------------------------------------------------------------

fn eligibility(name: &str) -> Vec<ConstraintEligibility> {
    validated(&repo_loader(), &fixture(name), &shipped_roster()).eligibility()
}

/// Criterion 5: each counting unit and stage reaches the projection exactly
/// as researched; nothing collapses to "characters".
#[test]
fn executable_constraints_keep_their_researched_unit_and_stage() {
    let cases = [
        ("contract/units-utf8-bytes.md", Unit::Utf8Bytes, Stage::FieldValue),
        ("contract/units-unicode-scalars.md", Unit::UnicodeScalars, Stage::FieldValue),
        ("contract/units-utf16.md", Unit::Utf16CodeUnits, Stage::FieldValue),
        ("contract/units-graphemes.md", Unit::GraphemeClusters, Stage::FieldValue),
        ("contract/units-parsed-text.md", Unit::UnicodeScalars, Stage::ParsedText),
        ("contract/units-serialized.md", Unit::Utf8Bytes, Stage::SerializedPayload),
    ];
    for (name, unit, stage) in cases {
        let entries = eligibility(name);
        assert_eq!(entries.len(), 1, "{name}");
        let executable = entries[0].executable().unwrap_or_else(|| panic!("{name}: {:?}", entries[0].reasons()));
        assert_eq!((executable.unit, executable.stage), (unit, stage), "{name}");
    }
}

#[test]
fn ambiguous_unknown_and_advisory_constraints_never_become_executable() {
    let reasons = |name: &str, id: &str| -> Vec<IneligibleReason> {
        let entries = eligibility(name);
        let entry = entries.iter().find(|e| e.id() == id).unwrap_or_else(|| panic!("{name}: no {id}"));
        assert!(entry.executable().is_none(), "{name}: {id} is executable");
        entry.reasons().to_vec()
    };
    assert!(reasons("contract/units-ambiguous.md", "c.fx.unspecified")
        .contains(&IneligibleReason::UnresolvedUnit { unit: Unit::UnspecifiedCharacters }));
    assert_eq!(reasons("contract/units-ambiguous.md", "c.fx.unknown_stage"), vec![IneligibleReason::UnknownStage]);
    assert!(reasons("contract/constraints-recommended.md", "c.fx.text.recommended").contains(&IneligibleReason::Advisory));
    for (id, state) in [("c.fx.unknown", "unknown"), ("c.fx.conflict", "conflicting"), ("c.fx.na", "not_applicable")] {
        let found = reasons("contract/knowledge-states.md", id);
        assert!(found.iter().any(|r| r.code() == "not_known"), "{id} ({state}): {found:?}");
    }
    let signal = eligibility("contract/pilot-signal.md");
    assert!(signal.iter().any(|e| e
        .reasons()
        .iter()
        .any(|r| matches!(r, IneligibleReason::UnresolvedCondition { .. }))));
}

/// A conditional bound is executable only with a resolved operand; the
/// same record with its operand replaced by a gap is not.
#[test]
fn an_unresolved_condition_makes_an_otherwise_known_bound_ineligible() {
    let workspace = TempWorkspace::new();
    let roster = shipped_roster();
    let resolved = workspace.document("resolved.md", "contract/constraints-bridge-conditional.md", &[]);
    let resolved = validated(&workspace.loader, &resolved, &roster);
    assert!(resolved.eligibility().iter().any(|e| e.executable().is_some_and(|c| !c.conditions.is_empty())));

    let gapped = workspace.document(
        "gapped.md",
        "contract/constraints-bridge-conditional.md",
        &[("    min: 0.12.0", "    gap: gap.fx.scope")],
    );
    let gapped = validated(&workspace.loader, &gapped, &roster);
    assert!(gapped.eligibility().iter().any(|e| e
        .reasons()
        .iter()
        .any(|r| matches!(r, IneligibleReason::UnresolvedCondition { .. }))));
}

// ---- applicability ------------------------------------------------------------

#[test]
fn only_conditions_that_can_hold_together_make_two_values_ambiguous() {
    let workspace = TempWorkspace::new();
    let roster = shipped_roster();
    let validate = |path: &Path| validate_document(&workspace.loader.load_document(path).expect("load"), &fragment(&roster));

    let ambiguous = workspace.document("a.md", "negative/semantic/sr-applicability--ambiguous-applicability.md", &[]);
    assert_eq!(rules(&validate(&ambiguous).diagnostics), vec![Rule::Applicability]);

    // Same condition kind with different values: mutually exclusive.
    let exclusive = workspace.document(
        "b.md",
        "negative/semantic/sr-applicability--ambiguous-applicability.md",
        &[("  - kind: account_tier\n    equals: premium", "  - kind: hosting_mode\n    equals: local")],
    );
    let result = validate(&exclusive);
    assert!(result.diagnostics.is_empty(), "{}", show(&result.diagnostics));

    // Identical conditions with different values are a duplicate bound.
    let duplicate = workspace.document(
        "c.md",
        "negative/semantic/sr-applicability--ambiguous-applicability.md",
        &[("  - kind: account_tier\n    equals: premium", "  - kind: hosting_mode\n    equals: cloud")],
    );
    assert_eq!(rules(&validate(&duplicate).diagnostics), vec![Rule::Unique]);
}

// ---- coverage ---------------------------------------------------------------

#[test]
fn an_interface_without_a_matrix_reports_missing_not_unrestricted() {
    let loader = repo_loader();
    let loaded = loader
        .load_document(&fixture("negative/semantic/sr-coverage--surface-uncovered.md"))
        .expect("load");
    let document = loaded.record.expect("typed");
    let summary = coverage_summary(&document);
    let webhook = summary.interfaces.iter().find(|i| i.interface == "discord_webhook").expect("webhook");
    assert_eq!(webhook.categories.len(), 16);
    assert!(webhook.categories.iter().all(|c| c.state == CoverageState::Missing));
    assert!(!webhook.is_complete());

    let minimal = validated(&loader, &fixture("contract/minimal-valid.md"), &shipped_roster());
    let summary = minimal.coverage();
    let bot = &summary.interfaces[0];
    assert!(bot.categories.iter().all(|c| c.state == CoverageState::InvestigatedGap && c.records == 0));
    assert!(bot.is_complete(), "investigated gaps complete coverage without implying support");
    assert!(minimal.eligibility().is_empty(), "an empty constraint list projects nothing");

    let pilot = validated(&loader, &fixture("contract/pilot-slack.md"), &shipped_roster());
    assert!(pilot
        .coverage()
        .interfaces
        .iter()
        .any(|i| i.categories.iter().any(|c| c.state == CoverageState::OpenGap)));
}

#[test]
fn accepted_scope_requires_the_whole_fleet_and_every_roster_interface() {
    let roster = shipped_roster();
    let diagnostics = validate_fleet(
        &messenger::research::RepoPath::from_portable("messenger/docs/platforms.yaml"),
        &roster,
        &[PlatformId::Discord, PlatformId::Discord],
    );
    let subjects: Vec<&str> = diagnostics.iter().filter_map(|d| d.subject.as_deref()).collect();
    assert_eq!(subjects, vec!["discord", "signal", "slack", "telegram", "whatsapp"], "{}", show(&diagnostics));
    assert!(diagnostics.iter().all(|d| d.rule == Rule::Roster));

    let loader = repo_loader();
    let small = loader.load_roster(&fixture("contract/roster-cap-10.yaml")).expect("roster");
    assert!(validate_roster(&small, Scope::Fragment).is_empty());
    let accepted = validate_roster(&small, Scope::Accepted);
    assert!(!accepted.is_empty() && accepted.iter().all(|d| d.rule == Rule::Roster), "{}", show(&accepted));

    let pilot = loader.load_document(&fixture("contract/pilot-discord.md")).expect("load");
    let accepted = validate_document(&pilot, &Context { roster: Some(&roster), scope: Scope::Accepted });
    assert!(accepted.diagnostics.iter().all(|d| d.rule == Rule::Gap), "{}", show(&accepted.diagnostics));
    assert!(!accepted.diagnostics.is_empty(), "the pilot rests on open gaps");
}

/// The schema's `max(3660)` holds for a roster built in code too: the typed
/// pass accepts the ceiling and rejects the first value past it, for the
/// default and for a platform override.
#[test]
fn refresh_intervals_past_the_ceiling_are_roster_findings() {
    let loaded = repo_loader().load_roster(&fixture("contract/roster-refresh-interval-max.yaml")).expect("roster");
    assert!(validate_roster(&loaded, Scope::Fragment).is_empty(), "{}", show(&validate_roster(&loaded, Scope::Fragment)));
    let pointers = |edit: &dyn Fn(&mut Roster)| {
        let mut variant = loaded.clone();
        edit(variant.record.as_mut().expect("typed roster"));
        let diagnostics = validate_roster(&variant, Scope::Fragment);
        assert!(diagnostics.iter().all(|d| d.rule == Rule::Roster), "{}", show(&diagnostics));
        diagnostics.into_iter().map(|d| d.pointer).collect::<Vec<_>>()
    };
    assert_eq!(pointers(&|roster| roster.refresh_interval_days = MAX_REFRESH_INTERVAL_DAYS + 1), ["/refresh_interval_days"]);
    assert_eq!(pointers(&|roster| roster.refresh_interval_days = 0), ["/refresh_interval_days"]);
    assert_eq!(
        pointers(&|roster| roster.platforms[0].refresh_interval_days = Some(MAX_REFRESH_INTERVAL_DAYS + 1)),
        ["/platforms/0/refresh_interval_days"]
    );
}

/// A `last_updated` late enough that adding the interval passes 9999-12-31
/// is an SR-ROSTER finding, and the catalog projection refuses it rather
/// than writing a five-digit year into `refresh_due`.
#[test]
fn a_refresh_date_past_9999_is_a_finding_not_a_catalog_date() {
    let workspace = TempWorkspace::new();
    let mut roster = shipped_roster();
    roster.refresh_interval_days = MAX_REFRESH_INTERVAL_DAYS;
    let schema = SchemaEntry { version: 1, xxh64: schema_hash(workspace.root()) };
    let catalog = |accepted: &[AcceptedDocument]| {
        project(CatalogInputs { roster: &roster, documents: accepted, overrides: None, assessments: &[], schema: &schema, inputs: &[] })
    };
    let accept = |path: &Path, context: &Context<'_>| {
        let loaded = workspace.loader.load_document(path).expect("load");
        let result = validate_document(&loaded, context);
        assert!(result.diagnostics.is_empty(), "{}", show(&result.diagnostics));
        AcceptedDocument::new(result.validated.expect("validated"), &fs::read_to_string(path).expect("read")).expect("hash")
    };

    let last = workspace.document("last.md", "contract/minimal-valid.md", &[("last_updated: *id001", "last_updated: 9989-12-23")]);
    let projected = catalog(&[accept(&last, &fragment(&roster))]).expect("representable refresh date");
    assert_eq!(projected.platform(PlatformId::Discord).expect("discord").refresh_due.as_str(), "9999-12-31");

    let over = workspace.document("over.md", "contract/minimal-valid.md", &[("last_updated: *id001", "last_updated: 9989-12-24")]);
    let result = validate_document(&workspace.loader.load_document(&over).expect("load"), &fragment(&roster));
    let found: Vec<(Rule, &str)> = result.diagnostics.iter().map(|d| (d.rule, d.pointer.as_str())).collect();
    assert_eq!(found, [(Rule::Roster, "/last_updated")], "{}", show(&result.diagnostics));

    // Without a roster the document validates, so only the projection itself
    // stands between it and the catalog.
    let unchecked = accept(&over, &Context { roster: None, scope: Scope::Fragment });
    let refused = catalog(&[unchecked]).expect_err("refresh_due past 9999-12-31");
    let found: Vec<(Rule, &str)> = refused.iter().map(|d| (d.rule, d.pointer.as_str())).collect();
    assert_eq!(found, [(Rule::Roster, "/last_updated")], "{}", show(&refused));
}

/// Stale but structurally valid research stays inspectable: an old
/// `last_updated` is not a validation failure, and a semantically invalid
/// document still exposes its typed record and frontmatter.
#[test]
fn stale_or_invalid_research_remains_inspectable() {
    let workspace = TempWorkspace::new();
    let roster = shipped_roster();
    let stale = workspace.document("stale.md", "contract/units-unicode-scalars.md", &[("last_updated: *id001", "last_updated: 2020-01-01")]);
    let stale = validated(&workspace.loader, &stale, &roster);
    assert_eq!(stale.document().last_updated.as_str(), "2020-01-01");
    assert!(stale.eligibility().iter().any(|e| e.executable().is_some()));

    let loaded = repo_loader()
        .load_document(&fixture("negative/semantic/sr-unique--duplicate-fact-id.md"))
        .expect("load");
    let result = validate_document(&loaded, &fragment(&roster));
    assert!(result.validated.is_none());
    assert!(loaded.record.is_some(), "the typed record is still available for reporting");
    assert!(loaded.frontmatter["constraints"].is_array());
}

#[test]
fn findings_are_deterministic_and_sorted() {
    let roster = shipped_roster();
    let path = fixture("negative/semantic/sr-roster--research-only-with-adapter.md");
    let run = || validate_document(&repo_loader().load_document(&path).expect("load"), &fragment(&roster)).diagnostics;
    let first = run();
    assert!(first.len() >= 2);
    assert_eq!(first, run());
    let mut sorted = first.clone();
    sorted.sort();
    assert_eq!(first, sorted);
}

// ---- overrides ---------------------------------------------------------------

fn schema_hash(root: &Path) -> String {
    let read = |name: &str| fs::read_to_string(root.join("messenger/docs/research/platforms").join(name)).expect("schema");
    schema_fingerprint(&read("_schema.yaml"), &read("_types.yaml"))
}

#[test]
fn overrides_fail_precisely_when_expired_orphaned_or_stale() {
    let loader = repo_loader();
    let roster = shipped_roster();
    let companion = validated(&loader, &fixture("contract/constraints-field.md"), &roster);
    let documents = [&companion];
    let current = schema_hash(&repo_root());
    let pointers = |name: &str, day: &Date, schema: &str| -> Vec<String> {
        let loaded = loader.load_overrides(&fixture(name)).expect("overrides");
        validate_overrides(&loaded, &documents, schema, day)
            .into_iter()
            .inspect(|d| assert_eq!(d.rule, Rule::Override))
            .map(|d| d.pointer)
            .collect()
    };
    assert!(pointers("contract/overrides-valid.yaml", &today(), &current).is_empty());
    // The review date itself is still valid; the day after is not.
    assert!(pointers("contract/overrides-valid.yaml", &Date::parse("2027-09-17").unwrap(), &current).is_empty());
    assert_eq!(
        pointers("contract/overrides-valid.yaml", &Date::parse("2027-09-18").unwrap(), &current),
        vec!["/overrides/0/review_by"]
    );
    assert_eq!(
        pointers("contract/overrides-valid.yaml", &today(), "xxh64:0000000000000000"),
        vec!["/overrides/0/schema_hash"]
    );
    assert_eq!(pointers("negative/semantic/sr-override--expired-override.yaml", &today(), &current), vec!["/overrides/0/review_by"]);
    assert_eq!(pointers("negative/semantic/sr-override--orphaned-override.yaml", &today(), &current), vec![
        "/overrides/0/fact",
        "/overrides/0/schema_hash"
    ]);
    assert_eq!(pointers("negative/semantic/sr-override--stale-override.yaml", &today(), &current), vec![
        "/overrides/0/schema_hash",
        "/overrides/0/target_hash"
    ]);
    let fact = companion.record("c.fx.content.service_max").expect("fact");
    assert_eq!(record_fingerprint(fact), "xxh64:e411ea8353eb4024", "overrides-valid.yaml pins this fact");
}

// ---- implementation assessments ----------------------------------------------

const CODE: &str = "messenger/lib/src/provider/discord.rs";
const TEST: &str = "messenger/lib/src/tests/discord.rs";

fn mappings(code: &str, test: &str, fact: &str, review: &str, status: &str) -> Mappings {
    let review = if review == "accepted" {
        serde_json::json!({"status": "accepted", "proposed_by": "agent", "reviewed_by": "maintainer", "reviewed_on": "2026-09-17"})
    } else {
        serde_json::json!({"status": review, "proposed_by": "agent"})
    };
    let mut assessment = serde_json::json!({
        "id": "map.fx.discord.constraints",
        "adapter": "discord",
        "category": "constraints",
        "platform_id": "discord",
        "facts": ["c.fx.content.service_max"],
        "status": status,
        "summary": "Content is truncated.",
        "code_refs": [CODE],
        "test_refs": [TEST],
        "assessed_revision": "d57faf7e8",
        "fingerprints": [
            {"input": CODE, "kind": "code_file", "digest": code},
            {"input": TEST, "kind": "test_file", "digest": test},
            {"input": "discord#c.fx.content.service_max", "kind": "research_fact", "digest": fact},
        ],
        "review": review,
        "requires_messenger_update": false,
    });
    if status == "unassessed" {
        assessment["stale_reason"] = Value::String("never_reviewed".into());
    }
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "inspected_revision": "d57faf7e8",
        "fingerprint_algorithm": "xxh64",
        "assessments": [assessment],
    }))
    .expect("mappings")
}

#[test]
fn assessments_are_reused_only_while_every_fingerprint_matches() {
    let files = TempWorkspace::new();
    let workspace = Workspace::new(files.root()).expect("workspace");
    files.write(CODE, "fn send() {}\n");
    files.write(TEST, "#[test]\nfn sends() {}\n");
    files.write("messenger/lib/src/provider/slack.rs", "fn unrelated() {}\n");

    let roster = shipped_roster();
    let document = validated(&repo_loader(), &fixture("contract/constraints-field.md"), &roster);
    let fact = record_fingerprint(document.record("c.fx.content.service_max").expect("fact"));
    let code = text_fingerprint("fn send() {}\n");
    let test = text_fingerprint("#[test]\nfn sends() {}\n");
    let accepted = mappings(&code, &test, &fact, "accepted", "partial");
    let state = |mappings: &Mappings, document: &ValidatedDocument| evaluate(mappings, &[document], &workspace)[0].clone();

    let current = state(&accepted, &document);
    assert_eq!(current.state, AssessmentState::Current { status: ImplementationStatus::Partial });
    assert!(current.gap.is_none());
    assert_eq!(evaluate(&accepted, &[&document], &workspace), evaluate(&accepted, &[&document], &workspace));

    // Provenance and unrelated files never invalidate an assessment.
    let mut moved = accepted.clone();
    moved.inspected_revision = "0123abc".into();
    moved.assessments[0].assessed_revision = "0123abc".into();
    files.write("messenger/lib/src/provider/slack.rs", "fn unrelated_changed() {}\n");
    assert!(matches!(state(&moved, &document).state, AssessmentState::Current { .. }));

    // A CRLF checkout of the same code fingerprints the same.
    files.write(CODE, "fn send() {}\r\n");
    assert!(matches!(state(&accepted, &document).state, AssessmentState::Current { .. }));

    // A relevant code change: unassessed, with the input named and a gap.
    files.write(CODE, "fn send() { truncate(); }\n");
    let changed = state(&accepted, &document);
    match &changed.state {
        AssessmentState::Unassessed { reason, changed } => {
            assert_eq!(*reason, StaleReason::FingerprintChanged);
            assert_eq!(changed.len(), 1);
            assert_eq!(changed[0].input, CODE);
        }
        other => panic!("expected unassessed, got {other:?}"),
    }
    let gap = changed.gap.expect("requires_messenger_update gap");
    assert_eq!(gap.facts, vec!["c.fx.content.service_max".to_string()]);
    files.write(CODE, "fn send() {}\n");

    // A changed research fact invalidates; a removed one says so.
    let workspace_docs = TempWorkspace::new();
    let raised = workspace_docs.document("raised.md", "contract/constraints-field.md", &[("  value: 2000", "  value: 4000")]);
    let raised = validated(&workspace_docs.loader, &raised, &roster);
    assert!(matches!(
        state(&accepted, &raised).state,
        AssessmentState::Unassessed { reason: StaleReason::FingerprintChanged, .. }
    ));
    let other = validated(&repo_loader(), &fixture("contract/minimal-valid.md"), &roster);
    assert!(matches!(
        state(&accepted, &other).state,
        AssessmentState::Unassessed { reason: StaleReason::FactRemoved, .. }
    ));

    // A proposed assessment is never an implementation claim.
    let proposed = mappings(&code, &test, &fact, "proposed", "unassessed");
    let outcome = state(&proposed, &document);
    assert!(matches!(outcome.state, AssessmentState::Unassessed { reason: StaleReason::NeverReviewed, .. }));
    assert!(outcome.gap.is_some());
}
