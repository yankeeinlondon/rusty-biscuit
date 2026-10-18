//! Self-check of the full-workspace oracle (`common::oracle`) that the index
//! store's answers are later compared against.
//!
//! Every expectation here is a literal path and zero-based line (or edit
//! range) read off the corpus under `tests/fixtures/index_store_corpus/`, so a
//! defect in the oracle cannot make the store comparisons vacuous. Expectations
//! marked **current behavior** record what DMLS answers today where a reader
//! might expect otherwise; they pin the oracle, not a design decision.

use std::path::{Path, PathBuf};

use lsp_types::SymbolKind;

use crate::common::oracle::{Corpus, Oracle, OracleEdit, OracleLocation, OracleSymbol};

fn corpus(source: PathBuf) -> (Corpus, Oracle) {
    let corpus = Corpus::materialize(&source);
    let oracle = corpus.oracle();
    (corpus, oracle)
}

/// Zero-width line locations under `base`, sorted like the oracle's answers.
fn lines(base: &Path, entries: &[(&str, u32)]) -> Vec<OracleLocation> {
    let mut out: Vec<OracleLocation> =
        entries.iter().map(|(path, line)| OracleLocation::line(base.join(path), *line)).collect();
    out.sort();
    out
}

fn references_at(oracle: &Oracle, path: &Path, needle: &str) -> Vec<OracleLocation> {
    oracle.references(path, oracle.position_of(path, needle), false)
}

fn edit(base: &Path, path: &str, start: (u32, u32), end: (u32, u32), new_text: &str) -> OracleEdit {
    OracleEdit { path: base.join(path), start, end, new_text: new_text.to_string() }
}

#[test]
fn file_references_cover_relative_bare_root_relative_and_absolute_links() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let guide = corpus.ws.join("docs/guide.md");

    // `./a.md` (line 2), `a.md` (4), the repository-root-relative bare
    // `docs/a.md` (6), and the absolute path in `absolute.md` (3). Not the
    // `::file` transclusions (no references edge), `A.md`, `/docs/a.md`, the
    // `file://` URL, or the link through the symlinked directory.
    let mut expected = lines(
        &corpus.base,
        &[("ws/docs/guide.md", 2), ("ws/docs/guide.md", 4), ("ws/docs/guide.md", 6), ("ws/docs/absolute.md", 3)],
    );
    let mut answers: Vec<Vec<OracleLocation>> = ["[relative]", "[bare sibling]", "[root relative]"]
        .into_iter()
        .map(|needle| references_at(&oracle, &guide, needle))
        .collect();
    if cfg!(windows) {
        // The drive-letter absolute form is not verified from a Windows host
        // yet; every other form is.
        let absolute = corpus.ws.join("docs/absolute.md");
        expected.retain(|location| location.path != absolute);
        answers.iter_mut().for_each(|answer| answer.retain(|location| location.path != absolute));
    }
    for answer in answers {
        assert_eq!(answer, expected);
    }

    let parent = references_at(&oracle, &guide, "[parent]");
    assert_eq!(parent, lines(&corpus.base, &[("ws/docs/guide.md", 3), ("ws/index.md", 2)]));
    let nested = references_at(&oracle, &guide, "[bare nested]");
    assert_eq!(nested, lines(&corpus.base, &[("ws/docs/guide.md", 5)]));
}

#[test]
fn percent_encoded_destination_does_not_resolve_but_angle_form_does() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let guide = corpus.ws.join("docs/guide.md");

    // `<my file.md>` (line 8) is the only reference to `docs/my file.md`.
    let expected = lines(&corpus.base, &[("ws/docs/guide.md", 8)]);
    assert_eq!(references_at(&oracle, &guide, "[angle]"), expected);
    // Current behavior: `my%20file.md` is not percent-decoded, so it is an
    // unresolved link and neither has nor contributes a reference.
    assert_eq!(references_at(&oracle, &guide, "[percent]"), Vec::new());
}

#[test]
fn unresolved_and_non_document_link_forms_have_no_references() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let guide = corpus.ws.join("docs/guide.md");

    // Current behavior, the same on every OS: a directory link and a
    // `/`-prefixed path are unresolved; `A.md` (case) and the NFD spelling of
    // `café.md` are at most an existing non-document file on a
    // case- or normalization-insensitive file system, never `a.md` or
    // `café.md` the graph documents.
    for needle in ["[directory]", "[root slash]", "[case]", "[nfd]"] {
        assert_eq!(references_at(&oracle, &guide, needle), Vec::new(), "{needle}");
    }
    // A `file://` URL is an external link.
    let absolute = corpus.ws.join("docs/absolute.md");
    assert_eq!(references_at(&oracle, &absolute, "[file url]"), Vec::new());
}

