//! A malformed inclusion that composition rejects is an error from every
//! reference reader, never an absent inclusion: both graph modes, the local
//! transclusion query, flattened references, and reference validation (fresh
//! and with a prebuilt graph). Valid spellings keep their graph shapes, and
//! an omitted or empty `prologue`/`epilogue` still lists nothing. The same
//! holds for an included file that exists but cannot be loaded and for a
//! `when=` condition that fails; a fragment naming a file that cannot be
//! loaded is reported, never passed.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, TransclusionError};
use darkmatter::markdown::reference::ReferenceError;
use darkmatter::markdown::reference::types::{ReferenceGraphOptions, ReferenceSyntax};
use darkmatter::markdown::reference::validate::{ReferenceIssueCode, ReferenceSeverity, ReferenceValidationOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use std::path::Path;
use tempfile::TempDir;

/// A temporary repository holding the included `side.md`.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::write(repo.path().join("side.md"), "# Side\n").unwrap();
    repo
}

/// A request in the repository's frozen context, as `md validate refs` and
/// `md graph` prepare one, so nothing scans the real working tree.
fn request(root: &Path) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    crate::request_support::request_in(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")),
        context,
    )
}

/// `doc.md` written with `content` and loaded from its file.
fn document(repo: &TempDir, content: &str) -> Markdown {
    let path = repo.path().join("doc.md");
    std::fs::write(&path, content).unwrap();
    Markdown::try_from(path.as_path()).unwrap()
}

/// Which typed error composition raises for a malformed inclusion.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Family {
    /// `TransclusionError::ParseDirective` (body directive or frontmatter
    /// field); the local query reports it as `ReferenceError::ParseDirective`.
    Directive,
    TocLinking,
    FileLinks,
}

/// The family of a graph or composition error, `None` for any other error.
fn family(error: &MarkdownError) -> Option<Family> {
    match error {
        MarkdownError::Transclusion(inner) if matches!(inner.as_ref(), TransclusionError::ParseDirective { .. }) => {
            Some(Family::Directive)
        }
        MarkdownError::Reference(inner) if matches!(inner.as_ref(), ReferenceError::ParseDirective { .. }) => {
            Some(Family::Directive)
        }
        MarkdownError::TocLinking(_) => Some(Family::TocLinking),
        MarkdownError::FileLinks(_) => Some(Family::FileLinks),
        _ => None,
    }
}

/// Asserts every reference reader, and composition, refuses `md` with the
/// `expected` family and an error naming `line` (when the parser has one).
fn assert_refused(repo: &TempDir, md: &Markdown, expected: Family, line: Option<usize>, shape: &str) {
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    let results: [(&str, Result<(), MarkdownError>); 6] = [
        ("compose", md.compose_with(&request).map(drop)),
        ("reference graph", md.reference_graph(graph.clone()).map(drop)),
        ("transclusion graph", md.transclusion_graph(graph.clone()).map(drop)),
        ("local query", md.transclusions_with_options(&request).map(drop)),
        ("flattened references", md.composed_references(graph.clone()).map(drop)),
        ("validation", md.validate_references(ReferenceValidationOptions::with_graph(graph)).map(drop)),
    ];
    for (reader, result) in results {
        let error = result.expect_err(&format!("{shape}: {reader} must refuse a malformed inclusion"));
        assert_eq!(family(&error), Some(expected), "{shape}: {reader}: {error:?}");
        if let Some(line) = line {
            assert!(error.to_string().contains(&format!("line {line}")), "{shape}: {reader}: {error}");
        }
    }
}

/// `(syntax, raw target)` of every flattened transclusion record, the node
/// counts of both graph modes, and the local query's kinds, after checking
/// that fresh and prebuilt-graph validation agree and report no issue.
type Shape = (Vec<(ReferenceSyntax, String)>, usize, usize, usize);

