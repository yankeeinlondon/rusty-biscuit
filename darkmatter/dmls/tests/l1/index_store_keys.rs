//! R2 over-approximation: for every document of the corpus, the store's
//! candidate set holds every file the full-graph oracle says references or
//! transcludes it.
//!
//! "True referrers" are the source documents of `references` and
//! `transcludes` edges into a target's root or headings: the only edge kinds
//! the references, rename, wiki, and DSL providers query.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dmls::clock::SystemClock;
use dmls::graph::{EdgeKind, WorkspaceGraph, index_document};
use dmls::index_store::{FactsSource, FileFacts, IndexStore, MemoryFacts, store_path};

use crate::common::oracle::Corpus;

/// Every target document's true referrers, other than itself.
fn true_referrers(graph: &WorkspaceGraph) -> BTreeMap<PathBuf, BTreeSet<PathBuf>> {
    let mut out = BTreeMap::new();
    for record in graph.documents() {
        let document = graph.document_id(record.path).expect("document id");
        let targets = std::iter::once(record.root).chain(graph.headings(document).map(|(id, _)| id));
        let mut referrers = BTreeSet::new();
        for node in targets {
            for kind in [EdgeKind::References, EdgeKind::Transcludes] {
                for edge in graph.incoming(node, kind) {
                    let source = graph.node(edge.source).expect("edge source").document;
                    let path = graph.document(source).expect("source document").path;
                    if path != record.path {
                        referrers.insert(path.to_path_buf());
                    }
                }
            }
        }
        out.insert(record.path.to_path_buf(), referrers);
    }
    out
}

#[test]
fn candidates_never_miss_a_true_referrer_on_the_corpus() {
    let corpus = Corpus::materialize(&biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    // S1's missed shape, alone in its file: bare paths written relative to
    // the repository root from a nested directory, so only the file-name key
    // can find them.
    let root_relative = corpus.ws.join("docs/topics/root-relative.md");
    std::fs::create_dir_all(root_relative.parent().unwrap()).unwrap();
    std::fs::write(&root_relative, "# Root Relative\n\n[b](docs/b.md)\n\n::file docs/sub/c.md\n").unwrap();
    let oracle = corpus.oracle();

    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) =
        IndexStore::open(&store_path(dir.path(), &corpus.ws), &corpus.ws, Arc::new(SystemClock)).unwrap();
    let facts: Vec<FileFacts> = oracle
        .files()
        .iter()
        .map(|path| FileFacts::from_index(&index_document(path, &std::fs::read_to_string(path).unwrap())))
        .collect();
    store.upsert_batch(&facts).unwrap();
    let mut memory = MemoryFacts::default();
    memory.upsert(&facts).unwrap();

    let referrers = true_referrers(oracle.graph());
    // Non-vacuous: the oracle resolves the root-relative forms, and the
    // corpus has pairs of every link family.
    assert!(referrers[&corpus.ws.join("docs/b.md")].contains(&root_relative), "{referrers:#?}");
    assert!(referrers[&corpus.ws.join("docs/sub/c.md")].contains(&root_relative), "{referrers:#?}");
    let pairs: usize = referrers.values().map(BTreeSet::len).sum();
    assert!(pairs >= 15, "only {pairs} true pairs: {referrers:#?}");

    for (target, expected) in &referrers {
        let candidates: BTreeSet<PathBuf> = store.candidates_for(target).unwrap().into_iter().collect();
        let missed: Vec<&PathBuf> = expected.difference(&candidates).collect();
        assert!(missed.is_empty(), "{} misses referrers {missed:?}", target.display());
        assert_eq!(
            memory.candidates_for(target).unwrap(),
            store.candidates_for(target).unwrap(),
            "memory and store disagree for {}",
            target.display()
        );
    }
}

#[test]
fn an_explicitly_relative_link_is_not_a_candidate_for_a_same_named_file_elsewhere() {
    let dir = tempfile::tempdir().unwrap();
    let base = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let (mut store, _) = IndexStore::open(&base.join("s.sqlite"), &base, Arc::new(SystemClock)).unwrap();
    let source = base.join("docs/topics/a.md");
    store.upsert_batch(&[FileFacts::from_index(&index_document(&source, "[x](../b.md)\n"))]).unwrap();
    assert_eq!(store.candidates_for(&base.join("docs/b.md")).unwrap(), vec![source.clone()]);
    assert!(store.candidates_for(&base.join("other/b.md")).unwrap().is_empty());
    assert!(store.candidates_for(Path::new("/elsewhere/b.md")).unwrap().is_empty());
}