#[test]
fn heading_references_follow_anchor_links_and_wiki_headings() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let b = corpus.ws.join("docs/b.md");

    // `#some-heading` in b.md (4), `b.md#some-heading` (guide 9), the
    // root-level `docs/b.md#some-heading` (index 5), `[[b#Some Heading]]`
    // (wiki 5).
    let referrers = [("ws/docs/b.md", 4), ("ws/docs/guide.md", 9), ("ws/index.md", 5), ("ws/notes/wiki.md", 5)];
    let heading = oracle.position_of(&b, "## Some Heading");
    let mut with_declaration = referrers.to_vec();
    with_declaration.push(("ws/docs/b.md", 2));
    assert_eq!(oracle.references(&b, heading, true), lines(&corpus.base, &with_declaration));
    assert_eq!(oracle.references(&b, heading, false), lines(&corpus.base, &referrers));
    // A cursor on a referring link answers about the same heading.
    let guide = corpus.ws.join("docs/guide.md");
    assert_eq!(references_at(&oracle, &guide, "[anchor]"), lines(&corpus.base, &referrers));
}

#[test]
fn wiki_references_resolve_across_roots_and_count_ambiguous_links() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let wiki = corpus.ws.join("notes/wiki.md");
    let base = &corpus.base;

    // `notes/note.md`: `[[note]]` here (2) and in the second root (other 3),
    // and the Markdown link in index.md (3).
    let note = lines(base, &[("ws/notes/wiki.md", 2), ("other/other.md", 3), ("ws/index.md", 3)]);
    assert_eq!(references_at(&oracle, &wiki, "[[note]]"), note);
    assert_eq!(references_at(&oracle, &corpus.other.join("other.md"), "[[note]]"), note);

    // `[[spec]]` from notes/ resolves to the nearer notes/spec.md, not the
    // root spec.md.
    let notes_spec = lines(base, &[("ws/notes/wiki.md", 3), ("ws/notes/wiki.md", 4)]);
    assert_eq!(references_at(&oracle, &wiki, "[[spec]]"), notes_spec);
    assert_eq!(references_at(&oracle, &wiki, "[[notes/spec]]"), notes_spec);

    // `[[topic]]` (9) is ambiguous between alpha/ and beta/. Current
    // behavior: it is a referrer of both candidates, and a cursor on it
    // answers for the first candidate (alpha/topic.md).
    let alpha = lines(base, &[("ws/notes/wiki.md", 9), ("ws/notes/wiki.md", 10)]);
    let beta = lines(base, &[("ws/notes/wiki.md", 9), ("ws/notes/wiki.md", 11)]);
    assert_eq!(references_at(&oracle, &wiki, "[[alpha/topic]]"), alpha);
    assert_eq!(references_at(&oracle, &wiki, "[[beta/topic]]"), beta);
    assert_eq!(references_at(&oracle, &wiki, "[[topic]]"), alpha);

    // An extension-elided NFD spelling resolves to the NFC-named café.md.
    assert_eq!(references_at(&oracle, &wiki, "[[cafe\u{301}]]"), lines(base, &[("ws/notes/wiki.md", 7)]));
    // Current behavior: `[[upper]]` does not resolve to docs/Upper.md.
    assert_eq!(references_at(&oracle, &wiki, "[[upper]]"), Vec::new());
}

#[test]
fn dsl_transcluded_by_lists_every_file_transclusion() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let compose = corpus.ws.join("docs/compose.md");

    // `::file ./a.md` (compose 2) and `::file ../docs/a.md` (also 2). The
    // cursor must be on the target, which is the transclusion's span.
    let expected = lines(&corpus.base, &[("ws/docs/compose.md", 2), ("ws/notes/also.md", 2)]);
    assert_eq!(references_at(&oracle, &compose, "./a.md"), expected);
    assert_eq!(references_at(&oracle, &corpus.ws.join("notes/also.md"), "../docs/a.md"), expected);
    // A `::code` target that is not a document has no transcluded-by answer.
    assert_eq!(references_at(&oracle, &compose, "../src/snippet.txt"), Vec::new());
}

