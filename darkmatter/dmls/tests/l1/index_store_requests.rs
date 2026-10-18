//! Level-1 equivalence of the store-backed requests with the full-graph
//! oracle (`common::oracle`), on the index-store corpus.
//!
//! The corpus is filled into real stores (one per root: `ws`, the nested
//! `ws/nested`, and `other`), then every request the server answers from
//! the store is asked at every heading, link, wiki link, and transclusion of
//! every corpus file: references (heading backlinks, link targets, wiki
//! backlinks, DSL transcluded-by), heading rename, file rename (including
//! wiki spellings, collisions, and a directory), and workspace symbols. Each
//! answer comes from [`WorkspaceFacts::request_graph`] or
//! [`WorkspaceFacts::symbol_documents`] through the same provider calls the
//! oracle makes, and must equal the oracle's.
//!
//! The same checks run with the stores unusable (off, read-only, corrupt
//! while another holder has them open), where facts live in memory, with
//! another writer committing an older row just before every request, and
//! with a store that stops decoding after ready, where the request reading
//! it first switches to memory and must still answer in full.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use dmls::clock::SystemClock;
use dmls::graph::{EdgeKind, WorkspaceGraph, index_document};
use dmls::index_store::{FileFacts, IndexStore, StoreLocation, StoreLock, WorkspaceFacts, lock_path, store_path};
use dmls::log_capture::CapturedLogs;
use dmls::providers::reference_target_at;
use dmls::wiki::WikiUniverse;
use dmls::workspace::startup::SilentProgress;

use crate::common::oracle::{Corpus, Oracle};

/// The store side of a comparison: every root's facts, filled from disk and
/// ready, plus what the server supplies with them.
struct Store {
    facts: WorkspaceFacts,
    listed: BTreeSet<PathBuf>,
    universe: Arc<WikiUniverse>,
    /// When set, commits its older row before every request.
    late: Option<LateWriter>,
}

/// Another process's connection to the `ws` store, holding the facts of an
/// earlier version of one file: a batch it computed before the file's
/// current bytes were saved, and commits after this process read them. Disk
/// is never changed.
struct LateWriter {
    store: IndexStore,
    older: FileFacts,
}

impl LateWriter {
    fn open(corpus: &Corpus, location: &StoreLocation, path: &Path, older: &str) -> Self {
        let StoreLocation::Directory(dir) = location else { panic!("expected a store directory, got {location:?}") };
        let (store, _) = IndexStore::open(&store_path(dir, &corpus.ws), &corpus.ws, Arc::new(SystemClock)).unwrap();
        Self { store, older: FileFacts::from_index(&index_document(path, older)) }
    }

    /// Commits the older row, and proves it is in the store and differs from
    /// what `facts` read.
    fn publish(&mut self, facts: &WorkspaceFacts) {
        self.store.upsert_batch(std::slice::from_ref(&self.older)).unwrap();
        let stored = self.store.hashes().unwrap()[&self.older.path];
        assert_eq!(stored, self.older.hash);
        assert_ne!(Some(stored), facts.verified_hash(&self.older.path), "the late row is not this process's read");
    }
}

impl Store {
    fn fill(corpus: &Corpus, oracle: &Oracle, location: &StoreLocation) -> Self {
        let roots = corpus.roots();
        let mut facts = WorkspaceFacts::open(&roots, location, Arc::new(SystemClock), "test");
        let files = oracle.files();
        let report = facts.revalidate(&files, &BTreeMap::new(), &SilentProgress, &AtomicBool::new(false));
        assert!(report.failed.is_empty() && report.missing.is_empty(), "{report:?}");
        facts.mark_ready();
        let listed: BTreeSet<PathBuf> = files.into_iter().collect();
        assert!(facts.is_ready(&listed, &roots));
        Self { facts, listed, universe: Arc::new(oracle.file_list_universe()), late: None }
    }

    fn publish_late(&mut self) {
        if let Some(late) = self.late.as_mut() {
            late.publish(&self.facts);
        }
    }

    fn graph(&mut self, oracle: &Oracle, targets: &[PathBuf]) -> WorkspaceGraph {
        self.publish_late();
        self.facts.request_graph(targets, &self.listed, std::iter::empty(), Arc::clone(&self.universe), oracle.contexts())
    }
}

