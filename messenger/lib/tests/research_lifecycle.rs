//! Delta engine (feature `research`): fact-level before/after comparison, the
//! fixed suspicious-change flags, initial research without a baseline,
//! evidence changes at an unchanged URL, prose-only changes, and the
//! assessments a change affects. Documents are variants of real fixtures,
//! validated through the loader before comparison.
#![cfg(feature = "research")]

use std::fs;
use std::path::{Path, PathBuf};

use messenger::research::delta::{Baseline, ChangeKind, Delta, FlagKind, compare};
use messenger::research::model::{Mappings, Roster};
use messenger::research::project::AcceptedDocument;
use messenger::research::{Context, Loader, Scope, Workspace, validate_document};
use tempfile::TempDir;

fn lib_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn repo_root() -> PathBuf {
    lib_dir().parent().and_then(Path::parent).expect("repository root").to_path_buf()
}

const SCHEMAS: &[&str] = &[
    "messenger/docs/platforms.yaml",
    "messenger/docs/platforms.schema.yaml",
    "messenger/docs/research/platforms/_schema.yaml",
    "messenger/docs/research/platforms/_types.yaml",
    "messenger/docs/research/implementation/_schema.yaml",
];

struct Repo {
    _dir: TempDir,
    loader: Loader,
    roster: Roster,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        for path in SCHEMAS {
            let target = dir.path().join(path);
            fs::create_dir_all(target.parent().expect("parent")).expect("mkdir");
            fs::copy(repo_root().join(path), target).expect("copy");
        }
        let loader = Loader::new(Workspace::new(dir.path()).expect("absolute"));
        let roster = loader
            .load_roster(&dir.path().join("messenger/docs/platforms.yaml"))
            .expect("roster")
            .record
            .expect("typed roster");
        Self { _dir: dir, loader, roster }
    }

    /// Validates `text` as the accepted Discord document.
    fn accepted(&self, text: &str) -> AcceptedDocument {
        let path = self.loader.workspace().document(messenger::research::model::PlatformId::Discord);
        let loaded = self.loader.load_document_text(&path, text).expect("load");
        let result = validate_document(&loaded, &Context { roster: Some(&self.roster), scope: Scope::Fragment });
        let shown: Vec<String> = result.diagnostics.iter().map(ToString::to_string).collect();
        assert!(result.diagnostics.is_empty(), "{}", shown.join("\n"));
        AcceptedDocument::new(result.validated.expect("validated"), text).expect("hash")
    }
}

/// A contract fixture rebound to the accepted document location.
fn fixture(name: &str) -> String {
    fs::read_to_string(lib_dir().join("tests/fixtures/research").join(name))
        .expect("fixture")
        .lines()
        .map(|line| if line.starts_with("$schema:") { "$schema: ./_schema.yaml" } else { line })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "edit target not found: {from}");
    text.replacen(from, to, 1)
}

fn flags(delta: &Delta) -> Vec<(FlagKind, &str)> {
    delta.flags.iter().map(|flag| (flag.kind, flag.fact.as_str())).collect()
}

#[test]
fn initial_research_reports_no_baseline_instead_of_inventing_one() {
    let repo = Repo::new();
    let candidate = repo.accepted(&fixture("contract/knowledge-states.md"));
    let delta = compare(None, &candidate, None);
    assert_eq!(delta.baseline, Baseline::None);
    assert!(delta.facts.iter().all(|fact| fact.change == ChangeKind::Added && fact.before.is_none()));
    assert_eq!(delta.facts.len(), 3);
    assert!(delta.sources.is_empty() && delta.gaps.is_empty(), "nothing to compare sources and gaps with");
    assert!(delta.prose.changed && delta.prose.before_body_hash.is_none());
    assert!(!delta.is_unchanged());
    assert_eq!(delta.conclusion_kind, "mechanical");
    // The initial conflict and the unknown unit are visible as flags.
    assert!(flags(&delta).contains(&(FlagKind::ConflictingEvidence, "c.fx.conflict")));
    assert!(flags(&delta).contains(&(FlagKind::NewUnmappableValue, "c.fx.unknown")));
}

