//! Refresh and review lifecycle (feature `research`): selection, prepared
//! inputs, stage checks, the approval policy, promotion with review records
//! and CHANGELOG, bounded recovery, budget exhaustion, interrupted promotion,
//! rejection, and retention. A fake agent writes each pass's outputs into the
//! prepared run; nothing launches Claudine or reaches the network. Every test
//! runs in a throwaway repository holding copies of the shipped roster,
//! schemas, and fleet prompt plus the accepted-fleet fixture.
#![cfg(feature = "research")]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use messenger::research::generate::{self, GenerateError, check, generate};
use messenger::research::model::{Date, PlatformId};
use messenger::research::publish::{self, Options, Point, read_verified};
use messenger::research::refresh::check::check_run;
use messenger::research::refresh::cleanup;
use messenger::research::refresh::prepare::{self, PreparedRun};
use messenger::research::refresh::promote::{self, Request as Approve};
use messenger::research::refresh::records::unsafe_content;
use messenger::research::refresh::review::{CHANGELOG, ReviewRecord};
use messenger::research::refresh::select::{self, Reason, Request, Skip};
use messenger::research::refresh::state::{RunRecord, StageStatus, StateError};
use messenger::research::refresh::{DecisionReason, InputError, Maintainer, RefreshError, RunLimits, RunStatus, Stage, StateArea};
use messenger::research::{Loader, Workspace};
use serde_json::{Value, json};
use tempfile::TempDir;

fn lib_dir() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
}

fn repo_root() -> PathBuf {
    lib_dir().parent().and_then(Path::parent).expect("repository root").to_path_buf()
}

fn day(text: &str) -> Date {
    Date::parse(text).expect("date")
}

const SHIPPED: &[&str] = &[
    "messenger/docs/platforms.yaml",
    "messenger/docs/platforms.schema.yaml",
    "messenger/docs/research/platforms/_schema.yaml",
    "messenger/docs/research/platforms/_types.yaml",
    "messenger/docs/research/platforms/_overrides.schema.yaml",
    "messenger/docs/research/platforms/_fleet.md",
    "messenger/docs/research/platforms/_rules.md",
    "messenger/docs/research/implementation/_schema.yaml",
];

const ROSTER: &str = "messenger/docs/platforms.yaml";
const MANIFEST: &str = "messenger/docs/research/publication.json";
const CATALOG: &str = "messenger/docs/research/platforms/catalog.json";
const LIMITS: RunLimits = RunLimits { max_seconds: 600, max_invocations: 8 };
const SUGGESTED: &str = "https://community.example.com/threads/limits";

struct Repo {
    dir: TempDir,
    loader: Loader,
}

impl Repo {
    /// Shipped inputs plus the five fleet fixtures at the fixed paths; no snapshot.
    fn new() -> Self {
        let repo = Self::bare();
        for platform in PlatformId::ALL {
            repo.write(&document(*platform), &fleet_text(*platform));
        }
        repo
    }

    /// Shipped inputs only: no accepted document anywhere.
    fn bare() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        for path in SHIPPED {
            let target = dir.path().join(path);
            fs::create_dir_all(target.parent().expect("parent")).expect("mkdir");
            fs::copy(repo_root().join(path), target).expect("copy shipped input");
        }
        Self { loader: Loader::new(Workspace::new(dir.path()).expect("absolute")), dir }
    }

    /// The fleet fixture published as a snapshot (no review records).
    fn published() -> Self {
        let repo = Self::new();
        generate(&repo.loader, &BTreeMap::new(), &day("2026-09-17"), Options::default()).expect("initial generation");
        repo
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root().to_path_buf(), |acc, segment| acc.join(segment))
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, content).expect("write");
    }

    fn read(&self, relative: &str) -> Option<String> {
        fs::read_to_string(self.path(relative)).ok()
    }

    fn state(&self) -> StateArea {
        StateArea::new(self.loader.workspace())
    }

    /// Every committed file under `messenger/` (the state area excluded).
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

    fn prepare(&self, platform: PlatformId, today: &Date) -> PreparedRun {
        let request = Request { forced: [platform].into(), ..Request::default() };
        let mut prepared = prepare::prepare(&self.loader, &[platform], LIMITS, today, &request).expect("prepare");
        assert_eq!(prepared.runs.len(), 1, "{:?}", prepared.selections);
        prepared.runs.remove(0)
    }

    fn run(&self, run: &PreparedRun) -> RunRecord {
        self.state().load(run.run_id.as_str()).expect("run record")
    }

    fn run_dir(&self, run: &PreparedRun) -> PathBuf {
        self.path(&run.run_dir)
    }

    fn curated(&self, platform: PlatformId) -> Vec<String> {
        let roster = self.loader.load_roster(&self.loader.workspace().roster()).expect("roster").record.expect("typed");
        roster.platform(platform).expect("platform").curated_sources.iter().map(|s| s.url.clone()).collect()
    }

    fn select(&self, today: &str, request: &Request) -> BTreeMap<PlatformId, select::Selection> {
        select::select(&self.loader, &day(today), request)
            .expect("select")
            .into_iter()
            .map(|s| (s.platform_id, s))
            .collect()
    }
}

fn document(platform: PlatformId) -> String {
    format!("messenger/docs/research/platforms/{platform}.md")
}

fn fleet_text(platform: PlatformId) -> String {
    fs::read_to_string(lib_dir().join(format!("tests/fixtures/research/lifecycle/fleet/{platform}.md")))
        .expect("fleet fixture")
        .replace("$schema: ../../../../../../docs/research/platforms/_schema.yaml", "$schema: ./_schema.yaml")
}

fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "edit target not found: {from}");
    text.replacen(from, to, 1)
}

/// The fixture re-observed on `on`: same substance, new observation dates.
fn renewed(platform: PlatformId, on: &str) -> String {
    fleet_text(platform)
        .replace("last_updated: 2026-09-10", &format!("last_updated: {on}"))
        .replace("retrieved: 2026-09-10", &format!("retrieved: {on}"))
}

/// Discord with its content limit raised and the change recorded.
fn raised(on: &str) -> String {
    let text = edit(&renewed(PlatformId::Discord, on), "  value: 2000\n", "  value: 4000\n");
    edit(
        &text,
        "changes: []\n",
        "changes:\n- id: chg.discord.content\n  kind: changed\n  facts:\n  - c.discord.content.max\n  evidence:\n  - src.discord.docs\n  summary: The documented content limit was re-checked.\n",
    )
}

fn fixture_source_url(platform: PlatformId) -> String {
    format!("https://docs.example.com/{platform}/messages")
}

/// The fake agent: writes each pass's outputs where the prepared inputs say.
struct Agent<'a> {
    repo: &'a Repo,
    run: &'a PreparedRun,
}

