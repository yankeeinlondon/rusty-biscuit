//! Level-1 sessions for referenced Markdown targets outside the file list.
//!
//! One target, `target.md` (`# Alpha`, `## Beta`, and a transclusion back
//! to the source), is reached three ways: listed (the control), excluded
//! from discovery by `workspace.exclude`, and outside the editor's workspace
//! folder (`docs/`) as `../target.md` inside the enclosing repository. Every
//! consumer of a target's contents must answer the same in all three:
//! missing-anchor diagnostics, heading completion, hover, definition,
//! transclusion cycles, and refresh after the target changes or is deleted,
//! through watched events and the save fallback alike. Answers drawn from the
//! file list (wiki candidates, workspace symbols) must not grow because an
//! unlisted target was parsed.
//!
//! Parse and read counts come from `dmls::work_counters`, which are
//! process-wide; nextest runs each test in its own process.

use std::path::{Path, PathBuf};

use crate::common;

use common::{LspFixture, LspWorkspace};
use dmls::work_counters::work_counts;
use serde_json::{Value, json};

const MISSING_ANCHOR: &str = "dm.links.missing_anchor";
const BROKEN_PATH: &str = "dm.links.broken_path";
const CYCLE: &str = "dm.transclusion.cycle";
const TRANSCLUSION_BROKEN: &str = "dm.transclusion.broken_path";

/// How the source reaches `target.md`.
#[derive(Clone, Copy, Debug)]
enum Case {
    /// The target is in the file list.
    Listed,
    /// `workspace.exclude` keeps the target out of the file list.
    Excluded,
    /// The workspace folder is `docs/`; the target sits above it.
    Outside,
}

/// How the client learns about disk changes.
#[derive(Clone, Copy, Debug)]
enum Client {
    /// Watches files and roots watchers at folders (`relativePatternSupport`).
    FolderWatches,
    /// Watches files but cannot root a watcher at a folder.
    GlobWatches,
    /// No file watching: the server rescans on save.
    Unwatched,
}

/// A repository holding `target.md` at its root and the source wherever the
/// case puts it.
struct Scenario {
    workspace: LspWorkspace,
    folder: PathBuf,
    source: PathBuf,
    target: PathBuf,
    /// How the source spells the target.
    reference: &'static str,
}

impl Scenario {
    fn new(case: Case) -> Self {
        let workspace = LspWorkspace::new();
        let repo = workspace.path().to_path_buf();
        init_repository(&repo);
        let (folder, reference, back) = match case {
            Case::Listed => (repo.clone(), "target.md", "./source.md"),
            Case::Excluded => {
                write(&repo.join(".dmls.toml"), "[workspace]\nexclude = [\"target.md\"]\n");
                (repo.clone(), "target.md", "./source.md")
            }
            Case::Outside => (repo.join("docs"), "../target.md", "./docs/source.md"),
        };
        std::fs::create_dir_all(&folder).unwrap();
        let target = repo.join("target.md");
        write(&target, &format!("# Alpha\n\n## Beta\n\n::file {back}\n"));
        let source = folder.join("source.md");
        Self { workspace, folder, source, target, reference }
    }

    /// Links to a missing and a present anchor, and transcludes the target.
    fn text(&self) -> String {
        format!("[m]({r}#missing)\n[b]({r}#beta)\n\n::file {r}\n", r = self.reference)
    }

    /// The character just after `#beta`'s `#` on line 1.
    fn beta_character(&self) -> u32 {
        ("[b](".len() + self.reference.len() + 2) as u32
    }

    fn start(&self, client: Client) -> LspFixture<'_> {
        let mut fixture = LspFixture::start(&self.workspace);
        fixture.initialize(initialize_params(&self.folder, client));
        fixture
    }

    fn open(&self, fixture: &mut LspFixture<'_>, text: &str) -> Vec<String> {
        fixture.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri(&self.source), "languageId": "markdown", "version": 1, "text": text } }),
        );
        codes(&latest(fixture, &uri(&self.source)))
    }
}

