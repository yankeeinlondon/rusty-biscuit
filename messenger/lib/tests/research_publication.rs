//! Snapshot generation and publication (feature `research`): deterministic
//! double generation, drift checking, refusal of invalid or incomplete
//! inputs, partial refresh, fault injection at every publication step, and
//! recovery. Every test runs in a throwaway repository that holds copies of the
//! shipped schemas and roster plus the accepted-fleet fixture
//! (`tests/fixtures/research/lifecycle/fleet/`) at the fixed document paths.
#![cfg(feature = "research")]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use messenger::research::canonical::{record_fingerprint, schema_fingerprint};
use messenger::research::generate::{self, Baseline, Drift, GenerateError, check, generate, load_fleet, published_catalog};
use messenger::research::model::{Date, PlatformId};
use messenger::research::publish::{
    self, Options, Point, PublishError, REGION_BEGIN, REGION_END, Recovery, read_verified, recover,
};
use messenger::research::report::{self, Enforceability, Filter};
use messenger::research::{Loader, Rule, Workspace};
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

fn today() -> Date {
    Date::parse("2026-09-17").expect("date")
}

const SHIPPED: &[&str] = &[
    "messenger/docs/platforms.yaml",
    "messenger/docs/platforms.schema.yaml",
    "messenger/docs/research/platforms/_schema.yaml",
    "messenger/docs/research/platforms/_types.yaml",
    "messenger/docs/research/platforms/_overrides.schema.yaml",
    "messenger/docs/research/platforms/_fleet.md",
    "messenger/docs/research/implementation/_schema.yaml",
];

const CATALOG: &str = "messenger/docs/research/platforms/catalog.json";
const SUMMARY: &str = "messenger/docs/research/summary/platforms.md";
const MANIFEST: &str = "messenger/docs/research/publication.json";

/// A throwaway repository: shipped schemas, roster, and fleet prompt, plus
/// the five fleet fixtures at their accepted paths.
struct Repo {
    dir: TempDir,
    loader: Loader,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        for path in SHIPPED {
            let target = dir.path().join(path);
            fs::create_dir_all(target.parent().expect("parent")).expect("mkdir");
            fs::copy(repo_root().join(path), target).expect("copy shipped input");
        }
        let repo = Self {
            loader: Loader::new(Workspace::new(dir.path()).expect("absolute")),
            dir,
        };
        for platform in PlatformId::ALL {
            repo.write(&document(*platform), &fleet_text(*platform));
        }
        repo
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn workspace(&self) -> &Workspace {
        self.loader.workspace()
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root().to_path_buf(), |acc, segment| acc.join(segment))
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, content).expect("write");
    }

    fn read(&self, relative: &str) -> Option<Vec<u8>> {
        fs::read(self.path(relative)).ok()
    }

    /// Every file under `messenger/`, excluding local state.
    fn tree(&self) -> BTreeMap<String, Vec<u8>> {
        fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(dir).expect("read dir").flatten() {
                let path = entry.path();
                let relative = path.strip_prefix(root).expect("under root");
                if relative.starts_with("messenger/.research-state") {
                    continue;
                }
                if path.is_dir() {
                    walk(&path, root, out);
                } else {
                    let key = relative.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                    out.insert(key, fs::read(&path).expect("read"));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(self.root(), self.root(), &mut out);
        out
    }

    fn generate(&self) -> Result<generate::Generated, GenerateError> {
        generate(&self.loader, &BTreeMap::new(), &today(), Options::default())
    }

    fn generate_with(&self, updates: &BTreeMap<PlatformId, String>, options: Options) -> Result<generate::Generated, GenerateError> {
        generate(&self.loader, updates, &today(), options)
    }

    fn state_is_clean(&self) -> bool {
        let publication = self.root().join("messenger/.research-state/publication");
        !publication.join("journal.json").exists() && !publication.join("tx").exists()
    }
}

fn document(platform: PlatformId) -> String {
    format!("messenger/docs/research/platforms/{platform}.md")
}

/// A fleet fixture rebound to the accepted document location.
fn fleet_text(platform: PlatformId) -> String {
    let path = lib_dir().join(format!("tests/fixtures/research/lifecycle/fleet/{platform}.md"));
    fs::read_to_string(path)
        .expect("fleet fixture")
        .replace("$schema: ../../../../../../docs/research/platforms/_schema.yaml", "$schema: ./_schema.yaml")
}

fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "edit target not found: {from}");
    text.replacen(from, to, 1)
}