fn corpus(source: PathBuf) -> (Corpus, Oracle) {
    let corpus = Corpus::materialize(&source);
    let oracle = corpus.oracle();
    (corpus, oracle)
}

fn cache_location(corpus: &Corpus) -> StoreLocation {
    let dir = corpus.base.join("cache");
    std::fs::create_dir_all(&dir).unwrap();
    StoreLocation::Directory(dir)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Probe {
    Heading,
    Link,
    WikiLink,
    Transclusion,
}

/// Every heading title, link, wiki link, and transclusion of every listed
/// file, as `(path, byte offset, kind)`.
fn probes(oracle: &Oracle) -> Vec<(PathBuf, usize, Probe)> {
    let graph = oracle.graph();
    let mut out = Vec::new();
    for path in oracle.files() {
        let document = graph.document_id(&path).expect("every listed file is a document");
        for (_, node) in graph.headings(document) {
            out.push((path.clone(), node.as_heading().unwrap().title_span.start, Probe::Heading));
        }
        out.extend(graph.links(document).map(|(_, node)| (path.clone(), node.span.start, Probe::Link)));
        out.extend(graph.wiki_links(document).map(|(_, node)| (path.clone(), node.span.start, Probe::WikiLink)));
        out.extend(graph.transclusions(document).map(|(_, node)| (path.clone(), node.span.start, Probe::Transclusion)));
    }
    out
}

/// The documents a references request is about, as the router computes
/// them: the document itself and the target under the cursor.
fn reference_targets(oracle: &Oracle, path: &Path, offset: usize) -> Vec<PathBuf> {
    std::iter::once(path.to_path_buf()).chain(reference_target_at(oracle.graph(), path, offset)).collect()
}

/// References at every probe, with and without the declaration. Returns
/// the probe kinds that had a non-empty answer.
fn check_references(store: &mut Store, oracle: &Oracle) -> BTreeSet<(Probe, bool)> {
    let mut answered = BTreeSet::new();
    for (path, offset, probe) in probes(oracle) {
        let position = oracle.position_at(&path, offset);
        let graph = store.graph(oracle, &reference_targets(oracle, &path, offset));
        for include in [false, true] {
            let expected = oracle.references(&path, position, include);
            let actual = oracle.references_over(&graph, &path, position, include);
            assert_eq!(actual, expected, "references at {}:{offset} ({probe:?}, declaration {include})", path.display());
            // Any location outside the document itself.
            if expected.iter().any(|location| location.path != path) {
                answered.insert((probe, include));
            }
        }
    }
    answered
}

/// Heading rename at every heading. Returns how many renames edited a file
/// other than the heading's own.
fn check_heading_renames(store: &mut Store, oracle: &Oracle) -> usize {
    let mut cross_document = 0;
    for (path, offset, probe) in probes(oracle) {
        if probe != Probe::Heading {
            continue;
        }
        let position = oracle.position_at(&path, offset);
        let graph = store.graph(oracle, std::slice::from_ref(&path));
        let expected = oracle.rename(&path, position, "Renamed Title");
        assert_eq!(
            oracle.rename_over(&graph, &path, position, "Renamed Title"),
            expected,
            "heading rename at {}:{offset}",
            path.display()
        );
        if expected.is_ok_and(|edits| edits.iter().any(|edit| edit.path != path)) {
            cross_document += 1;
        }
    }
    cross_document
}

/// Every file renamed within its directory, into a new directory, and onto
/// another listed file; plus a directory rename. Returns how many renames
/// produced edits.
fn check_file_renames(store: &mut Store, oracle: &Oracle, corpus: &Corpus) -> usize {
    let files = oracle.files();
    let mut renames: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (index, file) in files.iter().enumerate() {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        renames.push((file.clone(), file.with_file_name(format!("moved-{name}"))));
        renames.push((file.clone(), corpus.ws.join("elsewhere").join(&name)));
        // A collision: the destination is already listed.
        renames.push((file.clone(), files[(index + 1) % files.len()].clone()));
    }
    renames.push((corpus.ws.join("notes"), corpus.ws.join("renamed-notes")));
    let mut edited = 0;
    for (old, new) in renames {
        let pair = [(old.clone(), new.clone())];
        let graph = store.graph(oracle, std::slice::from_ref(&old));
        let expected = oracle.will_rename_files(&pair);
        assert_eq!(oracle.will_rename_files_over(&graph, &pair), expected, "rename {} -> {}", old.display(), new.display());
        if expected.is_some() {
            edited += 1;
        }
    }
    edited
}

const QUERIES: [&str; 7] = ["", "topic", "spec", "inner", "guide", "a", "no such symbol"];

fn check_workspace_symbols(store: &mut Store, oracle: &Oracle) {
    store.publish_late();
    let documents = store.facts.symbol_documents(&store.listed, std::iter::empty());
    for query in QUERIES {
        assert_eq!(oracle.workspace_symbols_over(&documents, query), oracle.workspace_symbols(query), "query {query:?}");
    }
}

/// Every check, for a store side whose facts may live in memory.
fn check_all_requests(store: &mut Store, oracle: &Oracle, corpus: &Corpus) {
    assert!(!check_references(store, oracle).is_empty());
    assert!(check_heading_renames(store, oracle) > 0);
    assert!(check_file_renames(store, oracle, corpus) > 0);
    check_workspace_symbols(store, oracle);
}

/// Acceptance 4, references: heading backlinks, link targets, wiki
/// backlinks, and DSL transcluded-by all equal the oracle's at every probe.
#[test]
fn references_equal_the_oracle_at_every_probe() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let mut store = Store::fill(&corpus, &oracle, &cache_location(&corpus));
    let answered = check_references(&mut store, &oracle);
    // Not vacuous: every probe kind found a referrer in another file.
    for probe in [Probe::Heading, Probe::Link, Probe::WikiLink, Probe::Transclusion] {
        assert!(answered.contains(&(probe, false)), "no {probe:?} probe answered across files: {answered:?}");
    }
}