fn accepted_shape(repo: &TempDir, md: &Markdown, shape: &str) -> Shape {
    let request = request(repo.path());
    let graph_options = ReferenceGraphOptions::with_compose(&request);
    let graph = md.reference_graph(graph_options.clone()).unwrap_or_else(|e| panic!("{shape}: {e}"));
    let transclusion_nodes =
        md.transclusion_graph(graph_options.clone()).unwrap_or_else(|e| panic!("{shape}: {e}")).node_count();
    let records = md
        .composed_references(graph_options.clone())
        .unwrap_or_else(|e| panic!("{shape}: {e}"))
        .records
        .into_iter()
        .filter(|record| record.attributes.get("toc_synthesized").is_none())
        .filter_map(|record| {
            matches!(
                record.origin.syntax,
                ReferenceSyntax::DirectiveFile
                    | ReferenceSyntax::DirectiveCode
                    | ReferenceSyntax::DirectiveUrl
                    | ReferenceSyntax::DirectiveTocLinking
                    | ReferenceSyntax::DirectiveFileLinks
                    | ReferenceSyntax::FrontmatterPrologue
                    | ReferenceSyntax::FrontmatterEpilogue
            )
            .then(|| (record.origin.syntax, record.target.raw().unwrap_or_default().to_string()))
        })
        .collect();
    let local = md.transclusions_with_options(&request).unwrap_or_else(|e| panic!("{shape}: {e}")).len();
    let fresh = md
        .validate_references(ReferenceValidationOptions::with_graph(graph_options.clone()))
        .unwrap_or_else(|e| panic!("{shape}: {e}"));
    let prebuilt = md
        .validate_references_with_graph(&graph, ReferenceValidationOptions::with_graph(graph_options))
        .unwrap_or_else(|e| panic!("{shape}: {e}"));
    assert!(fresh.issues.is_empty(), "{shape}: {:?}", fresh.issues);
    assert!(prebuilt.issues.is_empty(), "{shape}: {:?}", prebuilt.issues);
    (records, graph.node_count(), transclusion_nodes, local)
}

/// R10-F2.1, .4, .5: body `::file`, `::code`, and `::url` keep their graph
/// shape quoted or unquoted, and without a target every reader refuses them
/// with the parser's line.
#[test]
fn body_directive_without_target_is_refused_by_every_reader() {
    let repo = repository();
    for (directive, target, children) in
        [("file", "./side.md", 2), ("code", "./side.md", 1), ("url", "https://example.com/", 1)]
    {
        let unquoted = accepted_shape(&repo, &document(&repo, &format!("# Doc\n\n::{directive} {target}\n")), directive);
        let quoted = accepted_shape(&repo, &document(&repo, &format!("# Doc\n\n::{directive} \"{target}\"\n")), directive);
        assert_eq!(unquoted, quoted, "::{directive}: quoting must not change the graph");
        assert_eq!((unquoted.0.len(), unquoted.1, unquoted.3), (1, children, 1), "::{directive}: {unquoted:?}");

        let md = document(&repo, &format!("# Doc\n\n::{directive}\n"));
        assert_refused(&repo, &md, Family::Directive, Some(3), &format!("::{directive} without a target"));
    }
}

/// R10-F2.2: an invalid `::toc-linking` level and an invalid `::file-links`
/// depth are refused; their valid spellings keep one record each.
#[test]
fn invalid_toc_level_and_file_links_depth_are_refused() {
    let repo = repository();
    for (valid, quoted, invalid, family, code) in [
        (
            "::toc-linking ./side.md level=h2",
            "::toc-linking \"./side.md\" level=h2",
            "::toc-linking ./side.md level=h9",
            Family::TocLinking,
            "dm.toc_linking.invalid_level",
        ),
        (
            "::file-links --dir . --depth 1",
            "::file-links --dir \".\" --depth 1",
            "::file-links --dir . --depth two",
            Family::FileLinks,
            "dm.file_links.invalid_depth",
        ),
    ] {
        let unquoted = accepted_shape(&repo, &document(&repo, &format!("# Doc\n\n{valid}\n")), valid);
        let quoted_shape = accepted_shape(&repo, &document(&repo, &format!("# Doc\n\n{quoted}\n")), quoted);
        assert_eq!(unquoted, quoted_shape, "{valid}: quoting must not change the graph");
        assert_eq!((unquoted.0.len(), unquoted.3), (1, 1), "{valid}: {unquoted:?}");

        let md = document(&repo, &format!("# Doc\n\n{invalid}\n"));
        assert_refused(&repo, &md, family, Some(3), invalid);
        // Each reports its own registered code, never the catch-all.
        let request = request(repo.path());
        let graph = ReferenceGraphOptions::with_compose(&request);
        for (reader, error) in [
            ("compose", md.compose_with(&request).map(drop).unwrap_err()),
            ("graph", md.reference_graph(graph).map(drop).unwrap_err()),
        ] {
            let codes: Vec<String> = error_diagnostics(&error).into_iter().map(|d| d.code).collect();
            assert_eq!(codes, [code], "{invalid}: {reader}");
        }
    }
}