fn catalog_json(repo: &Repo) -> Value {
    serde_json::from_slice(&repo.read(CATALOG).expect("catalog")).expect("catalog json")
}

fn show(diagnostics: &[messenger::research::Diagnostic]) -> String {
    diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n  ")
}

// ---- the fixture -------------------------------------------------------------

#[test]
fn the_fleet_fixture_is_a_clean_accepted_fleet() {
    let repo = Repo::new();
    let fleet = load_fleet(&repo.loader, Baseline::FixedPaths, &BTreeMap::new()).expect("load");
    let validation = fleet.validate(&today());
    assert!(validation.is_clean(), "{}", show(&validation.diagnostics));
    assert_eq!(validation.accepted.len(), 5);
}

// ---- determinism and drift ---------------------------------------------------

#[test]
fn two_generations_from_identical_inputs_are_byte_identical() {
    let repo = Repo::new();
    let first = repo.generate().expect("first generation");
    assert!(!first.unchanged);
    let after_first = repo.tree();
    let second = repo.generate().expect("second generation");
    assert_eq!(repo.tree(), after_first, "the second generation rewrote bytes");
    assert!(second.unchanged, "{second:?}");
    assert_eq!((second.replaced, second.removed), (0, 0));
    assert_eq!(first.snapshot_id, second.snapshot_id);
    assert!(repo.state_is_clean());

    // An independent repository with the same inputs yields the same bytes:
    // nothing depends on the host path or the clock.
    let other = Repo::new();
    other.generate().expect("other generation");
    for path in [CATALOG, SUMMARY, MANIFEST] {
        assert_eq!(repo.read(path), other.read(path), "{path} differs between repositories");
    }
    let catalog = String::from_utf8(repo.read(CATALOG).expect("catalog")).expect("utf-8");
    assert!(!catalog.contains(&repo.root().display().to_string()), "host path leaked into the catalog");
    assert!(!catalog.contains('\r'));
    assert!(check(&repo.loader, &today()).expect("check").is_none());
}

#[test]
fn check_reports_drift_for_changed_inputs_and_refuses_hand_edits() {
    let repo = Repo::new();
    repo.generate().expect("generate");

    // A changed input (the roster's curated-source text) changes the catalog
    // and the manifest's input fingerprints; check names both.
    let roster = String::from_utf8(repo.read("messenger/docs/platforms.yaml").expect("roster")).expect("utf-8");
    repo.write(
        "messenger/docs/platforms.yaml",
        &roster.replacen("refresh_interval_days: 30", "refresh_interval_days: 45", 1),
    );
    let drift = check(&repo.loader, &today()).expect("check");
    let Some(Drift::Changed { paths }) = drift else { panic!("expected drift, got {drift:?}") };
    assert_eq!(paths, vec![CATALOG.to_string(), MANIFEST.to_string()]);
    repo.generate().expect("regenerate");
    assert!(check(&repo.loader, &today()).expect("check").is_none());
    assert_eq!(catalog_json(&repo)["platforms"][0]["refresh_due"], "2026-10-25", "10 September + 45 days");

    // A hand edit to a published artifact fails verification for check,
    // generate, and reporting alike.
    let original = repo.read(CATALOG).expect("catalog");
    repo.write(CATALOG, &String::from_utf8(original.clone()).expect("utf-8").replace("\"discord\"", "\"Discord\""));
    let refused = |result: Result<(), PublishError>| {
        matches!(result, Err(PublishError::Inconsistent { ref path, .. }) if path == CATALOG)
    };
    assert!(refused(check(&repo.loader, &today()).map(|_| ()).map_err(|e| match e {
        GenerateError::Publish(error) => error,
        other => panic!("{other}"),
    })));
    assert!(refused(repo.generate().map(|_| ()).map_err(|e| match e {
        GenerateError::Publish(error) => error,
        other => panic!("{other}"),
    })));
    assert!(refused(published_catalog(repo.workspace()).map(|_| ())));
    fs::write(repo.path(CATALOG), original).expect("restore");
    assert!(check(&repo.loader, &today()).expect("check").is_none());
}