/// Acceptance 4, heading rename: the same edits (or the same refusal) at
/// every heading.
#[test]
fn heading_renames_equal_the_oracle_at_every_heading() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let mut store = Store::fill(&corpus, &oracle, &cache_location(&corpus));
    // The corpus links one heading from other files (`docs/b.md`'s, by a
    // path link, a bare root-relative link, and a wiki heading link);
    // the server-level sessions add more.
    let cross_document = check_heading_renames(&mut store, &oracle);
    assert!(cross_document >= 1, "{cross_document} renames edited another file");
}

/// Acceptance 4, file rename: the same edits, wiki spellings, and refusals
/// for every file; a directory rename makes no edits.
#[test]
fn file_renames_equal_the_oracle_including_wiki_spellings() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let mut store = Store::fill(&corpus, &oracle, &cache_location(&corpus));
    assert!(check_file_renames(&mut store, &oracle, &corpus) >= 3);
    let directory = [(corpus.ws.join("notes"), corpus.ws.join("renamed-notes"))];
    let graph = store.graph(&oracle, &[corpus.ws.join("notes")]);
    assert_eq!(oracle.will_rename_files_over(&graph, &directory), None);
}

/// Acceptance 4, workspace symbols, and R8: a file under both `ws` and the
/// nested root has a row in each store but appears once.
#[test]
fn workspace_symbols_equal_the_oracle_and_list_a_nested_file_once() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let mut store = Store::fill(&corpus, &oracle, &cache_location(&corpus));
    check_workspace_symbols(&mut store, &oracle);
    let inner = corpus.nested.join("inner.md");
    let documents = store.facts.symbol_documents(&store.listed, std::iter::empty());
    assert_eq!(documents.iter().filter(|(path, _)| *path == inner).count(), 1);
    let titles = oracle.workspace_symbols_over(&documents, "");
    let inner_titles = titles
        .iter()
        .filter(|symbol| symbol.location.path == inner && symbol.kind == lsp_types::SymbolKind::FILE)
        .count();
    assert_eq!(inner_titles, 1);
}

/// R3: wiki diagnostics judged in a request graph equal the oracle's for
/// every file, and an ambiguous `[[topic]]` stays ambiguous though neither
/// rival is in the graph.
#[test]
fn wiki_ambiguity_is_judged_against_the_whole_file_list() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let mut store = Store::fill(&corpus, &oracle, &cache_location(&corpus));
    for path in oracle.files() {
        let graph = store.graph(&oracle, std::slice::from_ref(&path));
        assert_eq!(oracle.wiki_diagnostics_over(&graph, &path), oracle.wiki_diagnostics(&path), "{}", path.display());
    }
    let wiki = corpus.ws.join("notes/wiki.md");
    let graph = store.graph(&oracle, std::slice::from_ref(&wiki));
    for rival in ["alpha/topic.md", "beta/topic.md"] {
        assert!(graph.document_id(&corpus.ws.join(rival)).is_none(), "{rival} is not a candidate of wiki.md");
    }
    let diagnostics = oracle.wiki_diagnostics_over(&graph, &wiki);
    assert!(
        diagnostics.iter().any(|(_, _, message)| message.to_lowercase().contains("ambiguous")),
        "{diagnostics:?}"
    );
}