impl Agent<'_> {
    fn dir(&self) -> PathBuf {
        self.repo.run_dir(self.run)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = relative.split('/').fold(self.dir(), |acc, s| acc.join(s));
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, text).expect("write output");
    }

    fn discovery(&self) {
        self.write("outputs/discovery.md", "# Discovery\n\nFindings with sources.\n");
        self.write(
            "outputs/suggested-sources.json",
            &json!({ "suggestions": [{ "url": SUGGESTED, "questions": ["2"], "contribution": "Reports of undocumented limits." }] }).to_string(),
        );
    }

    /// Checks every curated source and the fixture's own source on `on`.
    fn checks(&self, on: &str, inaccessible: &[&str]) -> Value {
        let platform = self.run.platform_id;
        let mut urls = self.repo.curated(platform);
        urls.push(fixture_source_url(platform));
        let checks: Vec<Value> = urls
            .iter()
            .map(|url| {
                if inaccessible.contains(&url.as_str()) {
                    json!({ "url": url, "checked_on": on, "outcome": "inaccessible", "failure": "HTTP 403 without authentication" })
                } else {
                    json!({ "url": url, "checked_on": on, "outcome": "checked", "finding": "Limits section reviewed." })
                }
            })
            .collect();
        json!({ "checks": checks })
    }

    fn reconcile(&self, candidate: &str, checks: &Value) {
        self.write(&format!("candidate/{}.md", self.run.platform_id), candidate);
        self.write("source-checks.json", &checks.to_string());
    }

    fn sources(&self, add: bool) {
        let retain: Vec<Value> = self
            .repo
            .curated(self.run.platform_id)
            .iter()
            .map(|url| json!({ "url": url, "interfaces": [first_interface(self.run.platform_id)], "contribution": "Still the primary reference." }))
            .collect();
        let add: Vec<Value> = if add {
            vec![json!({ "url": SUGGESTED, "interfaces": [first_interface(self.run.platform_id)], "contribution": "Undocumented limit reports.",
                         "replaces": null, "coverage_gained": null, "coverage_lost": null })]
        } else {
            Vec::new()
        };
        self.write("outputs/source-proposal.json", &json!({ "retain": retain, "add": add, "remove": [] }).to_string());
    }

    /// Supports every changed fact and gap; leaves re-observed sources and
    /// prose unresolved, as an honest reviewer would without new evidence.
    fn review(&self) {
        let delta: Value = serde_json::from_slice(&fs::read(self.dir().join("delta.json")).expect("delta.json")).expect("delta json");
        let evidence = json!([format!("src.{}.docs", self.run.platform_id)]);
        let mut conclusions = Vec::new();
        for (key, kind, verdict) in [("facts", "fact", "supported"), ("sources", "source", "unresolved"), ("gaps", "gap", "supported")] {
            for item in delta[key].as_array().expect("array") {
                conclusions.push(json!({ "kind": kind, "subject": item["id"], "verdict": verdict, "evidence": evidence, "finding": "Compared with the cited limits section." }));
            }
        }
        if delta["prose"]["changed"] == json!(true) {
            conclusions.push(json!({ "kind": "prose", "subject": "body", "verdict": "unresolved", "evidence": [], "finding": "Prose restates the typed facts." }));
        }
        self.write("outputs/evidence-review.json", &json!({ "reviewer": "fake-reviewer", "conclusions": conclusions }).to_string());
    }

    /// Runs every pass and both checks; returns the final check.
    fn complete(&self, candidate: &str, checks: &Value, add_source: bool, today: &Date) -> messenger::research::refresh::check::CheckReport {
        self.discovery();
        self.reconcile(candidate, checks);
        self.sources(add_source);
        let validation = check_run(&self.repo.loader, self.run.run_id.as_str(), Stage::Validation, today).expect("check validation");
        assert!(validation.passed, "{:#?}", validation.stages);
        assert_eq!(validation.status, RunStatus::Active);
        self.review();
        check_run(&self.repo.loader, self.run.run_id.as_str(), Stage::Review, today).expect("check review")
    }
}

fn first_interface(platform: PlatformId) -> &'static str {
    match platform {
        PlatformId::Discord => "discord_bot_api",
        PlatformId::Slack => "slack_web_api",
        PlatformId::Telegram => "telegram_bot_api",
        PlatformId::WhatsApp => "whatsapp_cloud_api",
        PlatformId::Signal => "signal_cli_jsonrpc",
    }
}

fn approve(repo: &Repo, runs: &[&PreparedRun], today: &str) -> Result<Vec<promote::Promoted>, RefreshError> {
    approve_as(repo, runs, today, "maintainer")
}

fn approve_as(repo: &Repo, runs: &[&PreparedRun], today: &str, by: &str) -> Result<Vec<promote::Promoted>, RefreshError> {
    let ids: Vec<String> = runs.iter().map(|run| run.run_id.to_string()).collect();
    let by = Maintainer::new(by).expect("a named maintainer");
    promote::promote(&repo.loader, &ids, &Approve::Human { by }, &day(today), Options::default())
}

fn reject(repo: &Repo, run_id: &str, by: &str, reason: &str, on: &Date) -> Result<RunRecord, RefreshError> {
    let by = Maintainer::new(by).expect("a named maintainer");
    let reason = DecisionReason::new(reason).expect("a stated reason");
    promote::reject(&repo.loader, run_id, &by, &reason, on)
}

fn renew(repo: &Repo, run: &PreparedRun, today: &str) -> Result<Vec<promote::Promoted>, RefreshError> {
    promote::promote(&repo.loader, &[run.run_id.to_string()], &Approve::Renewal, &day(today), Options::default())
}

/// Human-approves a raised Discord limit on `on`, giving Discord a review record.
fn reviewed_discord(repo: &Repo, on: &str) -> PreparedRun {
    let run = repo.prepare(PlatformId::Discord, &day(on));
    let agent = Agent { repo, run: &run };
    let report = agent.complete(&raised(on), &agent.checks(on, &[]), false, &day(on));
    assert!(report.passed, "{:#?}", report.stages);
    approve(repo, &[&run], on).expect("human approval");
    run
}

fn reasons(error: RefreshError) -> Vec<String> {
    match error {
        RefreshError::NotEligible { reasons, .. } => reasons,
        other => panic!("expected NotEligible, got {other}"),
    }
}

// ---- selection ------------------------------------------------------------------

#[test]
fn selection_names_every_due_reason_and_skips_current_or_open_platforms() {
    let empty = Repo::new();
    for selection in empty.select("2026-09-17", &Request::default()).values() {
        assert_eq!(selection.due, vec![Reason::Missing], "no snapshot means no accepted document");
    }

    let repo = Repo::published();
    let before = repo.select("2026-09-17", &Request::default());
    assert!(before.values().all(|s| s.due == vec![Reason::NoReviewRecord]), "{before:#?}");

    reviewed_discord(&repo, "2026-09-18");
    let now = repo.select("2026-09-18", &Request::default());
    assert_eq!(
        now[&PlatformId::Discord].skip,
        Some(Skip::Current { last_updated: day("2026-09-18"), refresh_due: day("2026-10-18") })
    );
    assert!(now[&PlatformId::Discord].due.is_empty());
    assert_eq!(repo.select("2026-10-18", &Request::default())[&PlatformId::Discord].due, vec![Reason::Expired { refresh_due: day("2026-10-18") }]);
    let forced = Request { forced: [PlatformId::Discord].into(), ..Request::default() };
    assert_eq!(repo.select("2026-09-18", &forced)[&PlatformId::Discord].due, vec![Reason::Forced]);
    let all = Request { force_all: true, ..Request::default() };
    assert!(repo.select("2026-09-18", &all).values().all(|s| s.due.contains(&Reason::Forced)));
    let observed = Request { observed_versions: [("discord_bot_api".to_string(), "v11".to_string())].into(), ..Request::default() };
    assert_eq!(
        repo.select("2026-09-18", &observed)[&PlatformId::Discord].due,
        vec![Reason::VersionChanged { interface: "discord_bot_api".to_string(), observed: "v11".to_string() }]
    );
    // An observed version of another platform's interface is ignored.
    let foreign = Request { observed_versions: [("slack_web_api".to_string(), "v2".to_string())].into(), ..Request::default() };
    assert!(repo.select("2026-09-18", &foreign)[&PlatformId::Discord].due.is_empty());

    let fleet = repo.read("messenger/docs/research/platforms/_fleet.md").expect("fleet prompt");
    repo.write("messenger/docs/research/platforms/_fleet.md", &format!("{fleet}\nOne more instruction.\n"));
    assert_eq!(repo.select("2026-09-18", &Request::default())[&PlatformId::Discord].due, vec![Reason::PromptChanged]);
    repo.write("messenger/docs/research/platforms/_fleet.md", &fleet);
    let schema = repo.read("messenger/docs/research/platforms/_schema.yaml").expect("schema");
    repo.write("messenger/docs/research/platforms/_schema.yaml", &format!("{schema}# a comment changes the fingerprint\n"));
    assert_eq!(repo.select("2026-09-18", &Request::default())[&PlatformId::Discord].due, vec![Reason::SchemaChanged]);
    repo.write("messenger/docs/research/platforms/_schema.yaml", &schema);

    let open = repo.prepare(PlatformId::Slack, &day("2026-09-18"));
    assert_eq!(
        repo.select("2026-09-18", &Request::default())[&PlatformId::Slack].skip,
        Some(Skip::OpenRun { run_id: open.run_id.clone(), status: RunStatus::Active })
    );
    let prepared = prepare::prepare(&repo.loader, &[PlatformId::Slack], LIMITS, &day("2026-09-18"), &Request::default()).expect("prepare");
    assert!(prepared.runs.is_empty(), "an open run blocks a second run for the platform");
}