#[test]
fn the_summary_keeps_authored_prose_and_regenerates_only_its_region() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    let summary = String::from_utf8(repo.read(SUMMARY).expect("summary")).expect("utf-8");
    assert!(summary.contains(REGION_BEGIN) && summary.contains(REGION_END));
    assert!(summary.contains("| discord | `discord_bot_api` |"), "{summary}");
    assert!(summary.contains("`c.discord.content.max`"), "handoff rows cite stable IDs");

    // Authored prose outside the region may change without drift, and the
    // next generation keeps it byte for byte.
    let authored = summary.replacen("# Provider research summary\n", "# Provider research summary\n\nMaintainer guidance.\n", 1);
    repo.write(SUMMARY, &authored);
    assert!(read_verified(repo.workspace()).is_ok());
    assert!(check(&repo.loader, &today()).expect("check").is_none());
    repo.generate().expect("regenerate");
    assert_eq!(repo.read(SUMMARY).expect("summary"), authored.as_bytes());

    // Editing inside the generated region is a hand edit of generated output.
    repo.write(SUMMARY, &authored.replacen("| discord |", "| Discord |", 1));
    assert!(matches!(read_verified(repo.workspace()), Err(PublishError::Inconsistent { path, .. }) if path == SUMMARY));
}

// ---- refusal preserves the previous snapshot ---------------------------------

#[test]
fn invalid_inputs_are_refused_and_leave_the_snapshot_untouched() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    let before = repo.tree();

    // An expired override is found only against `today`; generation refuses.
    let fact = {
        let fleet = load_fleet(&repo.loader, Baseline::FixedPaths, &BTreeMap::new()).expect("load");
        let validation = fleet.validate(&today());
        let discord = validation.accepted.iter().find(|d| d.validated.platform_id() == PlatformId::Discord).expect("discord");
        record_fingerprint(discord.validated.record("c.discord.embed.description").expect("fact"))
    };
    let read = |name: &str| fs::read_to_string(repo.path(&format!("messenger/docs/research/platforms/{name}"))).expect("schema");
    let schema = schema_fingerprint(&read("_schema.yaml"), &read("_types.yaml"));
    repo.write(
        "messenger/docs/research/platforms/_overrides.yaml",
        &format!(
            "---\n$schema: ./_overrides.schema.yaml\nschema_version: 1\noverrides:\n- id: ovr.discord.unit\n  platform_id: discord\n  fact: c.discord.embed.description\n  field: unit\n  effective_value: unicode_scalars\n  target_hash: {fact}\n  schema_hash: {schema}\n  evidence:\n  - src.discord.docs\n  reason: Fixture correction.\n  author: maintainer\n  review_by: 2026-09-16\n---\n"
        ),
    );
    let before_with_overrides = repo.tree();
    let Err(GenerateError::Refused { diagnostics, missing }) = repo.generate() else { panic!("expected refusal") };
    assert!(missing.is_empty());
    assert_eq!(diagnostics.iter().map(|d| d.rule).collect::<Vec<_>>(), vec![Rule::Override], "{}", show(&diagnostics));
    assert_eq!(repo.tree(), before_with_overrides, "a refused generation wrote something");
    let mut expected = before.clone();
    expected.insert("messenger/docs/research/platforms/_overrides.yaml".to_string(), repo.read("messenger/docs/research/platforms/_overrides.yaml").unwrap());
    assert_eq!(repo.tree(), expected);
    assert!(read_verified(repo.workspace()).is_ok(), "the previous snapshot stays selected");

    // `check` reports the same findings as drift, without writing.
    let Some(Drift::Invalid { diagnostics, .. }) = check(&repo.loader, &today()).expect("check") else { panic!() };
    assert_eq!(diagnostics.len(), 1);

    // The published catalog stays inspectable although today's validation fails.
    let catalog = published_catalog(repo.workspace()).expect("published catalog");
    assert_eq!(catalog.platforms.len(), 5);

    // Renewing the override makes the fleet publishable, and both the
    // researched and the effective value are exposed.
    let overrides = fs::read_to_string(repo.path("messenger/docs/research/platforms/_overrides.yaml")).unwrap();
    repo.write("messenger/docs/research/platforms/_overrides.yaml", &overrides.replace("review_by: 2026-09-16", "review_by: 2026-12-31"));
    repo.generate().expect("generate with a current override");
    let catalog = catalog_json(&repo);
    let applied = &catalog["platforms"][0]["overrides"][0];
    assert_eq!(applied["researched_value"], "unspecified_characters");
    assert_eq!(applied["effective_value"], "unicode_scalars");
    let eligibility = catalog["platforms"][0]["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "c.discord.embed.description")
        .unwrap()
        .clone();
    assert_eq!(eligibility["eligibility"], "ineligible", "an override never makes a researched value executable");
}