/// Acceptance 5 (R2): for every target, the candidates hold every document
/// the oracle resolves a reference or transclusion from, including the
/// bare repository-root-relative link in `docs/guide.md`.
#[test]
fn candidates_never_miss_an_oracle_referrer() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let mut store = Store::fill(&corpus, &oracle, &cache_location(&corpus));
    let mut pairs = 0;
    for target in oracle.files() {
        let referrers = oracle_referrers(&oracle, &target);
        let candidates: BTreeSet<PathBuf> =
            store.facts.candidates_for(std::slice::from_ref(&target), &store.listed).into_iter().collect();
        let missed: Vec<&PathBuf> = referrers.difference(&candidates).collect();
        assert!(missed.is_empty(), "candidates of {} miss {missed:?}", target.display());
        pairs += referrers.len();
    }
    assert!(pairs >= 10, "only {pairs} referrer pairs: the corpus no longer exercises the keys");
    let a = corpus.ws.join("docs/a.md");
    let candidates = store.facts.candidates_for(std::slice::from_ref(&a), &store.listed);
    assert!(candidates.contains(&corpus.ws.join("docs/guide.md")), "{candidates:?}");
}

/// Every other document the oracle resolves a reference or transclusion
/// to `target` (its file or one of its headings) from.
fn oracle_referrers(oracle: &Oracle, target: &Path) -> BTreeSet<PathBuf> {
    let graph = oracle.graph();
    let document = graph.document_id(target).unwrap();
    let record = graph.document(document).unwrap();
    let nodes = std::iter::once(record.root).chain(graph.headings(document).map(|(node, _)| node));
    let mut referrers = BTreeSet::new();
    for node in nodes {
        for kind in [EdgeKind::References, EdgeKind::Transcludes] {
            for edge in graph.incoming(node, kind) {
                let source = graph.node(edge.source).unwrap().document;
                referrers.insert(graph.document(source).unwrap().path.clone());
            }
        }
    }
    referrers.remove(target);
    referrers
}

/// A row for a file outside this process's file list (another process
/// listed it) is never a candidate.
#[test]
fn a_row_outside_the_file_list_is_not_a_candidate() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let mut store = Store::fill(&corpus, &oracle, &cache_location(&corpus));
    let a = corpus.ws.join("docs/a.md");
    let guide = corpus.ws.join("docs/guide.md");
    let mut listed = store.listed.clone();
    listed.remove(&guide);
    assert!(!store.facts.candidates_for(std::slice::from_ref(&a), &listed).contains(&guide));
    assert!(store.facts.candidates_for(std::slice::from_ref(&a), &store.listed).contains(&guide));
}

/// The number of memory-fallback `warn` lines in `logs`.
fn fallback_warns(logs: &CapturedLogs) -> usize {
    logs.matching("index store unavailable, using memory").iter().filter(|line| line.contains("WARN")).count()
}

/// Acceptance 8, `DMLS_CACHE_DIR=off`: every request equals the oracle from
/// memory, with one `warn` for all three roots, and no store file exists.
#[test]
fn with_the_stores_off_every_request_answers_from_memory() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let ((), logs) = CapturedLogs::scoped(|| {
        let mut store = Store::fill(&corpus, &oracle, &StoreLocation::Disabled);
        check_all_requests(&mut store, &oracle, &corpus);
    });
    assert_eq!(fallback_warns(&logs), 1, "{:#?}", logs.lines());
    assert!(!corpus.base.join("cache").exists());
}

/// Acceptance 8, read-only store directory.
#[cfg(unix)]
#[test]
fn with_a_read_only_store_directory_every_request_answers_from_memory() {
    use std::os::unix::fs::PermissionsExt;
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let location = cache_location(&corpus);
    let StoreLocation::Directory(dir) = &location else { unreachable!() };
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let ((), logs) = CapturedLogs::scoped(|| {
        let mut store = Store::fill(&corpus, &oracle, &location);
        check_all_requests(&mut store, &oracle, &corpus);
    });
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(fallback_warns(&logs), 1, "{:#?}", logs.lines());
    assert_eq!(std::fs::read_dir(dir).unwrap().count(), 0, "nothing written");
}