/// A minimal repository Git discovery accepts, without running `git`.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn initialize_params(folder: &Path, client: Client) -> Value {
    let folder_uri = url::Url::from_directory_path(folder).unwrap();
    let mut params = json!({
        "processId": null,
        "clientInfo": { "name": "Referenced Targets Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "configuration": true }
        },
        "workspaceFolders": [ { "uri": folder_uri.as_str(), "name": "scratch" } ]
    });
    let watched = match client {
        Client::FolderWatches => Some(json!({ "dynamicRegistration": true, "relativePatternSupport": true })),
        Client::GlobWatches => Some(json!({ "dynamicRegistration": true })),
        Client::Unwatched => None,
    };
    if let Some(watched) = watched {
        params["capabilities"]["workspace"]["didChangeWatchedFiles"] = watched;
    }
    params
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

/// The diagnostics last published for `uri` once the server has handled
/// every earlier message and finished the registry jobs they queued. The
/// round trip is `documentSymbol`, which parses only `uri`, so lazy parse
/// counts are unaffected.
fn latest(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri } }));
    let latest = fixture.take_buffered_diagnostics(uri).expect("diagnostics were published");
    while fixture.take_buffered_diagnostics(uri).is_some() {}
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

fn codes(diagnostics: &[Value]) -> Vec<String> {
    let mut codes: Vec<String> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str().map(str::to_string))
        .collect();
    codes.sort();
    codes
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn position(fixture: &mut LspFixture<'_>, method: &str, source: &Path, line: u32, character: u32) -> Value {
    let response = fixture.request(
        method,
        json!({ "textDocument": { "uri": uri(source) }, "position": { "line": line, "character": character } }),
    );
    assert!(response.error.is_none(), "{method}: {:?}", response.error);
    response.result.unwrap_or(Value::Null)
}

fn completion_labels(result: &Value) -> Vec<String> {
    let items = result.as_array().cloned().or_else(|| result["items"].as_array().cloned()).unwrap_or_default();
    let mut labels: Vec<String> = items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect();
    labels.sort();
    labels
}

// ── consumers ──────────────────────────────────────────────────────────────

fn missing_anchor(case: Case) {
    let scenario = Scenario::new(case);
    let mut fixture = scenario.start(Client::GlobWatches);
    let codes = scenario.open(&mut fixture, &scenario.text());
    assert_eq!(codes.iter().filter(|code| *code == MISSING_ANCHOR).count(), 1, "{case:?}: {codes:?}");
    fixture.shutdown();
}

fn transclusion_cycle(case: Case) {
    let scenario = Scenario::new(case);
    let mut fixture = scenario.start(Client::GlobWatches);
    let codes = scenario.open(&mut fixture, &scenario.text());
    assert!(codes.contains(&CYCLE.to_string()), "{case:?}: {codes:?}");
    fixture.shutdown();
}

fn heading_completion(case: Case) {
    let scenario = Scenario::new(case);
    let mut fixture = scenario.start(Client::GlobWatches);
    // Nothing but the open anchor: completion alone must find the target.
    let text = format!("[c]({}#\n", scenario.reference);
    scenario.open(&mut fixture, &text);
    let character = (text.len() - 1) as u32;
    let result = position(&mut fixture, "textDocument/completion", &scenario.source, 0, character);
    assert_eq!(completion_labels(&result), strings(&["alpha", "beta"]), "{case:?}");
    fixture.shutdown();
}

fn hover(case: Case) {
    let scenario = Scenario::new(case);
    let mut fixture = scenario.start(Client::GlobWatches);
    scenario.open(&mut fixture, &scenario.text());
    let result = position(&mut fixture, "textDocument/hover", &scenario.source, 1, scenario.beta_character());
    let contents = result["contents"].to_string();
    assert!(contents.contains("Alpha") && contents.contains("Beta"), "{case:?}: {contents}");
    fixture.shutdown();
}

fn definition(case: Case) {
    let scenario = Scenario::new(case);
    let mut fixture = scenario.start(Client::GlobWatches);
    scenario.open(&mut fixture, &scenario.text());
    let result = position(&mut fixture, "textDocument/definition", &scenario.source, 1, scenario.beta_character());
    let location = if result.is_array() { result[0].clone() } else { result };
    let target = location.get("targetUri").or_else(|| location.get("uri")).cloned().unwrap_or(Value::Null);
    let range = location.get("targetSelectionRange").or_else(|| location.get("range")).cloned().unwrap_or(Value::Null);
    assert_eq!(target, json!(uri(&scenario.target)), "{case:?}: {location}");
    assert_eq!(range["start"]["line"], json!(2), "{case:?}: `## Beta` is line two: {location}");
    fixture.shutdown();
}

