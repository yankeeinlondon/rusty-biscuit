//! A graph over a few documents, matching wiki links against the whole file
//! list (`WikiUniverse`), answers like the full-workspace oracle.
//!
//! Each test builds a scoped graph over a target and its referrers only, the
//! shape the index store builds per request, and compares it with the oracle
//! over every corpus file. Where the rival that decides an answer is left out
//! of the scoped documents, a negative control scoped to its own documents
//! (`UniverseScope::Documents`) shows that the universe is what keeps the
//! answer equal.

use std::path::{Path, PathBuf};

use crate::common::oracle::{Corpus, Oracle, UniverseScope};

fn corpus() -> Corpus {
    Corpus::materialize(&biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"))
}

fn scoped(corpus: &Corpus, documents: &[&str], universe: UniverseScope) -> Oracle {
    let documents: Vec<PathBuf> = documents.iter().map(|path| corpus.base.join(path)).collect();
    Oracle::scoped(&corpus.roots(), corpus.snapshot(), &documents, universe)
}

fn ambiguous_lines(diagnostics: &[(u32, String, String)]) -> Vec<u32> {
    diagnostics
        .iter()
        .filter(|(_, code, _)| code == "wiki.ambiguous-target")
        .map(|(line, _, _)| *line)
        .collect()
}

fn references_at(oracle: &Oracle, path: &Path, needle: &str) -> Vec<crate::common::oracle::OracleLocation> {
    oracle.references(path, oracle.position_of(path, needle), false)
}

#[test]
fn ambiguous_wiki_link_stays_ambiguous_when_its_rival_is_not_a_document() {
    let corpus = corpus();
    let oracle = corpus.oracle();
    let wiki = corpus.ws.join("notes/wiki.md");
    // `[[topic]]` (line 9) matches alpha/topic.md and beta/topic.md; only
    // alpha is a document of the scoped graph.
    let documents = ["ws/notes/wiki.md", "ws/alpha/topic.md"];
    let scoped = scoped(&corpus, &documents, UniverseScope::FileList);

    let expected = oracle.wiki_diagnostics(&wiki);
    assert_eq!(ambiguous_lines(&expected), vec![9]);
    assert_eq!(scoped.wiki_diagnostics(&wiki), expected);
    for needle in ["[[topic]]", "[[alpha/topic]]"] {
        assert_eq!(references_at(&scoped, &wiki, needle), references_at(&oracle, &wiki, needle), "{needle}");
    }

    // Control: judged against its own two documents, `[[topic]]` is unique.
    let own = scoped_control(&corpus, &documents);
    assert_eq!(ambiguous_lines(&own.wiki_diagnostics(&wiki)), Vec::<u32>::new());
}

#[test]
fn wiki_link_to_a_listed_file_outside_the_graph_is_not_reported_unresolved() {
    let corpus = corpus();
    let oracle = corpus.oracle();
    let wiki = corpus.ws.join("notes/wiki.md");
    // Every target of wiki.md except itself is left out of the graph.
    let scoped = scoped(&corpus, &["ws/notes/wiki.md"], UniverseScope::FileList);

    assert_eq!(scoped.wiki_diagnostics(&wiki), oracle.wiki_diagnostics(&wiki));
    // A cursor on a link whose target is outside the graph has nothing to
    // answer for in it.
    assert_eq!(references_at(&scoped, &wiki, "[[note]]"), Vec::new());
}

#[test]
fn file_rename_spelling_avoids_a_rival_that_is_not_a_document() {
    let corpus = corpus();
    let oracle = corpus.oracle();
    // `[[spec]]` and `[[notes/spec]]` in wiki.md resolve to notes/spec.md.
    // Renamed to notes/deep/topic.md, the bare `topic` would collide with
    // alpha/ and beta/topic.md, which are not documents of the scoped graph.
    let rename = [(corpus.ws.join("notes/spec.md"), corpus.ws.join("notes/deep/topic.md"))];
    let documents = ["ws/notes/wiki.md", "ws/notes/spec.md"];

    let expected = oracle.will_rename_files(&rename).expect("the rename is safe");
    assert!(expected.iter().all(|edit| edit.new_text == "deep/topic"), "{expected:?}");
    assert_eq!(expected.len(), 2, "{expected:?}");
    let scoped = scoped(&corpus, &documents, UniverseScope::FileList);
    assert_eq!(scoped.will_rename_files(&rename), Some(expected.clone()));

    // Control: without the rivals the shortest spelling is the bare name.
    let own = scoped_control(&corpus, &documents).will_rename_files(&rename).expect("safe");
    assert_ne!(own, expected);
    assert!(own.iter().all(|edit| edit.new_text == "topic"), "{own:?}");
}

#[test]
fn file_rename_onto_a_listed_file_outside_the_graph_is_refused() {
    let corpus = corpus();
    let oracle = corpus.oracle();
    let rename = [(corpus.ws.join("notes/spec.md"), corpus.ws.join("alpha/topic.md"))];
    let documents = ["ws/notes/wiki.md", "ws/notes/spec.md"];

    assert_eq!(oracle.will_rename_files(&rename), None);
    assert_eq!(scoped(&corpus, &documents, UniverseScope::FileList).will_rename_files(&rename), None);
    // Control: a graph that cannot see the destination would rename onto it.
    assert!(scoped_control(&corpus, &documents).will_rename_files(&rename).is_some());
}

#[test]
fn cross_root_file_rename_over_its_referrers_equals_the_oracle() {
    let corpus = corpus();
    let oracle = corpus.oracle();
    // notes/note.md is named by `[[note]]` in wiki.md and in the second
    // root's other.md, and linked from index.md.
    let rename = [(corpus.ws.join("notes/note.md"), corpus.ws.join("notes/renamed-note.md"))];
    let documents = ["ws/notes/note.md", "ws/notes/wiki.md", "other/other.md", "ws/index.md"];

    let expected = oracle.will_rename_files(&rename).expect("the rename is safe");
    assert_eq!(scoped(&corpus, &documents, UniverseScope::FileList).will_rename_files(&rename), Some(expected));

    let note = corpus.ws.join("notes/note.md");
    let wiki = corpus.ws.join("notes/wiki.md");
    let scoped = scoped(&corpus, &documents, UniverseScope::FileList);
    assert_eq!(references_at(&scoped, &wiki, "[[note]]"), references_at(&oracle, &wiki, "[[note]]"));
    assert!(scoped.graph().document_id(&note).is_some());
}

#[test]
fn heading_rename_over_its_referrers_equals_the_oracle() {
    let corpus = corpus();
    let oracle = corpus.oracle();
    let b = corpus.ws.join("docs/b.md");
    // b.md's `## Some Heading` is named by anchor links in b.md, guide.md and
    // index.md, and by `[[b#Some Heading]]` in wiki.md, which also names
    // files outside this set.
    let documents = ["ws/docs/b.md", "ws/docs/guide.md", "ws/index.md", "ws/notes/wiki.md"];
    let scoped = scoped(&corpus, &documents, UniverseScope::FileList);

    let heading = oracle.position_of(&b, "Some Heading");
    assert_eq!(
        scoped.rename(&b, heading, "Renamed Heading").expect("unique heading"),
        oracle.rename(&b, heading, "Renamed Heading").expect("unique heading"),
    );
    let position = oracle.position_of(&b, "## Some Heading");
    assert_eq!(scoped.references(&b, position, true), oracle.references(&b, position, true));
}

/// A scoped graph that matches wiki links against its own documents only.
fn scoped_control(corpus: &Corpus, documents: &[&str]) -> Oracle {
    scoped(corpus, documents, UniverseScope::Documents)
}