/// The run directories under `runs/<platform>/`.
fn run_dirs(repo: &Repo, platform: PlatformId) -> Vec<String> {
    let Ok(entries) = fs::read_dir(repo.path(&format!("messenger/.research-state/runs/{platform}"))) else { return Vec::new() };
    let mut names: Vec<String> = entries.flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

/// Review-1 regression: selection once dropped unreadable run records, so a
/// truncated `run.json` for an active run let `prepare` start a second run
/// with a fresh budget.
#[test]
fn an_unreadable_run_record_blocks_its_platform_and_names_its_path() {
    let on = day("2026-09-18");
    let forced = Request { forced: [PlatformId::Discord].into(), ..Request::default() };
    for (name, corrupt) in [("truncated", "{\n  \"format\": \"messenger-research-run/1\",\n  \"run_"), ("invalid JSON", "not json at all\n")] {
        let repo = Repo::published();
        let open = repo.prepare(PlatformId::Discord, &on);
        repo.write(&format!("{}/run.json", open.run_dir), corrupt);

        let selection = repo.select("2026-09-18", &forced).remove(&PlatformId::Discord).expect("discord");
        let Some(Skip::UnreadableRun { path, error }) = &selection.skip else { panic!("{name}: {selection:?}") };
        assert_eq!(path, &open.run_dir, "{name}: the repository-relative run directory");
        assert!(error.contains(&format!("{}/run.json", open.run_dir)), "{name}: {error}");
        assert!(selection.due.is_empty(), "{name}: a blocked platform is never due");

        let prepared = prepare::prepare(&repo.loader, &[PlatformId::Discord], LIMITS, &on, &forced).expect(name);
        assert!(prepared.runs.is_empty(), "{name}: {:?}", prepared.runs);
        assert!(matches!(prepared.selections[0].skip, Some(Skip::UnreadableRun { .. })), "{name}");
        assert_eq!(run_dirs(&repo, PlatformId::Discord), vec![open.run_id.to_string()], "{name}: no second run or ledger");
        // The block is per platform: another platform still prepares.
        assert_eq!(repo.prepare(PlatformId::Slack, &on).platform_id, PlatformId::Slack, "{name}");
    }

    // A run directory whose record never landed, or that holds another
    // platform's record, blocks the platform whose directory it sits in.
    let repo = Repo::published();
    let open = repo.prepare(PlatformId::Discord, &on);
    let stray = format!("messenger/.research-state/runs/telegram/{}", open.run_id);
    repo.write(&format!("{stray}/run.json"), &repo.read(&format!("{}/run.json", open.run_dir)).expect("record"));
    fs::create_dir_all(repo.path("messenger/.research-state/runs/signal/2026-09-18-00000000")).expect("mkdir");
    let selections = repo.select("2026-09-18", &Request { force_all: true, ..Request::default() });
    assert!(matches!(&selections[&PlatformId::Telegram].skip, Some(Skip::UnreadableRun { path, .. }) if *path == stray));
    assert!(matches!(&selections[&PlatformId::Signal].skip, Some(Skip::UnreadableRun { .. })));
    assert!(matches!(&selections[&PlatformId::Discord].skip, Some(Skip::OpenRun { .. })));
}

/// Review-1 regression: selection and run creation were not atomic, so two
/// preparations could each see no open run and each create one.
#[test]
fn concurrent_preparations_leave_exactly_one_open_run() {
    let repo = Repo::published();
    let barrier = std::sync::Barrier::new(2);
    let outcomes: Vec<Result<prepare::Prepared, RefreshError>> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    let loader = Loader::new(Workspace::new(repo.root()).expect("absolute"));
                    let request = Request { forced: [PlatformId::Discord].into(), ..Request::default() };
                    barrier.wait();
                    prepare::prepare(&loader, &[PlatformId::Discord], LIMITS, &day("2026-09-18"), &request)
                })
            })
            .collect();
        workers.into_iter().map(|worker| worker.join().expect("worker")).collect()
    });
    assert_eq!(run_dirs(&repo, PlatformId::Discord).len(), 1, "{outcomes:?}");
    let created = outcomes.iter().filter(|outcome| matches!(outcome, Ok(prepared) if prepared.runs.len() == 1)).count();
    assert_eq!(created, 1, "{outcomes:?}");
    // The other either ran first into the lock or ran after and saw the open run.
    assert!(outcomes.iter().all(|outcome| match outcome {
        Ok(prepared) => prepared.runs.len() == 1 || matches!(prepared.selections[0].skip, Some(Skip::OpenRun { .. })),
        Err(error) => matches!(error, RefreshError::PrepareBusy { .. }),
    }));
}

#[test]
fn a_held_prepare_lock_refuses_preparation_and_a_released_one_never_lingers() {
    let repo = Repo::published();
    let path = repo.state().prepare_lock();
    fs::create_dir_all(path.parent().expect("runs dir")).expect("mkdir");
    let holder = fs::File::create(&path).expect("lock file");
    holder.try_lock().expect("hold the lock");
    let request = Request { forced: [PlatformId::Discord].into(), ..Request::default() };
    let refused = prepare::prepare(&repo.loader, &[PlatformId::Discord], LIMITS, &day("2026-09-18"), &request).expect_err("busy");
    assert!(
        matches!(&refused, RefreshError::PrepareBusy { path } if path == "messenger/.research-state/runs/prepare.lock"),
        "{refused}"
    );
    assert!(run_dirs(&repo, PlatformId::Discord).is_empty());
    // A holder that exits (or crashes) releases the OS lock; the file stays.
    drop(holder);
    assert!(path.exists());
    repo.prepare(PlatformId::Discord, &day("2026-09-18"));
}

#[test]
fn an_accepted_document_that_no_longer_validates_is_due() {
    let repo = Repo::published();
    reviewed_discord(&repo, "2026-09-18");
    // Removing a companion interface from the roster leaves the accepted
    // document describing an interface the roster no longer has.
    let roster = repo.read(ROSTER).expect("roster");
    let start = roster.find("          - interface_id: discord_gateway").expect("gateway");
    let end = roster[start..].find("          - interface_id: discord_interactions").expect("interactions") + start;
    repo.write(ROSTER, &format!("{}{}", &roster[..start], &roster[end..]));
    let due = repo.select("2026-09-18", &Request::default())[&PlatformId::Discord].due.clone();
    assert!(matches!(due.as_slice(), [Reason::SchemaInvalid { findings }] if *findings > 0), "{due:?}");
}

/// A published document whose refresh date moves past 9999-12-31 when the
/// roster interval grows is due as invalid, never `Expired` or `Current`
/// with a five-digit-year date.
#[test]
fn a_refresh_date_past_9999_makes_the_document_invalid_not_expired() {
    let repo = Repo::new();
    let discord = document(PlatformId::Discord);
    repo.write(&discord, &edit(&fleet_text(PlatformId::Discord), "last_updated: 2026-09-10", "last_updated: 9989-12-24"));
    generate(&repo.loader, &BTreeMap::new(), &day("2026-09-17"), Options::default()).expect("a 30-day interval fits");

    let roster = repo.read(ROSTER).expect("roster");
    repo.write(ROSTER, &edit(&roster, "refresh_interval_days: 30", "refresh_interval_days: 3660"));
    let selection = repo.select("2026-09-18", &Request::default()).remove(&PlatformId::Discord).expect("discord");
    assert!(
        matches!(selection.due.as_slice(), [Reason::SchemaInvalid { findings: 1 }, Reason::NoReviewRecord]),
        "{:?}",
        selection.due
    );
    assert_eq!(selection.skip, None);
}

// ---- prepared inputs -------------------------------------------------------------