/// The target loses `## Beta` and its transclusion, then is deleted; each
/// step reaches the server as `refresh` delivers it.
fn change_then_delete(case: Case, client: Client, refresh: fn(&Scenario, &mut LspFixture<'_>, u32)) {
    let scenario = Scenario::new(case);
    let mut fixture = scenario.start(client);
    let opened = scenario.open(&mut fixture, &scenario.text());
    assert_eq!(opened, strings(&[MISSING_ANCHOR, CYCLE]), "{case:?}");

    write(&scenario.target, "# Alpha\n\n## Gamma\n");
    refresh(&scenario, &mut fixture, 2);
    let changed = codes(&latest(&mut fixture, &uri(&scenario.source)));
    assert_eq!(changed, strings(&[MISSING_ANCHOR, MISSING_ANCHOR]), "{case:?} after the change");

    std::fs::remove_file(&scenario.target).unwrap();
    refresh(&scenario, &mut fixture, 3);
    let deleted = codes(&latest(&mut fixture, &uri(&scenario.source)));
    assert_eq!(deleted, strings(&[BROKEN_PATH, BROKEN_PATH, TRANSCLUSION_BROKEN]), "{case:?} after deletion");

    // Re-creating it restores the analysis.
    write(&scenario.target, "# Alpha\n\n## Beta\n");
    refresh(&scenario, &mut fixture, 1);
    let restored = codes(&latest(&mut fixture, &uri(&scenario.source)));
    assert_eq!(restored, strings(&[MISSING_ANCHOR]), "{case:?} after re-creation");
    fixture.shutdown();
}

fn watched_event(scenario: &Scenario, fixture: &mut LspFixture<'_>, kind: u32) {
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(&scenario.target), "type": kind } ] }),
    );
}

fn save(scenario: &Scenario, fixture: &mut LspFixture<'_>, _kind: u32) {
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(&scenario.source) } }));
}

fn watched_refresh(case: Case) {
    change_then_delete(case, Client::FolderWatches, watched_event);
}

fn save_rescan_refresh(case: Case) {
    change_then_delete(case, Client::Unwatched, save);
}

/// A watching client that cannot root a watcher at a folder: a save
/// rechecks every unlisted target.
fn folderless_save_refresh(case: Case) {
    change_then_delete(case, Client::GlobWatches, save);
}

/// The target is read and parsed once, then reused by every later request;
/// a `::code` file beside it is never parsed as Markdown.
fn parsed_once(case: Case) {
    let scenario = Scenario::new(case);
    let code = scenario.target.with_file_name("mod.rs");
    write(&code, "# not a heading\n");
    let code_reference = scenario.reference.replace("target.md", "mod.rs");
    let mut fixture = scenario.start(Client::GlobWatches);
    let before = work_counts();
    scenario.open(&mut fixture, &format!("{}::code {code_reference}\n", scenario.text()));
    let work = work_counts().since(before);
    assert_eq!((work.parses, work.reads), (2, 1), "{case:?}: the source, then the target: {work:?}");

    let before = work_counts();
    position(&mut fixture, "textDocument/hover", &scenario.source, 1, scenario.beta_character());
    position(&mut fixture, "textDocument/definition", &scenario.source, 1, scenario.beta_character());
    position(&mut fixture, "textDocument/hover", &scenario.source, 4, 8);
    let work = work_counts().since(before);
    assert_eq!((work.parses, work.reads), (0, 0), "{case:?}: reused: {work:?}");
    fixture.shutdown();
}

/// Wiki candidates come from the file list alone: a parsed unlisted target
/// is never one.
fn wiki_heading(case: Case) {
    let scenario = Scenario::new(case);
    let mut fixture = scenario.start(Client::GlobWatches);
    let text = format!("{}\n[[target#\n", scenario.text());
    scenario.open(&mut fixture, &text);
    let result = position(&mut fixture, "textDocument/completion", &scenario.source, 5, 9);
    let expected = match case {
        Case::Listed => strings(&["Alpha", "Beta"]),
        Case::Excluded | Case::Outside => Vec::new(),
    };
    assert_eq!(completion_labels(&result), expected, "{case:?}");
    fixture.shutdown();
}