#[test]
fn a_missing_initial_baseline_refuses_publication_and_writes_nothing() {
    let repo = Repo::new();
    fs::remove_file(repo.path(&document(PlatformId::Slack))).expect("remove");
    let before = repo.tree();
    let Err(GenerateError::Refused { missing, .. }) = repo.generate() else { panic!("expected refusal") };
    assert_eq!(missing, vec![PlatformId::Slack]);
    assert_eq!(repo.tree(), before);
    assert!(matches!(read_verified(repo.workspace()), Err(PublishError::NoSnapshot)));
}

#[test]
fn a_published_snapshot_is_the_only_baseline_for_carried_documents() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    // A document at the fixed path that the manifest does not list cannot
    // sneak in: with a published snapshot, documents come only from it or
    // from explicit updates.
    let manifest = String::from_utf8(repo.read(MANIFEST).unwrap()).unwrap();
    let mut value: Value = serde_json::from_str(&manifest).unwrap();
    value["artifacts"].as_array_mut().unwrap().retain(|a| a["path"] != document(PlatformId::Signal));
    repo.write(MANIFEST, &serde_json::to_string_pretty(&value).unwrap());
    let Err(GenerateError::Refused { missing, .. }) = repo.generate() else { panic!("expected refusal") };
    assert_eq!(missing, vec![PlatformId::Signal]);
}

// ---- partial refresh ---------------------------------------------------------

#[test]
fn a_partial_refresh_carries_prior_documents_with_their_own_dates() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    let slack_before = repo.read(&document(PlatformId::Slack)).unwrap();

    let discord = edit(&fleet_text(PlatformId::Discord), "last_updated: 2026-09-10", "last_updated: 2026-09-17");
    let discord = edit(&discord, "  value: 2000\n", "  value: 4000\n");
    let updates = BTreeMap::from([(PlatformId::Discord, discord.clone())]);
    let generated = repo.generate_with(&updates, Options::default()).expect("partial refresh");
    assert!(!generated.unchanged);

    assert_eq!(repo.read(&document(PlatformId::Discord)).unwrap(), discord.as_bytes());
    assert_eq!(repo.read(&document(PlatformId::Slack)).unwrap(), slack_before, "the failed platform is carried byte for byte");
    let catalog = catalog_json(&repo);
    let platform = |id: &str| catalog["platforms"].as_array().unwrap().iter().find(|p| p["platform_id"] == id).unwrap().clone();
    assert_eq!(platform("discord")["last_updated"], "2026-09-17");
    assert_eq!(platform("slack")["last_updated"], "2026-09-10", "carried research keeps its real freshness");
    assert_eq!(platform("slack")["refresh_due"], "2026-10-10");
    assert!(check(&repo.loader, &today()).expect("check").is_none());
}

#[test]
fn incompatible_or_incomplete_documents_block_a_partial_refresh() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    let before = repo.tree();

    // A candidate on another schema version is never mixed in.
    let v2 = edit(&fleet_text(PlatformId::Discord), "schema_version: 1", "schema_version: 2");
    let Err(GenerateError::Refused { diagnostics, .. }) = repo.generate_with(&BTreeMap::from([(PlatformId::Discord, v2)]), Options::default())
    else {
        panic!("expected refusal")
    };
    assert!(diagnostics.iter().any(|d| d.rule == Rule::Version || d.rule == Rule::Schema), "{}", show(&diagnostics));
    assert_eq!(repo.tree(), before);

    // A carried prior document that no longer satisfies required coverage (the
    // roster gained an interface it does not cover) blocks publication.
    let roster = String::from_utf8(repo.read("messenger/docs/platforms.yaml").unwrap()).unwrap();
    let roster = edit(
        &roster,
        "      interfaces:\n          - interface_id: signal_cli_jsonrpc",
        "      interfaces:\n          - interface_id: signal_receive\n            role: research_only\n            api_identity: signal-cli receive\n            adapters: []\n            identification_url: https://example.com/signal-receive\n            relationships:\n                - kind: receives_events_for\n                  target: signal_cli_jsonrpc\n          - interface_id: signal_cli_jsonrpc",
    );
    repo.write("messenger/docs/platforms.yaml", &roster);
    let before = repo.tree();
    let update = edit(&fleet_text(PlatformId::Discord), "last_updated: 2026-09-10", "last_updated: 2026-09-17");
    let Err(GenerateError::Refused { diagnostics, .. }) =
        repo.generate_with(&BTreeMap::from([(PlatformId::Discord, update)]), Options::default())
    else {
        panic!("expected refusal")
    };
    assert!(
        diagnostics.iter().any(|d| d.rule == Rule::Roster && d.subject.as_deref() == Some("signal_receive")),
        "{}",
        show(&diagnostics)
    );
    assert_eq!(repo.tree(), before, "the previous usable snapshot is preserved");
}