#[test]
fn identical_documents_produce_an_empty_deterministic_delta() {
    let repo = Repo::new();
    let text = fixture("contract/constraints-field.md");
    let (before, after) = (repo.accepted(&text), repo.accepted(&text));
    let delta = compare(Some(&before), &after, None);
    assert!(delta.is_unchanged(), "{delta:?}");
    assert!(delta.flags.is_empty());
    let again = compare(Some(&before), &after, None);
    assert_eq!(serde_json::to_vec(&delta).unwrap(), serde_json::to_vec(&again).unwrap());
}

#[test]
fn a_raised_limit_is_a_typed_change_with_its_flag_and_both_records() {
    let repo = Repo::new();
    let text = fixture("contract/constraints-field.md");
    let before = repo.accepted(&text);
    let after = repo.accepted(&edit(&text, "  value: 2000\n", "  value: 4000\n"));
    let delta = compare(Some(&before), &after, None);
    assert_eq!(delta.facts.len(), 1);
    let fact = &delta.facts[0];
    assert_eq!((fact.array.as_str(), fact.id.as_str(), fact.change), ("constraints", "c.fx.content.service_max", ChangeKind::Changed));
    assert_eq!(fact.typed_changes.len(), 1);
    assert_eq!(fact.typed_changes[0].pointer, "/value");
    assert_eq!(fact.typed_changes[0].before, Some(2000.into()));
    assert_eq!(fact.typed_changes[0].after, Some(4000.into()));
    assert_eq!(fact.before.as_ref().unwrap()["value"], 2000);
    assert_eq!(fact.after.as_ref().unwrap()["value"], 4000);
    assert_eq!(flags(&delta), [(FlagKind::RaisedLimit, "c.fx.content.service_max")]);

    // Lowering a maximum is a change but not a suspicious one.
    let lowered = repo.accepted(&edit(&text, "  value: 2000\n", "  value: 1000\n"));
    let delta = compare(Some(&before), &lowered, None);
    assert_eq!(delta.facts.len(), 1);
    assert!(delta.flags.is_empty());
}

#[test]
fn unit_changes_removed_constraints_and_new_unmappable_values_are_flagged() {
    let repo = Repo::new();
    let text = fixture("contract/constraints-field.md");
    let before = repo.accepted(&edit(&text, "  unit: unspecified_characters\n", "  unit: utf16_code_units\n"));
    let after = repo.accepted(&text);
    let delta = compare(Some(&before), &after, None);
    assert_eq!(
        flags(&delta),
        [(FlagKind::ChangedUnit, "c.fx.content.service_max"), (FlagKind::NewUnmappableValue, "c.fx.content.service_max")]
    );

    let without = edit(
        &text,
        &text[text.find("constraints:\n- id:").unwrap()..text.find("format_profiles:").unwrap()],
        "constraints: []\n",
    );
    let without = edit(&without, "    constraints:\n      status: researched\n", "    constraints:\n      status: gap\n      gap: gap.fx.scope\n");
    let delta = compare(Some(&after), &repo.accepted(&without), None);
    assert_eq!(flags(&delta), [(FlagKind::RemovedConstraint, "c.fx.content.service_max")]);
    assert_eq!(delta.facts[0].change, ChangeKind::Removed);
    assert!(delta.facts[0].after.is_none() && delta.facts[0].before.is_some());
}

#[test]
fn support_reversals_are_flagged() {
    let repo = Repo::new();
    let text = fixture("contract/delivery-controls-eligibility-rates.md");
    let before = repo.accepted(&text);
    let after = repo.accepted(&edit(&text, "  control: link_preview\n  support: supported\n", "  control: link_preview\n  support: unsupported\n"));
    let delta = compare(Some(&before), &after, None);
    assert_eq!(flags(&delta), [(FlagKind::SupportReversal, "dc.fx.preview")]);
    assert_eq!(delta.facts[0].typed_changes[0].pointer, "/support");
}