/// Workspace symbols answer from the file list: a parsed unlisted target's
/// headings are not among them.
fn workspace_symbols(case: Case) {
    let scenario = Scenario::new(case);
    let mut fixture = scenario.start(Client::GlobWatches);
    scenario.open(&mut fixture, &scenario.text());
    let response = fixture.request("workspace/symbol", json!({ "query": "Beta" }));
    assert!(response.error.is_none(), "{:?}", response.error);
    let names: Vec<String> = response
        .result
        .and_then(|result| result.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|symbol| symbol["name"].as_str().map(str::to_string))
        .collect();
    let expected = match case {
        Case::Listed => strings(&["Beta"]),
        Case::Excluded | Case::Outside => Vec::new(),
    };
    assert_eq!(names, expected, "{case:?}");
    fixture.shutdown();
}

/// One `#[test]` per (case, consumer) cell.
macro_rules! cells {
    ($($name:ident => $consumer:ident($case:ident);)*) => {
        $(
            #[test]
            fn $name() {
                $consumer(Case::$case);
            }
        )*
    };
}

cells! {
    listed_target_missing_anchor => missing_anchor(Listed);
    excluded_target_missing_anchor => missing_anchor(Excluded);
    outside_target_missing_anchor => missing_anchor(Outside);
    listed_target_transclusion_cycle => transclusion_cycle(Listed);
    excluded_target_transclusion_cycle => transclusion_cycle(Excluded);
    outside_target_transclusion_cycle => transclusion_cycle(Outside);
    listed_target_heading_completion => heading_completion(Listed);
    excluded_target_heading_completion => heading_completion(Excluded);
    outside_target_heading_completion => heading_completion(Outside);
    listed_target_hover => hover(Listed);
    excluded_target_hover => hover(Excluded);
    outside_target_hover => hover(Outside);
    listed_target_definition => definition(Listed);
    excluded_target_definition => definition(Excluded);
    outside_target_definition => definition(Outside);
    listed_target_watched_refresh => watched_refresh(Listed);
    excluded_target_watched_refresh => watched_refresh(Excluded);
    outside_target_watched_refresh => watched_refresh(Outside);
    listed_target_save_rescan_refresh => save_rescan_refresh(Listed);
    excluded_target_save_rescan_refresh => save_rescan_refresh(Excluded);
    outside_target_save_rescan_refresh => save_rescan_refresh(Outside);
    excluded_target_folderless_save_refresh => folderless_save_refresh(Excluded);
    outside_target_folderless_save_refresh => folderless_save_refresh(Outside);
    listed_target_parsed_once => parsed_once(Listed);
    excluded_target_parsed_once => parsed_once(Excluded);
    outside_target_parsed_once => parsed_once(Outside);
    listed_target_wiki_heading => wiki_heading(Listed);
    excluded_target_wiki_heading => wiki_heading(Excluded);
    outside_target_wiki_heading => wiki_heading(Outside);
    listed_target_workspace_symbols => workspace_symbols(Listed);
    excluded_target_workspace_symbols => workspace_symbols(Excluded);
    outside_target_workspace_symbols => workspace_symbols(Outside);
}

#[test]
fn an_outside_target_folder_is_watched_once_parsed() {
    let scenario = Scenario::new(Case::Outside);
    let mut fixture = scenario.start(Client::FolderWatches);
    scenario.open(&mut fixture, &scenario.text());
    let repo = scenario.target.parent().unwrap();
    let name = repo.file_name().unwrap().to_str().unwrap();
    let base = url::Url::from_directory_path(repo.parent().unwrap()).unwrap();
    let watched: Vec<(String, String)> = fixture
        .buffered_server_requests()
        .iter()
        .filter(|request| request.method == "client/registerCapability")
        .flat_map(|request| request.params["registrations"].as_array().cloned().unwrap_or_default())
        .filter(|registration| registration["id"] == json!("dmls-watch-schema-inputs"))
        .flat_map(|registration| registration["registerOptions"]["watchers"].as_array().cloned().unwrap_or_default())
        .map(|watcher| {
            let pattern = &watcher["globPattern"];
            (
                pattern["baseUri"].as_str().unwrap_or_default().trim_end_matches('/').to_string(),
                pattern["pattern"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    assert!(
        watched.contains(&(base.as_str().trim_end_matches('/').to_string(), format!("{name}/*"))),
        "the target's folder is watched: {watched:?}"
    );
    fixture.shutdown();
}