/// R10-F2.3: every `prologue`/`epilogue` shape. Omitted and `[]` list
/// nothing; a scalar or list path includes `side.md`; a blank key, explicit
/// null, number, object, mixed list, or all-invalid list is refused, as
/// composition refuses it.
#[test]
fn frontmatter_inclusion_shapes() {
    let repo = repository();
    for field in ["prologue", "epilogue"] {
        let empty = accepted_shape(&repo, &document(&repo, "---\ntitle: T\n---\n# Doc\n"), "absent");
        assert_eq!(empty, (Vec::new(), 1, 1, 0), "{field} absent");
        let listed_nothing = accepted_shape(&repo, &document(&repo, &format!("---\n{field}: []\n---\n# Doc\n")), "[]");
        assert_eq!(listed_nothing, empty, "{field}: [] lists nothing");

        let scalar =
            accepted_shape(&repo, &document(&repo, &format!("---\n{field}: ./side.md\n---\n# Doc\n")), "scalar");
        let list =
            accepted_shape(&repo, &document(&repo, &format!("---\n{field}:\n  - ./side.md\n---\n# Doc\n")), "list");
        assert_eq!(scalar, list, "{field}: a one-element list is the scalar");
        assert_eq!((scalar.0.len(), scalar.1, scalar.2, scalar.3), (1, 2, 2, 1), "{field}: {scalar:?}");

        for (shape, value) in [
            ("blank key", ""),
            ("explicit null", " null"),
            ("number", " 3"),
            ("object", " { path: ./side.md }"),
            ("mixed list", " [./side.md, 3]"),
            ("all-invalid list", " [1, 2]"),
        ] {
            // The key's authored line (2), in every reader's error.
            let md = document(&repo, &format!("---\n{field}:{value}\n---\n# Doc\n"));
            assert_refused(&repo, &md, Family::Directive, Some(2), &format!("{field} {shape}"));
        }
    }
}

/// R10-F2.1, .3: a malformed inclusion in a transcluded child fails the
/// parent's graph and validation too.
#[test]
fn malformed_inclusion_in_a_child_is_refused() {
    let repo = repository();
    std::fs::write(repo.path().join("child.md"), "---\nepilogue: 3\n---\n# Child\n").unwrap();
    let md = document(&repo, "# Doc\n\n::file ./child.md\n");
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    for (reader, result) in [
        ("reference graph", md.reference_graph(graph.clone()).map(drop)),
        ("transclusion graph", md.transclusion_graph(graph.clone()).map(drop)),
        ("validation", md.validate_references(ReferenceValidationOptions::with_graph(graph)).map(drop)),
    ] {
        let error = result.expect_err(reader);
        assert_eq!(family(&error), Some(Family::Directive), "{reader}: {error:?}");
    }
}

/// R10-F2.7: the boolean detector counts only inclusions that parse, so a
/// malformed sibling hides a valid one; the error-returning query names it.
#[test]
fn has_transclusions_reports_only_parsed_inclusions() {
    let repo = repository();
    assert!(document(&repo, "# Doc\n\n::file ./side.md\n").has_transclusions());
    let md = document(&repo, "# Doc\n\n::file ./side.md\n::file\n");
    assert!(!md.has_transclusions());
    assert!(md.transclusions_with_options(&request(repo.path())).is_err());
}

/// R10-F2.6: the reference check `md compose` runs ahead of composition
/// leaves a malformed inclusion to composition, which rejects the root's and
/// tolerates a child's, so composition's verdict is unchanged.
#[test]
fn reference_check_ahead_of_composition_defers_inclusion_parse_failures() {
    let repo = repository();
    std::fs::write(repo.path().join("child.md"), "# Child\n\n::file\n").unwrap();
    let md = document(&repo, "# Doc\n\n::file ./child.md\n");
    let check = request(repo.path()).map_options(ComposeOptions::for_reference_check);
    let report = md
        .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&check)))
        .expect("the check defers the child's parse failure");
    assert!(report.issues.is_empty(), "{:?}", report.issues);
    md.compose_with(&request(repo.path())).expect("composition tolerates a child's parse failure");
    assert!(md.reference_graph(ReferenceGraphOptions::with_compose(&request(repo.path()))).is_err());
}