// ---- interruption and recovery -----------------------------------------------

fn interruption_points(artifacts: usize) -> Vec<Point> {
    let mut points = vec![Point::BeforeStaging, Point::DuringStaging, Point::StagedNoJournal, Point::BeforeSelection];
    points.extend((1..=artifacts).map(Point::MidReplacement));
    points.extend([Point::BeforeManifest, Point::AfterSelection, Point::BeforeCleanup]);
    points
}

#[test]
fn a_generation_interrupted_before_selection_recovers_to_the_old_snapshot() {
    assert_interruptions_recover(|point| !matches!(point, Point::AfterSelection | Point::BeforeCleanup));
}

#[test]
fn a_generation_interrupted_after_selection_recovers_to_the_new_snapshot() {
    assert_interruptions_recover(|point| matches!(point, Point::AfterSelection | Point::BeforeCleanup));
}

fn assert_interruptions_recover(selects: impl Fn(Point) -> bool) {
    let update = edit(&fleet_text(PlatformId::Discord), "  value: 2000\n", "  value: 4000\n");
    let updates = BTreeMap::from([(PlatformId::Discord, update)]);

    // The expected new snapshot, from a clean run.
    let reference = Repo::new();
    reference.generate().expect("generate");
    let old_tree = reference.tree();
    reference.generate_with(&updates, Options::default()).expect("update");
    let new_tree = reference.tree();
    assert_ne!(old_tree, new_tree);
    let artifacts = 7; // five documents, catalog, summary

    for point in interruption_points(artifacts).into_iter().filter(|point| selects(*point)) {
        let repo = Repo::new();
        repo.generate().expect("generate");
        let error = repo
            .generate_with(&updates, Options { interrupt_at: Some(point), ..Options::default() })
            .expect_err("interrupted");
        assert!(matches!(error, GenerateError::Publish(PublishError::Interrupted(at)) if at == point), "{point}: {error}");

        let selected_new = matches!(point, Point::AfterSelection | Point::BeforeCleanup);
        // A verified read sees one whole snapshot or refuses; never a mixture.
        match read_verified(repo.workspace()) {
            Ok(verified) => {
                let expected = if selected_new { &new_tree } else { &old_tree };
                for (path, bytes) in &verified.files {
                    assert_eq!(Some(bytes), expected.get(path), "{point}: {path}");
                }
            }
            Err(PublishError::Inconsistent { .. }) => {
                assert!(matches!(point, Point::MidReplacement(_) | Point::BeforeManifest), "{point}: refused");
            }
            Err(other) => panic!("{point}: {other}"),
        }

        let journal_pending = publish::pending(repo.workspace());
        assert_eq!(
            journal_pending,
            !matches!(point, Point::BeforeStaging | Point::DuringStaging | Point::StagedNoJournal | Point::BeforeCleanup),
            "{point}"
        );
        if journal_pending {
            assert!(matches!(repo.generate(), Err(GenerateError::Publish(PublishError::RecoveryRequired))), "{point}");
            assert!(matches!(check(&repo.loader, &today()), Err(GenerateError::Publish(PublishError::RecoveryRequired))), "{point}");
        }

        let outcome = recover(repo.workspace(), Options::default()).expect("recover");
        let expected_outcome = match point {
            Point::BeforeSelection | Point::MidReplacement(_) | Point::BeforeManifest => Recovery::RolledBack,
            Point::AfterSelection => Recovery::RolledForward,
            _ => Recovery::Clean,
        };
        assert_eq!(outcome, expected_outcome, "{point}");
        assert_eq!(repo.tree(), if selected_new { new_tree.clone() } else { old_tree.clone() }, "{point}");
        assert!(repo.state_is_clean(), "{point}: leftovers");
        let leftovers: Vec<String> = repo.tree().into_keys().filter(|path| path.ends_with(".publish-tmp")).collect();
        assert!(leftovers.is_empty(), "{point}: {leftovers:?}");

        // A follow-up generation succeeds and converges on the new snapshot.
        repo.generate_with(&updates, Options::default()).expect("follow-up");
        assert_eq!(repo.tree(), new_tree, "{point}");
    }
}