#[test]
fn discovery_inputs_hold_no_previous_research_or_curated_sources() {
    let repo = Repo::published();
    let run = repo.prepare(PlatformId::Discord, &day("2026-09-18"));
    let dir = repo.run_dir(&run);
    let curated = repo.curated(PlatformId::Discord);
    let read_all = |pass: &str| -> String {
        let mut text = String::new();
        for entry in fs::read_dir(dir.join("inputs").join(pass)).expect("pass dir").flatten() {
            text.push_str(&fs::read_to_string(entry.path()).expect("input"));
        }
        text
    };
    let discovery = read_all("discovery");
    for url in &curated {
        // The shared instructions may name a documentation site; the curated
        // list itself is never copied into discovery inputs.
        assert!(!discovery.contains(&format!("- {url} (")), "discovery received curated source {url}");
    }
    assert!(!discovery.contains("# Discord fleet fixture"), "discovery received the previous document");
    assert!(!dir.join("inputs/discovery/previous.md").exists());
    assert!(!dir.join("inputs/discovery/curated-sources.md").exists());
    assert!(discovery.contains("discord_bot_api") && discovery.contains("https://discord.com"), "identification is present");

    let reconcile = read_all("reconcile");
    assert!(curated.iter().all(|url| reconcile.contains(url.as_str())), "reconciliation gets every curated source");
    assert!(reconcile.contains("# Discord fleet fixture"), "reconciliation gets the previous document");
    assert!(reconcile.contains("discovery.md"), "reconciliation reads the discovery output");

    // The sequence runs each pass as its own prompt step, never inline-compose.
    let sequence = fs::read_to_string(dir.join("run.md")).expect("run.md");
    assert!(sequence.starts_with("---\nsequence:\n"), "{sequence}");
    assert!(!sequence.lines().any(|line| line.starts_with("prompt:")), "no top-level prompt");
    let steps: Vec<&str> = sequence.lines().filter_map(|line| line.trim().strip_prefix("- name: ")).collect();
    assert_eq!(steps, ["discovery", "reconcile", "sources", "validation", "review", "review-check"]);
    assert!(sequence.contains("check-run") && sequence.contains("--through validation") && sequence.contains("--through review"));
    // Both limits reach Claudine's ledger; the shared fleet lock serializes platforms.
    let init = run.commands[0].join(" ");
    assert!(init.contains("budget init") && init.contains("--max-seconds 600") && init.contains("--max-invocations 8"), "{init}");
    assert!(init.contains("--exclusive-lock ../../fleet.lock"));
    assert!(run.commands[1].join(" ").contains("sequence --yolo --budget-ledger"));
    let record = repo.run(&run);
    assert_eq!(record.selected_because, ["forced", "no_review_record"]);
    assert_eq!(record.limits, LIMITS);
    assert!(record.baseline.is_some(), "a published document is the baseline");
    // Read/write/read round trip of the persisted run record.
    let bytes = fs::read(dir.join("run.json")).expect("run.json");
    let reparsed: RunRecord = serde_json::from_slice(&bytes).expect("parse");
    repo.state().save(&reparsed).expect("save");
    assert_eq!(fs::read(dir.join("run.json")).expect("reread"), bytes, "run.json is stable across a round trip");
    assert!(!bytes.contains(&b'\r'), "LF only");
}

#[test]
fn initial_research_reconciles_legacy_prose_without_treating_it_as_a_baseline() {
    let repo = Repo::new();
    let run = repo.prepare(PlatformId::Signal, &day("2026-09-18"));
    let record = repo.run(&run);
    assert_eq!(record.baseline, None, "no published snapshot, so no baseline");
    assert!(repo.run_dir(&run).join("inputs/reconcile/previous.md").exists(), "legacy prose is still reconciliation input");
}

// ---- check-run -------------------------------------------------------------------

#[test]
fn stages_are_judged_from_their_outputs_not_exit_codes() {
    let repo = Repo::published();
    let on = day("2026-09-18");
    let run = repo.prepare(PlatformId::Discord, &on);
    let id = run.run_id.as_str();
    let agent = Agent { repo: &repo, run: &run };

    // Nothing written: every stage pending, and the run fails its check.
    let report = check_run(&repo.loader, id, Stage::Validation, &on).expect("check");
    assert!(!report.passed);
    assert_eq!(report.status, RunStatus::Failed);
    assert!(report.stages.iter().all(|s| s.status == StageStatus::Pending));

    // Malformed discovery output fails; a timestamp-only candidate fails.
    agent.write("outputs/discovery.md", "# Discovery\n");
    agent.write("outputs/suggested-sources.json", "{\"suggestions\": [{\"url\": \"x\"}]}");
    let bumped = edit(&fleet_text(PlatformId::Discord), "last_updated: 2026-09-10", "last_updated: 2026-09-18");
    let all_failed: Vec<String> = repo.curated(PlatformId::Discord).into_iter().chain([fixture_source_url(PlatformId::Discord)]).collect();
    let all_failed: Vec<&str> = all_failed.iter().map(String::as_str).collect();
    agent.reconcile(&bumped, &agent.checks("2026-09-18", &all_failed));
    agent.sources(false);
    let report = check_run(&repo.loader, id, Stage::Validation, &on).expect("check");
    let stage = |report: &messenger::research::refresh::check::CheckReport, stage: Stage| {
        report.stages.iter().find(|s| s.stage == stage).cloned().expect("stage")
    };
    assert_eq!(stage(&report, Stage::Discovery).status, StageStatus::Failed);
    assert_eq!(stage(&report, Stage::Validation).status, StageStatus::Pending, "validation waits for every earlier pass");
    assert!(repo.run(&run).stop_reason.expect("stop reason").starts_with("discovery"));
    agent.discovery();
    let report = check_run(&repo.loader, id, Stage::Validation, &on).expect("check");
    let validation = stage(&report, Stage::Validation);
    assert_eq!(validation.status, StageStatus::Failed);
    assert!(validation.findings.iter().any(|f| f.contains("timestamp bump")), "{validation:#?}");

    // Integrity: created, removal without a change record, and an observation
    // date claimed for a curated source that was inaccessible.
    let curated = repo.curated(PlatformId::Discord)[0].clone();
    let mut candidate = edit(&renewed(PlatformId::Discord, "2026-09-18"), "created: 2026-09-01", "created: 2026-09-18");
    let start = candidate.find("- id: c.discord.content.max").expect("constraint");
    let end = candidate.find("- id: c.discord.embed.description").expect("next constraint");
    candidate.replace_range(start..end, "");
    candidate = edit(
        &candidate,
        "  retrieved: 2026-09-18\n",
        &format!("  retrieved: 2026-09-18\n- id: src.discord.curated\n  kind: official_docs\n  url: {curated}\n  locator: Limits\n  retrieved: 2026-09-18\n"),
    );
    agent.reconcile(&candidate, &agent.checks("2026-09-18", &[curated.as_str()]));
    let findings = stage(&check_run(&repo.loader, id, Stage::Validation, &on).expect("check"), Stage::Validation).findings;
    let has = |needle: &str| findings.iter().any(|f| f.contains(needle));
    assert!(has("created changed from 2026-09-01 to 2026-09-18"), "{findings:#?}");
    assert!(has("c.discord.content.max was removed without a changes entry"), "{findings:#?}");
    assert!(has("claims a check of") && has("no successful check on that date"), "{findings:#?}");

    // A candidate for another platform in this platform's slot.
    agent.reconcile(&renewed(PlatformId::Telegram, "2026-09-18"), &agent.checks("2026-09-18", &[]));
    let findings = stage(&check_run(&repo.loader, id, Stage::Validation, &on).expect("check"), Stage::Validation).findings;
    assert!(findings.iter().any(|f| f.contains("describes telegram, not discord")), "{findings:#?}");

    // A valid candidate passes validation; the review must cover every change
    // and store only safe, bounded text.
    agent.reconcile(&raised("2026-09-18"), &agent.checks("2026-09-18", &[]));
    assert!(check_run(&repo.loader, id, Stage::Validation, &on).expect("check").passed);
    agent.write(
        "outputs/evidence-review.json",
        &json!({ "reviewer": "r", "conclusions": [{ "kind": "fact", "subject": "c.discord.content.max", "verdict": "supported",
                 "evidence": ["src.discord.docs"], "finding": "red \u{1b}[31m text" }] })
        .to_string(),
    );
    let report = check_run(&repo.loader, id, Stage::Review, &on).expect("check");
    let review = stage(&report, Stage::Review).findings;
    assert!(review.iter().any(|f| f.contains("unsafe to store")), "{review:#?}");
    assert!(review.iter().any(|f| f.contains("no conclusion for changed source src.discord.docs")), "{review:#?}");
    assert_eq!(report.status, RunStatus::Failed);
    agent.review();
    let report = check_run(&repo.loader, id, Stage::Review, &on).expect("check");
    assert!(report.passed, "{:#?}", report.stages);
    assert_eq!(report.status, RunStatus::AwaitingReview);
}

// ---- promotion and approval ------------------------------------------------------