/// The authored line of an invalid `prologue:` reaches the published record
/// of composition and of the reference readers; it is never `0`.
#[test]
fn an_invalid_frontmatter_inclusion_is_located_at_its_key() {
    let repo = repository();
    let md = document(&repo, "---\ntitle: T\nprologue: 3\n---\n# Doc\n");
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    for (reader, error) in [
        ("compose", md.compose_with(&request).map(drop).unwrap_err()),
        ("graph", md.reference_graph(graph).map(drop).unwrap_err()),
        ("local query", md.transclusions_with_options(&request).map(drop).unwrap_err()),
    ] {
        assert!(error.to_string().contains("at line 3"), "{reader}: {error}");
        let records = error_diagnostics(&error);
        assert_eq!(records.len(), 1, "{reader}: {records:?}");
        let source = records[0].source.as_ref().expect("a file");
        assert_eq!((records[0].code.as_str(), source.line), ("dm.directive.parse_failure", Some(3)), "{reader}");
        assert!(source.file.as_ref().is_some_and(|file| file.ends_with("doc.md")), "{reader}: {source:?}");
    }
    // A document built in memory has no authored line: the message names
    // none, and the record has no line.
    let mut unfiled = Markdown::new("# Doc\n");
    unfiled.fm_insert("prologue", 3).unwrap();
    let error = unfiled.transclusions_with_options(&request).map(drop).unwrap_err();
    assert!(!error.to_string().contains("line 0"), "{error}");
}

/// The document every reader loads to inspect `bad.md`: it exists, but its
/// frontmatter does not parse.
fn write_unloadable(repo: &TempDir) {
    std::fs::write(repo.path().join("bad.md"), "---\ntitle: [unclosed\n---\n# Bad\n").unwrap();
}

/// An included file that exists but cannot be loaded fails both graph modes
/// and validation with its own frontmatter error, for a body `::file`, a
/// `prologue`/`epilogue`, and the target a `::toc-linking` follows.
/// Composition tolerates it (a `::toc-linking` reads the raw headings), and
/// the check ahead of composition leaves it to composition.
#[test]
fn an_included_file_that_cannot_be_loaded_is_refused() {
    let repo = repository();
    write_unloadable(&repo);
    for content in [
        "# Doc\n\n::file ./bad.md\n",
        "---\nprologue: ./bad.md\n---\n# Doc\n",
        "---\nepilogue: [./bad.md]\n---\n# Doc\n",
        "# Doc\n\n::toc-linking ./bad.md\n",
    ] {
        let md = document(&repo, content);
        let request = request(repo.path());
        let graph = ReferenceGraphOptions::with_compose(&request);
        for (reader, result) in [
            ("reference graph", md.reference_graph(graph.clone()).map(drop)),
            ("transclusion graph", md.transclusion_graph(graph.clone()).map(drop)),
            ("validation", md.validate_references(ReferenceValidationOptions::with_graph(graph)).map(drop)),
        ] {
            let error = result.expect_err(&format!("{content:?}: {reader}"));
            assert!(
                matches!(error, MarkdownError::FrontmatterParse { ref ctx, .. } if ctx.display.ends_with("bad.md")),
                "{content:?}: {reader}: {error:?}"
            );
        }
        md.compose_with(&request).unwrap_or_else(|e| panic!("{content:?}: composition tolerates it: {e}"));
        let check = request.map_options(ComposeOptions::for_reference_check);
        let report = md
            .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&check)))
            .unwrap_or_else(|e| panic!("{content:?}: the check defers it: {e}"));
        assert!(report.issues.is_empty(), "{content:?}: {:?}", report.issues);
    }
}

/// A link to a file that cannot be loaded is checked like any link, but a
/// fragment naming it cannot be verified, so it is an error issue, in the
/// check ahead of composition too.
#[test]
fn a_fragment_into_a_file_that_cannot_be_loaded_is_reported() {
    let repo = repository();
    write_unloadable(&repo);
    let md = document(&repo, "# Doc\n\n[whole](./bad.md)\n[part](./bad.md#bad)\n[side](./side.md#side)\n");
    let check = request(repo.path()).map_options(ComposeOptions::for_reference_check);
    for request in [request(repo.path()), check] {
        let options = ReferenceValidationOptions {
            validate_fragments: true,
            ..ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request))
        };
        let report = md.validate_references(options).unwrap();
        let issues: Vec<_> =
            report.issues.iter().map(|i| (i.code, i.severity, i.reference_display.as_str())).collect();
        assert_eq!(issues, [(ReferenceIssueCode::UnreadableTarget, ReferenceSeverity::Error, "./bad.md#bad")]);
    }
}