/// Acceptance 8, corrupt and locked: every store is garbage while another
/// holder has it open, so repair cannot take the exclusive lock and every
/// root uses memory.
#[test]
fn with_corrupt_stores_held_by_another_process_every_request_answers_from_memory() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let location = cache_location(&corpus);
    let StoreLocation::Directory(dir) = &location else { unreachable!() };
    let holders: Vec<StoreLock> = corpus
        .roots()
        .iter()
        .map(|root| {
            let path = store_path(dir, root);
            std::fs::write(&path, b"this is not a database").unwrap();
            StoreLock::shared(&lock_path(&path), std::time::Duration::from_secs(1)).unwrap()
        })
        .collect();
    let ((), logs) = CapturedLogs::scoped(|| {
        let mut store = Store::fill(&corpus, &oracle, &location);
        check_all_requests(&mut store, &oracle, &corpus);
    });
    drop(holders);
    assert_eq!(fallback_warns(&logs), 1, "{:#?}", logs.lines());
    for root in corpus.roots() {
        assert_eq!(std::fs::read(store_path(dir, &root)).unwrap(), b"this is not a database", "left untouched");
    }
}

/// A corrupt store nobody holds is recreated (no memory fallback), and
/// every request still equals the oracle.
#[test]
fn with_corrupt_stores_nobody_holds_the_stores_are_recreated_and_answer() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let location = cache_location(&corpus);
    let StoreLocation::Directory(dir) = &location else { unreachable!() };
    for root in corpus.roots() {
        std::fs::write(store_path(dir, &root), b"this is not a database").unwrap();
    }
    let ((), logs) = CapturedLogs::scoped(|| {
        let mut store = Store::fill(&corpus, &oracle, &location);
        check_all_requests(&mut store, &oracle, &corpus);
    });
    assert_eq!(fallback_warns(&logs), 0, "{:#?}", logs.lines());
    assert_eq!(logs.matching("index store recreated").len(), 3, "{:#?}", logs.lines());
}

/// The candidates of `target`, after the late writer (if any) commits.
fn late_candidates(store: &mut Store, target: &Path) -> Vec<PathBuf> {
    store.publish_late();
    store.facts.candidates_for(&[target.to_path_buf()], &store.listed)
}

/// Every request check, each one asked just after another writer committed
/// an older row: all four reference probe kinds, heading rename, file
/// rename, and workspace symbols equal the oracle's.
fn check_every_request_after_a_late_row(store: &mut Store, oracle: &Oracle, corpus: &Corpus) {
    let answered = check_references(store, oracle);
    for probe in [Probe::Heading, Probe::Link, Probe::WikiLink, Probe::Transclusion] {
        assert!(answered.contains(&(probe, false)), "no {probe:?} probe answered across files: {answered:?}");
    }
    assert!(check_heading_renames(store, oracle) >= 1);
    assert!(check_file_renames(store, oracle, corpus) >= 3);
    check_workspace_symbols(store, oracle);
}