#[test]
fn an_interrupted_recovery_is_repaired_by_running_recovery_again() {
    let update = edit(&fleet_text(PlatformId::Discord), "  value: 2000\n", "  value: 4000\n");
    let updates = BTreeMap::from([(PlatformId::Discord, update)]);
    for (crash, recovery_crash, expected) in [
        (Point::MidReplacement(3), Point::MidReplacement(2), Recovery::RolledBack),
        (Point::AfterSelection, Point::MidReplacement(1), Recovery::RolledForward),
    ] {
        let repo = Repo::new();
        repo.generate().expect("generate");
        let old_tree = repo.tree();
        repo.generate_with(&updates, Options { interrupt_at: Some(crash), ..Options::default() }).expect_err("crash");
        let selected = repo.read(MANIFEST);
        let error = recover(repo.workspace(), Options { interrupt_at: Some(recovery_crash), ..Options::default() });
        if expected == Recovery::RolledBack {
            assert!(matches!(error, Err(PublishError::Interrupted(_))), "{error:?}");
        }
        assert!(publish::pending(repo.workspace()) || expected == Recovery::RolledForward);
        let again = recover(repo.workspace(), Options::default()).expect("recover again");
        assert!(again == expected || again == Recovery::Clean, "{again:?}");
        if expected == Recovery::RolledBack {
            assert_eq!(repo.tree(), old_tree);
        } else {
            assert_eq!(repo.read(MANIFEST), selected, "roll-forward keeps the selected manifest");
            assert!(read_verified(repo.workspace()).is_ok());
        }
        assert!(repo.state_is_clean());
    }
}

#[test]
fn an_interrupted_initial_publication_rolls_back_to_no_snapshot() {
    for point in [Point::BeforeSelection, Point::MidReplacement(4), Point::BeforeManifest] {
        let repo = Repo::new();
        let before = repo.tree();
        repo.generate_with(&BTreeMap::new(), Options { interrupt_at: Some(point), ..Options::default() }).expect_err("crash");
        assert!(read_verified(repo.workspace()).is_err(), "{point}");
        assert_eq!(recover(repo.workspace(), Options::default()).expect("recover"), Recovery::RolledBack);
        assert_eq!(repo.tree(), before, "{point}: new artifacts are removed");
        assert!(matches!(read_verified(repo.workspace()), Err(PublishError::NoSnapshot)));
    }
}

#[test]
fn recovery_refuses_a_manifest_matching_neither_side_of_the_journal() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    let update = edit(&fleet_text(PlatformId::Discord), "  value: 2000\n", "  value: 4000\n");
    repo.generate_with(&BTreeMap::from([(PlatformId::Discord, update)]), Options { interrupt_at: Some(Point::BeforeManifest), ..Options::default() })
        .expect_err("crash");
    repo.write(MANIFEST, "{\"hand\": \"edited\"}\n");
    assert!(matches!(recover(repo.workspace(), Options::default()), Err(PublishError::Corrupt(_))));
    assert!(publish::pending(repo.workspace()), "the journal is kept for a human decision");
}

#[test]
fn a_manifest_naming_paths_outside_the_repository_is_refused() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    for hostile in ["../outside.txt", "/etc/hosts", "C:/Windows/win.ini", "messenger\\docs\\x"] {
        let manifest = String::from_utf8(repo.read(MANIFEST).unwrap()).unwrap();
        let mut value: Value = serde_json::from_str(&manifest).unwrap();
        value["artifacts"][0]["path"] = Value::from(hostile);
        repo.write(MANIFEST, &serde_json::to_string_pretty(&value).unwrap());
        assert!(matches!(read_verified(repo.workspace()), Err(PublishError::Corrupt(_))), "{hostile}");
        assert!(matches!(repo.generate(), Err(GenerateError::Publish(PublishError::Corrupt(_)))), "{hostile}");
        fs::write(repo.path(MANIFEST), manifest).unwrap();
    }
}