/// A `when=` condition that fails is composition's error, and every graph
/// raises it at the directive's authored line; the check ahead of
/// composition leaves it to composition. A false, unset, or missing value
/// still just leaves the directive out.
#[test]
fn a_failing_when_condition_is_refused_unless_composition_owns_it() {
    let repo = repository();
    let condition = |when: &str| document(&repo, &format!("---\nflag: null\n---\n# Doc\n\n::file ./side.md when=\"{when}\"\n"));
    let md = condition("1 / 0");
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    for (reader, result) in [
        ("compose", md.compose_with(&request).map(drop)),
        ("reference graph", md.reference_graph(graph.clone()).map(drop)),
        ("transclusion graph", md.transclusion_graph(graph.clone()).map(drop)),
        ("validation", md.validate_references(ReferenceValidationOptions::with_graph(graph.clone())).map(drop)),
    ] {
        let error = result.expect_err(reader);
        assert!(
            matches!(&error, MarkdownError::Transclusion(inner) if matches!(inner.as_ref(), TransclusionError::ConditionEval { .. })),
            "{reader}: {error:?}"
        );
        let records = error_diagnostics(&error);
        let source = records[0].source.as_ref().expect("a file");
        assert_eq!((records[0].code.as_str(), source.line), ("dm.expression.evaluation_failure", Some(6)), "{reader}");
    }
    let check = request.clone().map_options(ComposeOptions::for_reference_check);
    let report = md
        .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&check)))
        .expect("the check defers a failing condition");
    assert!(report.issues.is_empty(), "{:?}", report.issues);

    for (when, nodes) in [("flag", 1), ("fm.missing.deep", 1), ("false", 1), ("true", 2)] {
        let graph = condition(when).reference_graph(graph.clone()).unwrap_or_else(|e| panic!("{when}: {e}"));
        assert_eq!(graph.node_count(), nodes, "when={when}");
    }
}

/// A chain past the transclusion depth limit leaves documents unread, so the
/// graph and validation fail as composition does; the check ahead of
/// composition leaves it to composition. A cycle reads every document, so
/// the graph still represents it, and a refused descent never shifts the
/// depth of a later sibling.
#[test]
fn a_chain_past_the_depth_limit_is_refused_but_a_cycle_is_a_graph() {
    let repo = repository();
    std::fs::write(repo.path().join("a.md"), "# A\n\n::file ./b.md\n").unwrap();
    std::fs::write(repo.path().join("b.md"), "# B\n\n::file ./c.md\n").unwrap();
    std::fs::write(repo.path().join("c.md"), "# C\n").unwrap();
    let md = document(&repo, "# Doc\n\n::file ./a.md\n");
    let shallow = request(repo.path()).map_options(|options| options.with_max_transclusion_depth(3));
    let graph = ReferenceGraphOptions::with_compose(&shallow);
    for (reader, result) in [
        ("reference graph", md.reference_graph(graph.clone()).map(drop)),
        ("transclusion graph", md.transclusion_graph(graph.clone()).map(drop)),
        ("validation", md.validate_references(ReferenceValidationOptions::with_graph(graph)).map(drop)),
        ("compose", md.compose_with(&shallow).map(drop)),
    ] {
        let error = result.expect_err(reader);
        assert!(
            matches!(error, MarkdownError::Transclusion(ref inner) if matches!(inner.as_ref(), TransclusionError::MaxDepthExceeded { max_depth: 3 })),
            "{reader}: {error:?}"
        );
    }
    let check = shallow.map_options(ComposeOptions::for_reference_check);
    md.validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&check)))
        .expect("the check ahead of composition defers the depth refusal");

    // Control: one level more admits the whole chain.
    let deep = request(repo.path()).map_options(|options| options.with_max_transclusion_depth(4));
    assert_eq!(md.transclusion_graph(ReferenceGraphOptions::with_compose(&deep)).unwrap().node_count(), 4);

    // A cycle is a finite graph naming each document once.
    std::fs::write(repo.path().join("c.md"), "# C\n\n::file ./a.md\n").unwrap();
    let cyclic = md.transclusion_graph(ReferenceGraphOptions::with_compose(&request(repo.path()))).unwrap();
    assert_eq!(cyclic.node_count(), 4);
}