/// R11: after ready, another writer commits the facts of an earlier
/// `notes/wiki.md` (only `# Older`, no `[[b#Some Heading]]`). Ready still
/// holds, yet no answer may follow that row: the wiki file stays a
/// candidate of `docs/b.md`, its documents and symbols are its current
/// bytes', and every request equals the disk oracle.
#[test]
fn a_late_older_wiki_row_from_another_writer_changes_no_answer() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let location = cache_location(&corpus);
    let mut store = Store::fill(&corpus, &oracle, &location);
    let wiki = corpus.ws.join("notes/wiki.md");
    let b = corpus.ws.join("docs/b.md");
    assert!(late_candidates(&mut store, &b).contains(&wiki), "control: the wiki file is a candidate");
    // Not vacuous: the oracle's heading rename and file rename of `docs/b.md`
    // both edit the wiki file.
    let heading = oracle.position_of(&b, "## Some Heading");
    assert!(oracle.rename(&b, heading, "Renamed").unwrap().iter().any(|edit| edit.path == wiki));
    let moved = [(b.clone(), b.with_file_name("moved.md"))];
    assert!(oracle.will_rename_files(&moved).unwrap().iter().any(|edit| edit.path == wiki));

    store.late = Some(LateWriter::open(&corpus, &location, &wiki, "# Older\n"));
    assert!(late_candidates(&mut store, &b).contains(&wiki), "an older row's missing key excluded a referrer");
    assert!(store.facts.is_ready(&store.listed, &corpus.roots()));

    store.publish_late();
    let documents = store.facts.request_documents(std::slice::from_ref(&wiki), []);
    assert_eq!(Some(documents[&wiki].content_hash), store.facts.verified_hash(&wiki));
    let titles: Vec<&str> = documents[&wiki].headings.iter().map(|heading| heading.title.as_str()).collect();
    assert_eq!(titles, ["Wiki Links"]);

    let graph = store.graph(&oracle, std::slice::from_ref(&b));
    assert_eq!(oracle.rename_over(&graph, &b, heading, "Renamed"), oracle.rename(&b, heading, "Renamed"));
    let graph = store.graph(&oracle, std::slice::from_ref(&b));
    assert_eq!(oracle.will_rename_files_over(&graph, &moved), oracle.will_rename_files(&moved));

    check_every_request_after_a_late_row(&mut store, &oracle, &corpus);
}

/// The same for DSL transclusions: another writer commits an earlier
/// `docs/compose.md` without its `::file ./a.md`, and `docs/a.md`'s
/// transcluded-by references still equal the oracle's.
#[test]
fn a_late_older_transcluding_row_from_another_writer_changes_no_answer() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let location = cache_location(&corpus);
    let mut store = Store::fill(&corpus, &oracle, &location);
    let compose = corpus.ws.join("docs/compose.md");
    let a = corpus.ws.join("docs/a.md");
    assert!(late_candidates(&mut store, &a).contains(&compose), "control: the composing file is a candidate");

    store.late = Some(LateWriter::open(&corpus, &location, &compose, "# Compose\n\n::code ../src/snippet.txt\n"));
    assert!(late_candidates(&mut store, &a).contains(&compose), "an older row's missing key excluded a referrer");
    // Asked from the other transcluding file, so the composing file is only
    // a referrer the store must find.
    let also = corpus.ws.join("notes/also.md");
    let transclusion = oracle.position_of(&also, "../docs/a.md");
    let expected = oracle.references(&also, transclusion, false);
    assert!(expected.iter().any(|location| location.path == compose), "{expected:?}");
    let graph = store.graph(&oracle, &reference_targets(&oracle, &also, oracle.offset_at(&also, transclusion)));
    assert_eq!(oracle.references_over(&graph, &also, transclusion, false), expected);

    check_every_request_after_a_late_row(&mut store, &oracle, &corpus);
}

/// A refresh replaces a row that does not match the bytes it reads, and a
/// row another writer commits after a refresh found its row current changes
/// no answer either.
#[test]
fn a_row_committed_after_a_refresh_checked_it_changes_no_answer() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let location = cache_location(&corpus);
    let mut store = Store::fill(&corpus, &oracle, &location);
    let wiki = corpus.ws.join("notes/wiki.md");
    let mut late = LateWriter::open(&corpus, &location, &wiki, "# Older\n");

    late.publish(&store.facts);
    let report = store.facts.refresh(std::slice::from_ref(&wiki), &BTreeMap::new());
    assert_eq!((report.read, report.reparsed), (1, 1), "{report:?}");
    assert_eq!(Some(late.store.hashes().unwrap()[&wiki]), store.facts.verified_hash(&wiki), "the older row was replaced");

    let report = store.facts.refresh(std::slice::from_ref(&wiki), &BTreeMap::new());
    assert_eq!((report.read, report.reparsed), (1, 0), "{report:?}");
    // The other writer commits after that refresh checked the row.
    store.late = Some(late);
    check_every_request_after_a_late_row(&mut store, &oracle, &corpus);
}

/// Rows of the `ws` store selected as candidates of `docs/b.md` (key `b`)
/// get a path that is not UTF-8, so the candidate read cannot decode them.
const UNDECODABLE_CANDIDATE_PATHS: &str =
    "UPDATE files SET path = CAST(X'FF' AS TEXT) || id WHERE id IN (SELECT file_id FROM cand WHERE key = 'b')";