#[test]
fn a_newly_conflicting_record_is_flagged() {
    let repo = Repo::new();
    let text = fixture("contract/knowledge-states.md");
    let known = edit(
        &text,
        "  knowledge:\n    state: conflicting\n    evidence:\n    - src.fx.docs\n    - src.fx.sdk\n    gap: gap.fx.content_limit\n    explanation: Sources disagree.\n    claims:\n    - statement: 'Docs: 4096.'\n      value: 4096\n      evidence:\n      - src.fx.docs\n    - statement: 'SDK: 4000.'\n      value: 4000\n      evidence:\n      - src.fx.sdk\n",
        "  knowledge:\n    state: known\n    evidence:\n    - src.fx.docs\n",
    );
    let known = edit(&known, "  surface: rich_description\n  native_locator: embeds[].description\n  kind: hard_max\n", "  surface: rich_description\n  native_locator: embeds[].description\n  kind: hard_max\n  value: 4096\n");
    let delta = compare(Some(&repo.accepted(&known)), &repo.accepted(&text), None);
    assert!(flags(&delta).contains(&(FlagKind::ConflictingEvidence, "c.fx.conflict")), "{:?}", delta.flags);
    let fact = delta.facts.iter().find(|f| f.id == "c.fx.conflict").unwrap();
    assert_eq!(fact.evidence_added, ["src.fx.sdk"]);
    assert!(fact.prose_changes.iter().any(|c| c.pointer == "/knowledge/explanation"));
}

#[test]
fn changed_evidence_at_the_same_url_is_reported_with_its_citing_facts() {
    let repo = Repo::new();
    let text = fixture("contract/constraints-field.md");
    let before = repo.accepted(&text);
    let after = repo.accepted(&edit(&text, "  url: https://docs.example.com/api/messages\n  locator: Limits\n", "  url: https://docs.example.com/api/messages\n  locator: Limits (revised)\n"));
    let delta = compare(Some(&before), &after, None);
    assert!(delta.facts.is_empty(), "typed facts are unchanged");
    assert_eq!(delta.sources.len(), 1);
    let source = &delta.sources[0];
    assert_eq!(source.id, "src.fx.docs");
    assert!(source.same_location);
    assert_eq!(source.field_changes[0].pointer, "/locator");
    assert_eq!(source.cited_by, ["c.fx.content.service_max"]);
    assert!(!delta.is_unchanged());
}

#[test]
fn prose_only_changes_are_visible_with_unchanged_typed_values() {
    let repo = Repo::new();
    let text = fixture("contract/constraints-field.md");
    let before = repo.accepted(&text);

    let body = repo.accepted(&edit(&text, "Contract fixture; not research.", "Contract fixture; not research. Revised explanation."));
    let delta = compare(Some(&before), &body, None);
    assert!(delta.facts.is_empty() && delta.sources.is_empty());
    assert!(delta.prose.changed);
    assert_ne!(delta.prose.before_body_hash.as_deref(), Some(delta.prose.after_body_hash.as_str()));

    let explanation = repo.accepted(&edit(
        &text,
        "    explanation: Documentation says characters without defining the unit.\n",
        "    explanation: Documentation still says characters without a unit.\n",
    ));
    let delta = compare(Some(&before), &explanation, None);
    assert!(!delta.prose.changed, "the body is unchanged");
    let fact = &delta.facts[0];
    assert!(fact.typed_changes.is_empty());
    assert_eq!(fact.prose_changes.len(), 1);
    assert_eq!(fact.prose_changes[0].pointer, "/knowledge/explanation");
    assert!(delta.flags.is_empty());
}

#[test]
fn a_changed_fact_names_the_accepted_assessments_it_affects() {
    let repo = Repo::new();
    let text = fixture("contract/constraints-field.md");
    let before = repo.accepted(&text);
    let after = repo.accepted(&edit(&text, "  value: 2000\n", "  value: 4000\n"));
    let mappings: Mappings = {
        let path = repo.loader.workspace().mappings();
        let text = fixture("contract/mappings-valid.yaml");
        fs::write(&path, text).expect("write mappings");
        let loaded = repo.loader.load_mappings(&path).expect("mappings");
        loaded.record.expect("typed mappings")
    };
    let delta = compare(Some(&before), &after, Some(&mappings));
    let accepted_on_fact: Vec<&str> = mappings
        .assessments
        .iter()
        .filter(|a| a.review.status == messenger::research::model::ReviewStatus::Accepted && a.facts.iter().any(|f| f == "c.fx.content.service_max"))
        .map(|a| a.id.as_str())
        .collect();
    let affected: Vec<&str> = delta.affected_assessments.iter().map(|a| a.assessment.as_str()).collect();
    assert_eq!(affected, accepted_on_fact);
    assert!(!affected.is_empty(), "the mappings fixture assesses the changed fact");
}