#[test]
fn a_reviewed_change_publishes_with_its_review_record_and_changelog_entry() {
    let repo = Repo::published();
    let before = repo.tree();
    let run = repo.prepare(PlatformId::Discord, &day("2026-09-18"));
    let agent = Agent { repo: &repo, run: &run };
    agent.complete(&raised("2026-09-18"), &agent.checks("2026-09-18", &[]), false, &day("2026-09-18"));
    assert_eq!(repo.tree(), before, "preparing and checking a run writes only the state area");

    // Validation passed, but a changed limit is never an automatic renewal.
    let refused = reasons(renew(&repo, &run, "2026-09-18").expect_err("not a renewal"));
    assert!(refused[0].contains("not a verified unchanged renewal"), "{refused:?}");
    assert!(refused.iter().any(|r| r.contains("typed metadata changed")), "{refused:?}");
    assert_eq!(repo.tree(), before, "a refused promotion writes nothing");

    let promoted = approve(&repo, &[&run], "2026-09-18").expect("approve");
    assert_eq!(promoted[0].kind, "human");
    let review_path = promoted[0].review.clone().expect("review record");
    assert_eq!(review_path, format!("messenger/docs/research/reviews/2026-09-18-discord-{}.json", run.run_id));
    assert_eq!(repo.read(&document(PlatformId::Discord)).expect("doc"), raised("2026-09-18"));
    for platform in [PlatformId::Slack, PlatformId::Telegram, PlatformId::WhatsApp, PlatformId::Signal] {
        assert_eq!(repo.read(&document(platform)).expect("doc").as_bytes(), before[&document(platform)].as_slice(), "{platform} keeps its own dates");
    }
    assert_eq!(repo.read(ROSTER).expect("roster").as_bytes(), before[ROSTER].as_slice(), "promotion never edits the roster");
    assert!(repo.read("messenger/docs/research/implementation/mappings.yaml").is_none(), "promotion never writes mappings");
    assert_eq!(repo.run(&run).status, RunStatus::Accepted);

    let verified = read_verified(repo.loader.workspace()).expect("verified");
    assert!(verified.files.contains_key(&review_path) && verified.files.contains_key(CHANGELOG), "history is in the snapshot");
    let bytes = repo.read(&review_path).expect("review record");
    let record = ReviewRecord::parse(bytes.as_bytes()).expect("parse");
    assert_eq!(record.to_bytes(), bytes.as_bytes(), "review record round trip is byte-identical");
    assert_eq!(record.summary.facts_changed, ["c.discord.content.max"]);
    assert!(record.summary.flags.iter().any(|f| f == "raised_limit: c.discord.content.max"), "{:?}", record.summary.flags);
    assert_eq!(record.delta["conclusion_kind"], "mechanical", "mechanical flags stay separate from the reviewer");
    assert_eq!(record.evidence_review.reviewer, "fake-reviewer");
    assert!(record.summary.unresolved.iter().any(|u| u == "source src.discord.docs (unresolved)"), "unresolved stays unresolved: {:?}", record.summary.unresolved);
    let changelog = repo.read(CHANGELOG).expect("changelog");
    assert!(changelog.contains(&format!("## 2026-09-18 — discord (run {})", run.run_id)), "{changelog}");
    assert!(changelog.contains("change to the accepted baseline, approved by maintainer"));
    assert!(changelog.contains("`raised_limit: c.discord.content.max`") && changelog.contains("remaining uncertainty"));
    assert!(changelog.contains(&format!("(reviews/2026-09-18-discord-{}.json)", run.run_id)));

    // Discovery suggestions are leads only: not evidence, not curated.
    let catalog = repo.read(CATALOG).expect("catalog");
    assert!(!catalog.contains(SUGGESTED) && !repo.read(ROSTER).expect("roster").contains(SUGGESTED));

    // Stored history is safe, portable, and free of host paths and transcripts.
    let host = repo.root().display().to_string();
    for (path, bytes) in repo.tree() {
        let text = String::from_utf8_lossy(&bytes);
        if path.starts_with("messenger/docs/research/reviews/") || path == CHANGELOG || path == MANIFEST || path == CATALOG {
            assert!(!text.contains(&host), "{path} holds a host path");
            assert_eq!(unsafe_content(&text), None, "{path}");
            assert!(!text.contains('\r'), "{path} is LF-only");
        }
    }

    // Deterministic afterwards: no drift, and regeneration changes nothing.
    assert_eq!(check(&repo.loader, &day("2026-09-18")).expect("check"), None);
    let tree = repo.tree();
    let again = generate(&repo.loader, &BTreeMap::new(), &day("2026-09-18"), Options::default()).expect("regenerate");
    assert!(again.unchanged, "{again:?}");
    assert_eq!(repo.tree(), tree);
    // A second promotion of a decided run is refused.
    assert!(matches!(approve(&repo, &[&run], "2026-09-18"), Err(RefreshError::WrongStatus { status: RunStatus::Accepted, .. })));
}