/// Every stored heading of the `ws` store gets level 9, so loading facts
/// and scanning headings cannot decode them.
const UNDECODABLE_HEADING_LEVELS: &str = "UPDATE headings SET level = 9";

/// Damages the `ws` store through another connection after ready. The files
/// on disk stay valid and readable; only the cache stops decoding, and no
/// salvage of the whole store can succeed.
fn damage(corpus: &Corpus, location: &StoreLocation, sql: &str) {
    let StoreLocation::Directory(dir) = location else { panic!("expected a store directory, got {location:?}") };
    let conn = rusqlite::Connection::open(store_path(dir, &corpus.ws)).unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
    assert!(conn.execute(sql, []).unwrap() > 0, "the damage changed no row: {sql}");
}

/// A filled, ready store whose answers equal the oracle (the control), then
/// damaged by `sql`.
fn damaged_after_ready(sql: &str) -> (Corpus, Oracle, Store) {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let location = cache_location(&corpus);
    let mut store = Store::fill(&corpus, &oracle, &location);
    check_workspace_symbols(&mut store, &oracle);
    let b = corpus.ws.join("docs/b.md");
    let heading = oracle.position_of(&b, "## Some Heading");
    let graph = store.graph(&oracle, std::slice::from_ref(&b));
    assert_eq!(oracle.references_over(&graph, &b, heading, false), oracle.references(&b, heading, false), "control");
    damage(&corpus, &location, sql);
    (corpus, oracle, store)
}

/// The in-request switch withdrew ready, so the router's next request
/// queues a build; that build (a revalidation) refills memory, and every
/// request still equals the oracle afterwards.
fn assert_withdrawn_then_reconciled(store: &mut Store, oracle: &Oracle, corpus: &Corpus) {
    assert!(!store.facts.is_ready(&store.listed, &corpus.roots()), "the switch to memory withdrew ready");
    let files = oracle.files();
    let report = store.facts.revalidate(&files, &BTreeMap::new(), &SilentProgress, &AtomicBool::new(false));
    assert!(report.failed.is_empty() && report.missing.is_empty() && !report.switched, "{report:?}");
    store.facts.mark_ready();
    assert!(store.facts.is_ready(&store.listed, &corpus.roots()));
    check_all_requests(store, oracle, corpus);
}

/// R12: a candidate row that stops decoding after ready switches its root
/// to memory with nothing salvaged, yet the first candidate set already
/// holds every oracle referrer (and equals the set before the damage),
/// for every target; later requests and the reconciling build agree.
#[test]
fn an_undecodable_candidate_row_after_ready_leaves_the_first_candidates_complete() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let location = cache_location(&corpus);
    let mut store = Store::fill(&corpus, &oracle, &location);
    let b = corpus.ws.join("docs/b.md");
    let referrers = oracle_referrers(&oracle, &b);
    assert!(referrers.contains(&corpus.ws.join("notes/wiki.md")), "not vacuous: {referrers:?}");
    let control: BTreeSet<PathBuf> = store.facts.candidates_for(std::slice::from_ref(&b), &store.listed).into_iter().collect();
    assert!(referrers.is_subset(&control), "control: {control:?}");
    damage(&corpus, &location, UNDECODABLE_CANDIDATE_PATHS);

    let first: BTreeSet<PathBuf> = store.facts.candidates_for(std::slice::from_ref(&b), &store.listed).into_iter().collect();
    assert_eq!(first, control, "the first candidate set after the switch");
    assert!(!store.facts.is_ready(&store.listed, &corpus.roots()), "the switch to memory withdrew ready");
    for target in oracle.files() {
        let candidates: BTreeSet<PathBuf> =
            store.facts.candidates_for(std::slice::from_ref(&target), &store.listed).into_iter().collect();
        let referrers = oracle_referrers(&oracle, &target);
        assert!(referrers.is_subset(&candidates), "candidates of {} miss {:?}", target.display(), referrers.difference(&candidates));
    }
    assert_withdrawn_then_reconciled(&mut store, &oracle, &corpus);
}