// ---- catalog and report ------------------------------------------------------

#[test]
fn the_catalog_retains_unknowns_ineligibility_provenance_and_freshness() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    let catalog = catalog_json(&repo);
    assert_eq!(catalog["generated"], "Generated by `messenger research generate`; do not edit.");
    assert_eq!(catalog["schema"]["version"], 1);
    let platforms: Vec<&str> = catalog["platforms"].as_array().unwrap().iter().map(|p| p["platform_id"].as_str().unwrap()).collect();
    assert_eq!(platforms, ["discord", "signal", "slack", "telegram", "whatsapp"], "sorted by stable key");

    let discord = &catalog["platforms"][0];
    assert_eq!(discord["document"]["path"], "messenger/docs/research/platforms/discord.md");
    assert!(discord["document"]["frontmatter_hash"].as_str().unwrap().len() == 16);
    assert_eq!((discord["last_updated"].as_str(), discord["refresh_due"].as_str()), (Some("2026-09-10"), Some("2026-10-10")));
    let constraints = discord["constraints"].as_array().unwrap();
    let find = |id: &str| constraints.iter().find(|c| c["id"] == id).unwrap();
    assert_eq!(find("c.discord.content.max")["eligibility"], "eligible");
    assert_eq!(find("c.discord.content.max")["unit"], "unicode_scalars");
    assert_eq!(find("c.discord.embed.description")["eligibility"], "ineligible");
    assert_eq!(find("c.discord.embed.description")["reasons"][0]["reason"], "unresolved_unit");
    assert_eq!(find("c.discord.embed.total")["reasons"][0]["reason"], "unresolved_unit");

    // Every fact is listed with its fingerprint and the key-sorted record.
    let fact = discord["facts"].as_array().unwrap().iter().find(|f| f["id"] == "c.discord.content.max").unwrap();
    assert_eq!(fact["state"], "known");
    assert_eq!(fact["evidence"][0], "src.discord.docs");
    assert!(fact["fingerprint"].as_str().unwrap().starts_with("xxh64:"));
    let keys: Vec<&String> = fact["record"].as_object().unwrap().keys().collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);

    // Coverage keeps investigated gaps explicit, never "unrestricted".
    let webhook = discord["coverage"].as_array().unwrap().iter().find(|c| c["interface"] == "discord_webhook").unwrap();
    assert!(webhook["categories"].as_array().unwrap().iter().all(|c| c["state"] == "investigated_gap" && c["gap"] == "gap.discord.scope"));

    // The manifest binds the catalog to its inputs and the schema.
    let manifest: Value = serde_json::from_slice(&repo.read(MANIFEST).unwrap()).unwrap();
    let inputs: Vec<&str> = manifest["inputs"].as_array().unwrap().iter().map(|i| i["path"].as_str().unwrap()).collect();
    for input in ["messenger/docs/platforms.yaml", "messenger/docs/research/platforms/_schema.yaml", "messenger/docs/research/platforms/_types.yaml", "messenger/docs/research/platforms/_fleet.md", "messenger/docs/research/platforms/discord.md"] {
        assert!(inputs.contains(&input), "{input} missing from {inputs:?}");
    }
    let discord_input = manifest["inputs"].as_array().unwrap().iter().find(|i| i["path"] == "messenger/docs/research/platforms/discord.md").unwrap();
    assert_eq!(discord_input["frontmatter"], discord["document"]["frontmatter_hash"]);
    assert_eq!(manifest["schema"], catalog["schema"]);
}

#[test]
fn a_changed_schema_invalidates_the_published_projection() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    let types = fs::read_to_string(repo.path("messenger/docs/research/platforms/_types.yaml")).unwrap();
    repo.write("messenger/docs/research/platforms/_types.yaml", &format!("{types}\n# a comment changes the fingerprint\n"));
    let Some(Drift::Changed { paths }) = check(&repo.loader, &today()).expect("check") else { panic!("expected drift") };
    assert!(paths.contains(&CATALOG.to_string()) && paths.contains(&MANIFEST.to_string()), "{paths:?}");
}