#[test]
fn decisions_name_a_trimmed_maintainer_and_invalid_persisted_names_do_not_load() {
    for blank in ["", "   ", "\t\n"] {
        assert_eq!(Maintainer::new(blank), Err(InputError::BlankMaintainer), "{blank:?}");
        assert_eq!(DecisionReason::new(blank), Err(InputError::BlankReason), "{blank:?}");
    }
    assert_eq!(Maintainer::new("Ken\n- approved by mallory"), Err(InputError::ControlInMaintainer));
    let repo = Repo::published();
    let on = "2026-09-18";
    let before = repo.tree();

    let rejected = repo.prepare(PlatformId::Discord, &day(on));
    let agent = Agent { repo: &repo, run: &rejected };
    agent.complete(&raised(on), &agent.checks(on, &[]), false, &day(on));
    reject(&repo, rejected.run_id.as_str(), " Rejecting Maintainer\t", "  duplicate run \n", &day(on)).expect("reject");
    let decision = repo.run(&rejected).decision.expect("rejection decision");
    assert_eq!(decision.by.as_ref().map(Maintainer::as_str), Some("Rejecting Maintainer"));
    assert_eq!(decision.reason.as_ref().map(DecisionReason::as_str), Some("duplicate run"));
    assert_eq!(repo.tree(), before, "a rejection publishes nothing");

    let run = repo.prepare(PlatformId::Discord, &day(on));
    let agent = Agent { repo: &repo, run: &run };
    agent.complete(&raised(on), &agent.checks(on, &[]), false, &day(on));
    let promoted = approve_as(&repo, &[&run], on, "  Ken Snyder \t").expect("approve");
    let review_path = promoted[0].review.clone().expect("review record");
    let review_text = repo.read(&review_path).expect("review record");
    let record = ReviewRecord::parse(review_text.as_bytes()).expect("parse");
    assert_eq!(record.approval.by.as_str(), "Ken Snyder");
    assert!(repo.read(CHANGELOG).expect("changelog").contains("approved by Ken Snyder\n"));
    assert_eq!(repo.run(&run).decision.expect("decision").by.as_ref().map(Maintainer::as_str), Some("Ken Snyder"));

    // Reading back applies the same validation: a blank or multi-line approver
    // in a review record, or a blank name in a local run record, is refused on load.
    for (approver, expect) in [(r#""  ""#, "maintainer name is empty"), (r#""Ken\n- approved by mallory""#, "control character")] {
        let tampered = review_text.replace(r#""by": "Ken Snyder""#, &format!(r#""by": {approver}"#));
        assert_ne!(tampered, review_text, "the fixture edit applies");
        let error = ReviewRecord::parse(tampered.as_bytes()).expect_err(approver);
        assert!(error.contains(expect), "{approver}: {error}");
    }
    let run_json = repo.run_dir(&run).join("run.json");
    let run_text = fs::read_to_string(&run_json).expect("run.json");
    let blank_run = run_text.replace(r#""by": "Ken Snyder""#, r#""by": """#);
    assert_ne!(blank_run, run_text, "the fixture edit applies");
    fs::write(&run_json, blank_run).expect("write run.json");
    match repo.state().load(run.run_id.as_str()) {
        Err(StateError::Corrupt { message, .. }) => assert!(message.contains("maintainer name is empty"), "{message}"),
        other => panic!("expected a corrupt run record, got {other:?}"),
    }
}

#[test]
fn a_verified_unchanged_renewal_is_accepted_without_a_changelog_entry() {
    let repo = Repo::published();
    reviewed_discord(&repo, "2026-09-18");
    let changelog = repo.read(CHANGELOG).expect("changelog");
    let reviews = fs::read_dir(repo.path("messenger/docs/research/reviews")).expect("reviews").count();
    let accepted = repo.read(&document(PlatformId::Discord)).expect("doc");

    let run = repo.prepare(PlatformId::Discord, &day("2026-10-20"));
    let agent = Agent { repo: &repo, run: &run };
    let candidate = accepted.replace("2026-09-18", "2026-10-20");
    agent.complete(&candidate, &agent.checks("2026-10-20", &[]), false, &day("2026-10-20"));
    let promoted = renew(&repo, &run, "2026-10-20").expect("automatic renewal");
    assert_eq!(promoted[0].kind, "renewal");
    assert!(promoted[0].review.is_none());
    let renewal = promoted[0].renewal.clone().expect("local renewal record");
    assert!(renewal.starts_with("messenger/.research-state/renewals/2026-10-20-discord-"), "{renewal}");
    assert!(repo.path(&renewal).exists());
    assert_eq!(repo.read(CHANGELOG).expect("changelog"), changelog, "no no-change CHANGELOG entry");
    assert_eq!(fs::read_dir(repo.path("messenger/docs/research/reviews")).expect("reviews").count(), reviews);
    let published = repo.read(&document(PlatformId::Discord)).expect("doc");
    assert_eq!(published, candidate, "observation dates advance");
    let body = |text: &str| text.split("\n---\n").nth(1).map(str::to_string);
    assert_eq!(body(&published), body(&accepted), "accepted prose is preserved");
    let now = repo.select("2026-10-20", &Request::default());
    assert_eq!(now[&PlatformId::Discord].skip, Some(Skip::Current { last_updated: day("2026-10-20"), refresh_due: day("2026-11-19") }));
    assert_eq!(repo.run(&run).decision.expect("decision").kind, "renewal");
}

#[test]
fn automatic_renewal_is_refused_unless_everything_substantive_is_unchanged_and_rechecked() {
    let repo = Repo::published();
    reviewed_discord(&repo, "2026-09-18");
    let accepted = repo.read(&document(PlatformId::Discord)).expect("doc");
    let on = "2026-10-20";
    let dated = accepted.replace("2026-09-18", on);
    let curated = repo.curated(PlatformId::Discord)[0].clone();
    let added_source = edit(
        &dated,
        &format!("  retrieved: {on}\n"),
        &format!("  retrieved: {on}\n- id: src.discord.extra\n  kind: official_docs\n  url: https://docs.example.com/discord/extra\n  locator: Limits\n  retrieved: {on}\n"),
    );
    let prose = format!("{dated}\nA new explanatory paragraph.\n");
    let cases: [(&str, String, Vec<&str>, bool, &str); 4] = [
        ("failed check", dated.clone(), vec![curated.as_str()], false, "was inaccessible"),
        ("source added", added_source, vec![], false, "typed metadata changed"),
        ("prose changed", prose, vec![], false, "explanatory prose changed"),
        ("curated change", dated.clone(), vec![], true, "curated-source change"),
    ];
    for (name, candidate, inaccessible, add, reason) in cases {
        let run = repo.prepare(PlatformId::Discord, &day(on));
        let agent = Agent { repo: &repo, run: &run };
        let mut checks = agent.checks(on, &inaccessible);
        if name == "source added" {
            checks["checks"].as_array_mut().expect("checks").push(
                json!({ "url": "https://docs.example.com/discord/extra", "checked_on": on, "outcome": "checked", "finding": "New limits page." }),
            );
        }
        let report = agent.complete(&candidate, &checks, add, &day(on));
        assert!(report.passed, "{name}: {:#?}", report.stages);
        let refused = reasons(renew(&repo, &run, on).expect_err(name));
        assert!(refused.iter().any(|r| r.contains(reason)), "{name}: {refused:?}");
        reject(&repo, run.run_id.as_str(), "maintainer", "test case", &day(on)).expect("reject");
    }
    // A rejected run never reaches the CHANGELOG and cannot be promoted.
    let changelog = repo.read(CHANGELOG).expect("changelog");
    assert_eq!(changelog.matches("\n## ").count(), 1, "{changelog}");
    let rejected = repo.state().list().into_iter().filter_map(|(_, _, r)| r.ok()).filter(|r| r.status == RunStatus::Rejected).count();
    assert_eq!(rejected, 4);
}

/// Review-1 regression: `2026-09-31` once parsed, so an impossible `checked_on`
/// could match an equally impossible `retrieved` date. The schema already
/// refused the document's date; the source-check record now refuses its own.
#[test]
fn an_impossible_evidence_date_cannot_qualify_an_unchanged_renewal() {
    let repo = Repo::published();
    reviewed_discord(&repo, "2026-09-18");
    let accepted = repo.read(&document(PlatformId::Discord)).expect("doc");
    let (on, impossible) = ("2026-10-20", "2026-09-31");
    let dated = accepted.replace("2026-09-18", on);
    let candidate = dated.replace(&format!("retrieved: {on}"), &format!("retrieved: {impossible}"));
    assert_ne!(candidate, dated, "the fixture edit applies");

    for (checked_on, refused_at) in [(impossible, Stage::Reconcile), (on, Stage::Validation)] {
        let run = repo.prepare(PlatformId::Discord, &day(on));
        let agent = Agent { repo: &repo, run: &run };
        agent.discovery();
        agent.reconcile(&candidate, &agent.checks(checked_on, &[]));
        agent.sources(false);
        let report = check_run(&repo.loader, run.run_id.as_str(), Stage::Validation, &day(on)).expect("check validation");
        assert_eq!(report.status, RunStatus::Failed, "checked on {checked_on}: {:#?}", report.stages);
        let stage = report.stages.iter().find(|result| result.stage == refused_at).expect("stage");
        assert_eq!(stage.status, StageStatus::Failed, "checked on {checked_on}: {:#?}", report.stages);
        assert!(stage.findings.iter().any(|f| f.contains(impossible)), "checked on {checked_on}: the finding names the date: {:#?}", stage.findings);
        assert!(
            matches!(renew(&repo, &run, on), Err(RefreshError::WrongStatus { status: RunStatus::Failed, .. })),
            "checked on {checked_on}"
        );
    }
    assert_eq!(repo.read(&document(PlatformId::Discord)).as_deref(), Some(accepted.as_str()), "nothing is published");
}

#[test]
fn an_initial_publication_promotes_every_platform_together() {
    let repo = Repo::bare();
    let on = "2026-09-18";
    let mut runs = Vec::new();
    for platform in PlatformId::ALL {
        let run = repo.prepare(*platform, &day(on));
        assert_eq!(repo.run(&run).selected_because, ["forced", "missing"]);
        let agent = Agent { repo: &repo, run: &run };
        let report = agent.complete(&renewed(*platform, on), &agent.checks(on, &[]), false, &day(on));
        assert!(report.passed, "{platform}: {:#?}", report.stages);
        runs.push(run);
    }
    let before = repo.tree();
    // One platform alone cannot form the first snapshot.
    match approve(&repo, &[&runs[0]], on) {
        Err(RefreshError::Generate(inner)) => match *inner {
            GenerateError::Refused { missing, .. } => assert_eq!(missing.len(), 4),
            other => panic!("expected a refused generation, got {other}"),
        },
        other => panic!("expected a refused generation, got {other:?}"),
    }
    assert_eq!(repo.tree(), before, "nothing is published");
    assert_eq!(repo.run(&runs[0]).status, RunStatus::AwaitingReview);
    // Renewal cannot accept an initial baseline.
    assert!(reasons(renew(&repo, &runs[1], on).expect_err("initial")).iter().any(|r| r.contains("initial baseline")));

    let refs: Vec<&PreparedRun> = runs.iter().collect();
    let promoted = approve(&repo, &refs, on).expect("initial publication");
    assert_eq!(promoted.len(), 5);
    let changelog = repo.read(CHANGELOG).expect("changelog");
    assert_eq!(changelog.matches("initial baseline, approved by maintainer").count(), 5, "{changelog}");
    assert!(runs.iter().all(|run| repo.run(run).status == RunStatus::Accepted));
    assert_eq!(check(&repo.loader, &day(on)).expect("check"), None);
}

#[test]
fn a_partial_refresh_publishes_successes_and_keeps_failed_platforms_resumable() {
    let repo = Repo::published();
    let before = repo.tree();
    let on = "2026-09-18";
    let good = repo.prepare(PlatformId::Discord, &day(on));
    let bad = repo.prepare(PlatformId::Slack, &day(on));
    let agent = Agent { repo: &repo, run: &good };
    agent.complete(&raised(on), &agent.checks(on, &[]), false, &day(on));
    let failing = Agent { repo: &repo, run: &bad };
    failing.discovery();
    failing.reconcile("---\nnot: a document\n---\n", &failing.checks(on, &[]));
    failing.sources(false);
    assert!(!check_run(&repo.loader, bad.run_id.as_str(), Stage::Validation, &day(on)).expect("check").passed);
    assert!(matches!(approve(&repo, &[&bad], on), Err(RefreshError::WrongStatus { status: RunStatus::Failed, .. })));

    approve(&repo, &[&good], on).expect("publish the success");
    assert_eq!(repo.read(&document(PlatformId::Slack)).expect("slack").as_bytes(), before[&document(PlatformId::Slack)].as_slice());
    assert!(repo.read(&document(PlatformId::Slack)).expect("slack").contains("last_updated: 2026-09-10"), "old research is not made to look new");
    assert_eq!(repo.run(&bad).status, RunStatus::Failed);
    let resumed = prepare::resume(&repo.loader, bad.run_id.as_str()).expect("resume");
    assert_eq!(resumed.stages, [Stage::Reconcile, Stage::Sources, Stage::Validation, Stage::Review]);
}

// ---- recovery and budgets ---------------------------------------------------------

#[test]
fn recovery_is_bounded_and_never_adds_budget() {
    let repo = Repo::published();
    let on = day("2026-09-18");
    let run = repo.prepare(PlatformId::Discord, &on);
    let id = run.run_id.as_str();
    let agent = Agent { repo: &repo, run: &run };
    agent.discovery();
    agent.reconcile("---\nbroken\n", &agent.checks("2026-09-18", &[]));
    agent.sources(false);
    assert!(matches!(prepare::resume(&repo.loader, id), Err(RefreshError::WrongStatus { status: RunStatus::Active, .. })));
    for attempt in 1..=2 {
        assert!(!check_run(&repo.loader, id, Stage::Validation, &on).expect("check").passed);
        let resumed = prepare::resume(&repo.loader, id).expect("resume");
        assert_eq!(resumed.stages[0], Stage::Reconcile, "validation failures rerun reconciliation");
        assert!(resumed.commands.iter().all(|c| !c.contains(&"init".to_string())), "a resumption keeps the ledger");
        assert_eq!(repo.run(&run).recovery_attempts, attempt);
        let sequence = fs::read_to_string(repo.run_dir(&run).join("run.md")).expect("run.md");
        assert!(!sequence.contains("name: discovery"), "completed discovery is not rerun");
    }
    assert!(!check_run(&repo.loader, id, Stage::Validation, &on).expect("check").passed);
    assert!(matches!(prepare::resume(&repo.loader, id), Err(RefreshError::RecoveryLimit { .. })));
}

/// Review-1 regression: a failed run is not open, so a new run could be
/// prepared for its platform and the failed run then resumed, leaving two
/// open runs.
#[test]
fn resuming_is_refused_while_another_run_for_the_platform_is_open_or_unreadable() {
    let repo = Repo::published();
    let on = day("2026-09-18");
    let failed = repo.prepare(PlatformId::Discord, &on);
    assert!(!check_run(&repo.loader, failed.run_id.as_str(), Stage::Validation, &on).expect("check").passed);
    assert_eq!(repo.run(&failed).status, RunStatus::Failed);
    let newer = repo.prepare(PlatformId::Discord, &on);

    let refused = prepare::resume(&repo.loader, failed.run_id.as_str()).expect_err("another run is open");
    assert!(matches!(&refused, RefreshError::OtherRunBlocks { path, .. } if *path == newer.run_dir), "{refused}");
    assert!(refused.to_string().contains(newer.run_id.as_str()), "{refused}");
    let failed_record = repo.run(&failed);
    assert_eq!((failed_record.status, failed_record.recovery_attempts), (RunStatus::Failed, 0), "nothing was reopened");
    let open = repo.state().list().into_iter().filter_map(|(_, _, r)| r.ok()).filter(|r| r.status.is_open()).count();
    assert_eq!(open, 1);

    // An unreadable record might be the open run, so it refuses too.
    let record = repo.read(&format!("{}/run.json", newer.run_dir)).expect("record");
    repo.write(&format!("{}/run.json", newer.run_dir), "not json at all\n");
    let refused = prepare::resume(&repo.loader, failed.run_id.as_str()).expect_err("unreadable record");
    assert!(matches!(&refused, RefreshError::OtherRunBlocks { path, .. } if *path == newer.run_dir), "{refused}");
    repo.write(&format!("{}/run.json", newer.run_dir), &record);

    // Once the other run is no longer open, the failed run resumes.
    assert!(!check_run(&repo.loader, newer.run_id.as_str(), Stage::Validation, &on).expect("check").passed);
    reject(&repo, newer.run_id.as_str(), "Maintainer", "superseded", &on).expect("reject");
    prepare::resume(&repo.loader, failed.run_id.as_str()).expect("resume");
    assert_eq!(repo.run(&failed).status, RunStatus::Active);
}

#[test]
fn an_exhausted_budget_stays_incomplete_and_needs_a_recorded_grant() {
    let repo = Repo::published();
    let before = repo.tree();
    let on = day("2026-09-18");
    let run = repo.prepare(PlatformId::Discord, &on);
    let agent = Agent { repo: &repo, run: &run };
    agent.discovery();
    // Claudine's ledger after exhaustion during reconciliation.
    let ledger = |state: &str| {
        json!({ "format": "claudine-budget-ledger/1", "run_id": run.run_id.as_str(), "platform": "discord", "state": state,
                "stop_reason": null, "stage": "reconcile", "limits": { "invocations": 8, "active_ms": 600000 },
                "used": { "invocations": 8, "active_ms": 412000 }, "grants": [], "heartbeat_ms": 5000, "runs": 1,
                "segment": null, "in_flight": [], "events": [], "exclusive_lock": "../../fleet.lock" })
        .to_string()
    };
    agent.write("budget.json", &ledger("exhausted"));
    let report = check_run(&repo.loader, run.run_id.as_str(), Stage::Validation, &on).expect("check");
    assert!(!report.passed);
    assert_eq!(report.status, RunStatus::Exhausted, "exhaustion is never a finished or investigated result");
    assert!(repo.run(&run).stop_reason.expect("reason").contains("reconcile"));
    assert!(matches!(approve(&repo, &[&run], "2026-09-18"), Err(RefreshError::WrongStatus { status: RunStatus::Exhausted, .. })));
    assert!(matches!(
        prepare::resume(&repo.loader, run.run_id.as_str()),
        Err(RefreshError::Ledger { ref state, .. }) if state == "exhausted"
    ));
    assert_eq!(repo.run(&run).recovery_attempts, 0, "a refused resumption consumes no attempt");
    // After an operator grant Claudine rests the ledger `stopped`; resumption
    // continues the same run and ledger.
    agent.write("budget.json", &ledger("stopped"));
    let resumed = prepare::resume(&repo.loader, run.run_id.as_str()).expect("resume after grant");
    assert_eq!(resumed.stages[0], Stage::Reconcile);
    assert_eq!(repo.tree(), before, "accepted research is untouched");
}

#[test]
fn a_sequence_that_stops_before_its_checks_leaves_a_resumable_or_rejectable_run() {
    let repo = Repo::published();
    let before = repo.tree();
    let on = day("2026-09-18");
    let run = repo.prepare(PlatformId::Discord, &on);
    let id = run.run_id.as_str();
    let agent = Agent { repo: &repo, run: &run };
    let ledger = |runs: u64, stop_reason: &str| {
        json!({ "format": "claudine-budget-ledger/1", "run_id": id, "platform": "discord", "state": "stopped",
                "stop_reason": stop_reason, "stage": null, "limits": { "invocations": 8, "active_ms": 600000 },
                "used": { "invocations": 0, "active_ms": 1200 }, "grants": [], "heartbeat_ms": 5000, "runs": runs,
                "segment": null, "in_flight": [], "events": [], "exclusive_lock": "../../fleet.lock" })
        .to_string()
    };
    let active = |result: Result<_, RefreshError>| matches!(result, Err(RefreshError::WrongStatus { status: RunStatus::Active, .. }));
    let discord_is_open = |repo: &Repo| {
        let selection = &repo.select("2026-09-18", &Request { forced: [PlatformId::Discord].into(), ..Request::default() })[&PlatformId::Discord];
        matches!(selection.skip, Some(Skip::OpenRun { .. }))
    };

    // Initialized but never launched: the printed sequence command is still
    // the way forward, so the run stays active.
    agent.write("budget.json", &ledger(0, "initialized"));
    assert!(active(prepare::resume(&repo.loader, id).map(|_| ())));
    assert!(active(reject(&repo, id, "Maintainer", "unused", &on).map(|_| ())));
    assert!(discord_is_open(&repo));

    // Claudine refused the first step (no agent named, no terminal); the ledger
    // rests `stopped` and no stage recorded a result.
    agent.write("budget.json", &ledger(1, "agent resolution failed for run.md: NoAgent"));
    assert!(!discord_is_open(&repo), "a failed run no longer blocks selection");
    let resumed = prepare::resume(&repo.loader, id).expect("a stopped sequence is resumable");
    assert_eq!(resumed.stages[0], Stage::Discovery, "nothing completed, so everything reruns");
    assert!(resumed.commands.iter().all(|c| !c.contains(&"init".to_string())), "a resumption keeps the ledger");
    assert_eq!(repo.run(&run).recovery_attempts, 1);
    assert_eq!(repo.run(&run).status, RunStatus::Active);

    // Resumed but not yet relaunched: the ledger still shows the earlier
    // sequence, which must not count against the new attempt.
    assert!(active(prepare::resume(&repo.loader, id).map(|_| ())));
    assert_eq!(repo.run(&run).recovery_attempts, 1, "a refused resumption consumes no attempt");
    assert!(discord_is_open(&repo), "the resumed run is open again");

    // The relaunched sequence is stopped by Ctrl+C before any check ran.
    agent.write("budget.json", &ledger(2, "interrupted by the operator"));
    let rejected = reject(&repo, id, "Maintainer", "wrong agent", &on).expect("a stopped run is rejectable");
    assert_eq!(rejected.status, RunStatus::Rejected);
    assert_eq!(repo.run(&run).status, RunStatus::Rejected, "the decision is persisted");
    assert!(!discord_is_open(&repo), "a rejected run no longer blocks selection");
    assert_eq!(repo.tree(), before, "accepted research is untouched");
}

#[test]
fn an_interrupted_promotion_recovers_to_one_consistent_snapshot_and_completes_on_retry() {
    let points = [
        Point::BeforeStaging,
        Point::DuringStaging,
        Point::StagedNoJournal,
        Point::BeforeSelection,
        Point::MidReplacement(1),
        Point::BeforeManifest,
        Point::AfterSelection,
        Point::BeforeCleanup,
    ];
    for point in points {
        let repo = Repo::published();
        let old = read_verified(repo.loader.workspace()).expect("old").manifest;
        let run = repo.prepare(PlatformId::Discord, &day("2026-09-18"));
        let agent = Agent { repo: &repo, run: &run };
        agent.complete(&raised("2026-09-18"), &agent.checks("2026-09-18", &[]), false, &day("2026-09-18"));
        let ids = [run.run_id.to_string()];
        let approval = Approve::Human { by: Maintainer::new("maintainer").expect("a named maintainer") };
        let options = Options { interrupt_at: Some(point), ..Options::default() };
        assert!(promote::promote(&repo.loader, &ids, &approval, &day("2026-09-18"), options).is_err(), "{point}");
        assert_eq!(repo.run(&run).status, RunStatus::AwaitingReview, "{point}: not accepted before publication completes");
        publish::recover(repo.loader.workspace(), Options::default()).expect("recover");
        let selected = read_verified(repo.loader.workspace()).expect("a consistent snapshot").manifest;
        let rolled_forward = selected != old;
        promote::promote(&repo.loader, &ids, &approval, &day("2026-09-18"), Options::default()).expect("retry");
        assert_eq!(repo.run(&run).status, RunStatus::Accepted, "{point}");
        assert_eq!(repo.read(&document(PlatformId::Discord)).expect("doc"), raised("2026-09-18"), "{point}");
        let changelog = repo.read(CHANGELOG).expect("changelog");
        assert_eq!(changelog.matches(&format!("(run {})", run.run_id)).count(), 1, "{point} rolled forward: {rolled_forward}");
        assert_eq!(check(&repo.loader, &day("2026-09-18")).expect("check"), None, "{point}");
    }
}

// ---- retention -----------------------------------------------------------------------

#[test]
fn cleanup_previews_exact_removals_and_protects_open_runs() {
    let repo = Repo::published();
    let state = repo.state();
    let mut runs = BTreeMap::new();
    for (platform, status, created) in [
        (PlatformId::Discord, RunStatus::Failed, "2026-08-01"),
        (PlatformId::Slack, RunStatus::AwaitingReview, "2026-08-01"),
        (PlatformId::Telegram, RunStatus::Accepted, "2026-08-01"),
        (PlatformId::WhatsApp, RunStatus::Rejected, "2026-09-10"),
        (PlatformId::Signal, RunStatus::Exhausted, "2026-08-18"),
    ] {
        let run = repo.prepare(platform, &day(created));
        let mut record = repo.run(&run);
        record.status = status;
        state.save(&record).expect("save");
        runs.insert(platform, run);
    }
    let renewal_dir = state.renewals_dir();
    fs::create_dir_all(&renewal_dir).expect("mkdir");
    fs::write(renewal_dir.join("broken.json"), "{}").expect("write");
    // Hold the Signal run's ledger lock, as a running Claudine would.
    let lock_path = repo.run_dir(&runs[&PlatformId::Signal]).join("budget.json.lock");
    let lock = fs::File::create(&lock_path).expect("lock file");
    lock.lock().expect("hold the ledger lock");

    let before = repo.tree();
    let plan = cleanup::plan(&state, &day("2026-09-17"), cleanup::DEFAULT_THRESHOLD_DAYS);
    let removal_paths: Vec<&str> = plan.removals.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        removal_paths,
        [runs[&PlatformId::Discord].run_dir.as_str(), runs[&PlatformId::Telegram].run_dir.as_str()],
        "{plan:#?}"
    );
    assert!(plan.removals[0].loses_resumability, "a failed run can no longer be resumed");
    assert!(!plan.removals[1].loses_resumability);
    assert_eq!(plan.removals[0].age_days, 47);
    let protected: BTreeMap<&str, &str> = plan.protected.iter().map(|p| (p.path.as_str(), p.reason.as_str())).collect();
    assert_eq!(protected[runs[&PlatformId::Slack].run_dir.as_str()], "the candidate awaits review");
    assert_eq!(protected[runs[&PlatformId::Signal].run_dir.as_str()], "a Claudine runner holds the ledger lock");
    assert!(protected["messenger/.research-state/renewals/broken.json"].contains("unreadable"));
    assert_eq!(plan.younger, 1, "the recent rejected run is kept");
    assert!(repo.run_dir(&runs[&PlatformId::Discord]).exists(), "a preview deletes nothing");

    drop(lock);
    let removed = cleanup::apply(&state, &plan).expect("apply");
    assert_eq!(removed, removal_paths);
    assert!(!repo.run_dir(&runs[&PlatformId::Discord]).exists());
    assert!(!repo.run_dir(&runs[&PlatformId::Telegram]).exists());
    for platform in [PlatformId::Slack, PlatformId::WhatsApp, PlatformId::Signal] {
        assert!(repo.run_dir(&runs[&platform]).exists(), "{platform} kept");
    }
    assert_eq!(repo.tree(), before, "cleanup never touches accepted research");
    // The Signal run was released after the preview; it was not in the plan,
    // so it survives until a new preview lists it.
    let again = cleanup::plan(&state, &day("2026-09-17"), cleanup::DEFAULT_THRESHOLD_DAYS);
    assert_eq!(again.removals.len(), 1);
    assert!(again.removals[0].loses_resumability, "an exhausted run can be resumed after a grant");
}

#[test]
fn prepared_runs_never_write_accepted_research_even_when_the_worker_misbehaves() {
    let repo = Repo::published();
    let before = repo.tree();
    let run = repo.prepare(PlatformId::Discord, &day("2026-09-18"));
    let agent = Agent { repo: &repo, run: &run };
    agent.complete(&raised("2026-09-18"), &agent.checks("2026-09-18", &[]), false, &day("2026-09-18"));
    // A worker that edits the accepted file directly: verification refuses the
    // snapshot, so nothing downstream consumes the edit.
    repo.write(&document(PlatformId::Discord), &raised("2026-09-18"));
    assert!(read_verified(repo.loader.workspace()).is_err());
    assert!(generate::published_catalog(repo.loader.workspace()).is_err());
    repo.write(&document(PlatformId::Discord), std::str::from_utf8(&before[&document(PlatformId::Discord)]).expect("utf8"));
    assert_eq!(repo.tree(), before);
}