/// R12: stored headings that stop decoding after ready, with the request
/// documents as the first reader: every requested file is there, with this
/// process's read of its bytes.
#[test]
fn undecodable_headings_after_ready_leave_the_first_request_documents_complete() {
    let (corpus, oracle, mut store) = damaged_after_ready(UNDECODABLE_HEADING_LEVELS);
    let files = oracle.files();
    let documents = store.facts.request_documents(&files, []);
    assert_eq!(documents.keys().cloned().collect::<Vec<_>>(), files);
    for (path, document) in &documents {
        assert_eq!(Some(document.content_hash), store.facts.verified_hash(path), "{}", path.display());
        let disk = index_document(path, &std::fs::read_to_string(path).unwrap());
        assert_eq!(document.headings, disk.headings, "{}", path.display());
    }
    assert_withdrawn_then_reconciled(&mut store, &oracle, &corpus);
}

/// R12: the same damage with a request graph as the first reader: the
/// first references answer at `docs/b.md`'s heading (a path link, a bare
/// root-relative link, and a wiki heading link) equals the oracle's, and
/// so does every later request at every probe.
#[test]
fn undecodable_headings_after_ready_leave_the_first_request_graph_complete() {
    let (corpus, oracle, mut store) = damaged_after_ready(UNDECODABLE_HEADING_LEVELS);
    let b = corpus.ws.join("docs/b.md");
    let heading = oracle.position_of(&b, "## Some Heading");
    let expected = oracle.references(&b, heading, false);
    assert!(expected.iter().any(|location| location.path == corpus.ws.join("notes/wiki.md")), "{expected:?}");
    let graph = store.graph(&oracle, &reference_targets(&oracle, &b, oracle.offset_at(&b, heading)));
    assert_eq!(oracle.references_over(&graph, &b, heading, false), expected, "the first answer after the switch");
    assert!(!store.facts.is_ready(&store.listed, &corpus.roots()), "the switch to memory withdrew ready");
    check_all_requests(&mut store, &oracle, &corpus);
    assert_withdrawn_then_reconciled(&mut store, &oracle, &corpus);
}

/// R12: the same damage with rename's pre-graph refresh as the first reader
/// (as the router does it: the candidates, then a refresh of them and the
/// target, then the graph). The heading rename and file rename of
/// `docs/b.md` equal the oracle's, including the wiki referrer's edits.
#[test]
fn undecodable_headings_first_read_by_a_rename_refresh_leave_the_renames_complete() {
    let (corpus, oracle, mut store) = damaged_after_ready(UNDECODABLE_HEADING_LEVELS);
    let b = corpus.ws.join("docs/b.md");
    let wiki = corpus.ws.join("notes/wiki.md");
    let targets = std::slice::from_ref(&b);
    let refresh_then_graph = |store: &mut Store| {
        let mut paths: Vec<PathBuf> = store.facts.candidates_for(targets, &store.listed);
        paths.push(b.clone());
        store.facts.refresh(&paths, &BTreeMap::new());
        store.graph(&oracle, targets)
    };

    let graph = refresh_then_graph(&mut store);
    assert!(!store.facts.is_ready(&store.listed, &corpus.roots()), "the switch to memory withdrew ready");
    let heading = oracle.position_of(&b, "## Some Heading");
    let expected = oracle.rename(&b, heading, "Renamed");
    assert!(expected.as_ref().unwrap().iter().any(|edit| edit.path == wiki), "{expected:?}");
    assert_eq!(oracle.rename_over(&graph, &b, heading, "Renamed"), expected, "the first rename after the switch");

    let moved = [(b.clone(), b.with_file_name("moved.md"))];
    let graph = refresh_then_graph(&mut store);
    let expected = oracle.will_rename_files(&moved);
    assert!(expected.as_ref().unwrap().iter().any(|edit| edit.path == wiki), "{expected:?}");
    assert_eq!(oracle.will_rename_files_over(&graph, &moved), expected);
    assert_withdrawn_then_reconciled(&mut store, &oracle, &corpus);
}

/// R12: the same damage with the heading scan as the first reader: the
/// first workspace-symbols answer equals the oracle's for every query.
#[test]
fn undecodable_headings_after_ready_leave_the_first_workspace_symbols_complete() {
    let (corpus, oracle, mut store) = damaged_after_ready(UNDECODABLE_HEADING_LEVELS);
    // The control already scanned once; this is the first scan since the
    // damage.
    check_workspace_symbols(&mut store, &oracle);
    assert!(!store.facts.is_ready(&store.listed, &corpus.roots()), "the switch to memory withdrew ready");
    check_workspace_symbols(&mut store, &oracle);
    assert_withdrawn_then_reconciled(&mut store, &oracle, &corpus);
}