#[test]
fn reports_hand_off_every_emitted_surface_with_an_enforceability_reason() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    let catalog = published_catalog(repo.workspace()).expect("catalog");
    let report = report::build(&catalog, &Filter::default(), &today());

    let adapters: Vec<String> = report.truncation.iter().map(|h| h.adapter.to_string()).collect();
    assert_eq!(adapters, ["discord", "discord-webhook", "slack", "slack-webhook", "telegram", "whatsapp", "signal"]);
    let handoff = |adapter: &str| report.truncation.iter().find(|h| h.adapter.to_string() == adapter).unwrap();

    let discord = handoff("discord");
    assert_eq!(discord.interface.as_deref(), Some("discord_bot_api"));
    let surface = |name: &str| discord.surfaces.iter().find(|s| s.surface.as_str() == name).unwrap();
    assert_eq!(surface("body").enforceability, Enforceability::Enforceable);
    let Enforceability::NotEnforceable { reasons } = &surface("rich_description").enforceability else { panic!() };
    assert_eq!(reasons, &vec!["c.discord.embed.description: unresolved_unit".to_string(), "c.discord.embed.total: unresolved_unit".to_string()]);
    assert!(surface("rich_description").constraints.iter().any(|c| c.id == "c.discord.embed.total" && c.aggregate));
    assert!(matches!(
        &surface("filename").enforceability,
        Enforceability::NoResearchedBound { gap: None, coverage: Some(_) }
    ));

    let webhook = handoff("discord-webhook");
    for surface in &webhook.surfaces {
        let Enforceability::NoResearchedBound { gap, .. } = &surface.enforceability else { panic!("{:?}", surface) };
        assert_eq!(gap.as_deref(), Some("gap.discord.scope"), "never unlimited");
    }
    assert!(webhook.unresolved.iter().any(|q| q.starts_with("body: no researched bound")));

    let slack = handoff("slack");
    assert_eq!(slack.service_side_loss.len(), 1);
    assert_eq!(slack.service_side_loss[0].id, "c.slack.text.truncation");
    assert_eq!(slack.service_side_loss[0].overflow_behavior, "truncate");

    for handoff in &report.diagnostics {
        assert!(handoff.envelopes.is_empty());
        assert!(handoff.unresolved.iter().any(|q| q.starts_with("no researched response envelope")), "{handoff:?}");
        assert!(handoff.unresolved.iter().any(|q| q.ends_with("error handling is unassessed")));
    }

    // Freshness is decided by `today`; the boundary day is stale.
    let stale = |day: &str| report::build(&catalog, &Filter::default(), &Date::parse(day).unwrap()).freshness.iter().all(|f| f.stale);
    assert!(!stale("2026-10-09"));
    assert!(stale("2026-10-10"));
}

#[test]
fn report_filters_narrow_rows_and_handoffs() {
    let repo = Repo::new();
    repo.generate().expect("generate");
    let catalog = published_catalog(repo.workspace()).expect("catalog");
    let constraints = |filter: Filter| -> Vec<String> {
        report::build(&catalog, &filter, &today())
            .sections
            .into_iter()
            .find(|s| s.kind == report::SectionKind::Constraints)
            .unwrap()
            .rows
            .into_iter()
            .map(|row| row.id)
            .collect()
    };
    assert_eq!(constraints(Filter::default()).len(), 4);
    assert_eq!(constraints(Filter { platform: Some(PlatformId::Slack), ..Filter::default() }), ["c.slack.text.truncation"]);
    assert_eq!(constraints(Filter { operation: Some("create_message".to_string()), ..Filter::default() }).len(), 3);
    assert!(constraints(Filter { interface: Some("discord_webhook".to_string()), ..Filter::default() }).is_empty());

    let report = report::build(&catalog, &Filter { interface: Some("discord_webhook".to_string()), ..Filter::default() }, &today());
    assert_eq!(report.truncation.len(), 1);
    assert_eq!(report.truncation[0].adapter.to_string(), "discord-webhook");
    assert!(report.coverage.iter().all(|row| row.interface == "discord_webhook"));

    let report = report::build(&catalog, &Filter { platform: Some(PlatformId::Discord), ..Filter::default() }, &today());
    assert_eq!(report.truncation.len(), 2);
    assert_eq!(report.freshness.len(), 1);
}