#[test]
fn heading_rename_rewrites_anchor_slugs_and_wiki_heading_text() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let b = corpus.ws.join("docs/b.md");
    let base = &corpus.base;

    let edits = oracle
        .rename(&b, oracle.position_of(&b, "Some Heading"), "Renamed Heading")
        .expect("the heading is unique");
    let mut expected = vec![
        edit(base, "ws/docs/b.md", (2, 3), (2, 15), "Renamed Heading"),
        edit(base, "ws/docs/b.md", (4, 15), (4, 27), "renamed-heading"),
        edit(base, "ws/docs/guide.md", (9, 16), (9, 28), "renamed-heading"),
        edit(base, "ws/index.md", (5, 27), (5, 39), "renamed-heading"),
        edit(base, "ws/notes/wiki.md", (5, 6), (5, 18), "Renamed Heading"),
    ];
    expected.sort();
    assert_eq!(edits, expected);
}

#[test]
fn will_rename_files_repaths_markdown_links_and_respells_wiki_links() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let base = &corpus.base;

    let edits = oracle
        .will_rename_files(&[(corpus.ws.join("notes/note.md"), corpus.ws.join("notes/renamed-note.md"))])
        .expect("the rename is safe");
    let mut expected = vec![
        edit(base, "other/other.md", (3, 2), (3, 6), "renamed-note"),
        edit(base, "ws/index.md", (3, 15), (3, 28), "notes/renamed-note.md"),
        edit(base, "ws/notes/wiki.md", (2, 4), (2, 8), "renamed-note"),
    ];
    expected.sort();
    assert_eq!(edits, expected);
}

#[test]
fn workspace_symbols_match_document_titles_and_headings() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));

    let symbol = |path: &str, name: &str, kind: SymbolKind, container: Option<&str>| OracleSymbol {
        location: OracleLocation::line(corpus.base.join(path), 0),
        name: name.to_string(),
        kind,
        container: container.map(str::to_string),
    };
    assert_eq!(
        oracle.workspace_symbols("topic"),
        vec![
            symbol("ws/alpha/topic.md", "Alpha Topic", SymbolKind::FILE, None),
            symbol("ws/alpha/topic.md", "Alpha Topic", SymbolKind::STRING, Some("topic.md")),
            symbol("ws/beta/topic.md", "Beta Topic", SymbolKind::FILE, None),
            symbol("ws/beta/topic.md", "Beta Topic", SymbolKind::STRING, Some("topic.md")),
        ]
    );
    // The file listed under both `ws` and the nested root `ws/nested`
    // appears once.
    assert_eq!(
        oracle.workspace_symbols("inner"),
        vec![
            symbol("ws/nested/inner.md", "Inner Doc", SymbolKind::FILE, None),
            symbol("ws/nested/inner.md", "Inner Doc", SymbolKind::STRING, Some("inner.md")),
        ]
    );
}

#[test]
fn file_list_holds_each_corpus_document_once() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));

    // `ws/nested/inner.md` sits under two roots; `ws/linked/` is a symlink,
    // which discovery does not follow.
    let mut expected: Vec<PathBuf> = [
        "other/other.md",
        "other/peer.md",
        "ws/alpha/topic.md",
        "ws/beta/topic.md",
        "ws/docs/Upper.md",
        "ws/docs/a.md",
        "ws/docs/absolute.md",
        "ws/docs/b.md",
        "ws/docs/café.md",
        "ws/docs/compose.md",
        "ws/docs/guide.md",
        "ws/docs/my file.md",
        "ws/docs/sub/c.md",
        "ws/index.md",
        "ws/nested/inner.md",
        "ws/notes/also.md",
        "ws/notes/note.md",
        "ws/notes/spec.md",
        "ws/notes/symlinked.md",
        "ws/notes/wiki.md",
        "ws/spec.md",
    ]
    .into_iter()
    .map(|path| corpus.base.join(path))
    .collect();
    expected.sort();
    assert_eq!(oracle.files(), expected);
    assert_eq!(oracle.graph().document_count(), expected.len());
}

#[test]
fn link_through_symlinked_directory_is_not_a_reference() {
    let (corpus, oracle) = corpus(biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    if !corpus.symlink_created {
        // `Corpus::materialize` fails on Unix without the symlink, so only a
        // Windows host lacking the privilege reaches this skip.
        eprintln!("skipped: this Windows host cannot create a directory symlink");
        return;
    }
    let symlinked = corpus.ws.join("notes/symlinked.md");

    // Current behavior: `../linked/a.md` reaches docs/a.md on disk, but the
    // graph keys documents by listed path, so the link is an existing
    // non-document file and docs/a.md does not count it.
    assert_eq!(references_at(&oracle, &symlinked, "[linked a]"), Vec::new());
    let referrers = references_at(&oracle, &corpus.ws.join("docs/guide.md"), "[relative]");
    assert!(referrers.iter().all(|location| location.path != symlinked), "{referrers:?}");
}
