use std::path::{Path, PathBuf};

use super::*;

/// An absolute repository root on the host's path syntax.
fn repo() -> PathBuf {
    if cfg!(windows) { PathBuf::from(r"C:\repo") } else { PathBuf::from("/repo") }
}

fn uri(path: &Path) -> String {
    file_path_to_uri(path).expect("absolute path").as_str().replace('(', "%28").replace(')', "%29")
}

fn lines(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("p{index}: string")).collect()
}

#[test]
fn escape_and_code_spans_keep_text_literal() {
    assert_eq!(markdown_escape("a*b_c [d](e) <f> & ~g | !h \\ `i`"), "a\\*b\\_c \\[d\\]\\(e\\) \\<f\\> \\& \\~g \\| \\!h \\\\ \\`i\\`");
    assert_eq!(markdown_escape("two\r\nlines"), "two lines");
    assert_eq!(code_span("feature-review.yaml"), "`feature-review.yaml`");
    assert_eq!(code_span("a`b"), "`` a`b ``");
    assert_eq!(code_span("``x``"), "``` ``x`` ```");
    assert_eq!(code_span("café"), "`café`");
}

#[test]
fn a_fence_is_longer_than_any_backtick_run_in_the_excerpt() {
    assert_eq!(fence_for(&lines(2)), "```");
    assert_eq!(fence_for(&["a: string(description(```))".to_string()]), "````");
    assert_eq!(fence_for(&["`````".to_string()]), "``````");
}

#[test]
fn display_paths_are_repository_relative_inside_and_absolute_outside() {
    let root = repo();
    assert_eq!(display_path(&root.join("schemas").join("a.yaml"), Some(&root)), "schemas/a.yaml");
    let outside = root.parent().unwrap().join("elsewhere").join("b.yaml");
    let expected = outside.to_string_lossy().replace('\\', "/");
    assert_eq!(display_path(&outside, Some(&root)), expected);
    assert_eq!(display_path(&outside, None), expected);
}

#[test]
fn links_percent_encode_parentheses_and_keep_non_ascii() {
    let path = repo().join("[x] (y) café").join("s.yaml");
    let link = markdown_link("label", &path);
    assert!(link.starts_with("[label](file://"), "{link}");
    assert!(link.contains("%5Bx%5D%20%28y%29%20caf"), "{link}");
    assert!(!link["[label](".len()..link.len() - 1].contains(['(', ')']), "{link}");
    assert_eq!(link, format!("[label]({})", uri(&path)));
}

#[test]
fn a_valid_target_shows_description_definition_and_link() {
    let path = repo().join("schemas").join("feature-review.yaml");
    let preview = lines(10);
    let markdown = valid_schema_target(
        &ValidTarget {
            path: &path,
            format: SchemaTargetFormat::Yaml,
            description: Some("Reviews of a *feature*."),
            preview: &preview,
            total_lines: 14,
        },
        Some(&repo()),
    );
    let body = preview.join("\n");
    assert_eq!(
        markdown,
        format!(
            "Reviews of a *feature*.\n\nSchema Definition:\n\n```yaml\n{body}\n```\n\n*Showing **10** of **14** lines*\n\nResolves to [schemas/feature-review.yaml]({})",
            uri(&path)
        )
    );
}

#[test]
fn ten_or_fewer_lines_have_no_truncation_line_and_json_uses_a_json_fence() {
    let path = repo().join("schemas").join("s.json");
    for count in [1, 10] {
        let preview = lines(count);
        let markdown = valid_schema_target(
            &ValidTarget { path: &path, format: SchemaTargetFormat::Json, description: Some("  "), preview: &preview, total_lines: count },
            Some(&repo()),
        );
        assert!(markdown.starts_with("Schema Definition:\n\n```json\n"), "{markdown}");
        assert!(!markdown.contains("Showing"), "{markdown}");
    }
    let preview = vec!["a: string(description(```x```))".to_string()];
    let markdown = valid_schema_target(
        &ValidTarget { path: &path, format: SchemaTargetFormat::Yaml, description: None, preview: &preview, total_lines: 1 },
        Some(&repo()),
    );
    assert!(markdown.contains("````yaml\na: string(description(```x```))\n````"), "{markdown}");
}

#[test]
fn a_label_with_markdown_punctuation_is_escaped() {
    let path = repo().join("[a]_*b*").join("s.yaml");
    let markdown = valid_schema_target(
        &ValidTarget { path: &path, format: SchemaTargetFormat::Yaml, description: None, preview: &[], total_lines: 0 },
        Some(&repo()),
    );
    assert!(markdown.ends_with(&format!("Resolves to [\\[a\\]\\_\\*b\\*/s.yaml]({})", uri(&path))), "{markdown}");
}

#[test]
fn rejected_targets_link_the_file_with_their_own_reason() {
    let root = repo();
    let path = root.join("schemas").join("lib.yaml");
    let link = format!("[`schemas/lib.yaml`]({})", uri(&path));
    assert_eq!(
        not_a_schema_target(&path, Some(&root)),
        format!("The file {link} exists but it is **not** a valid schema definition!")
    );
    assert_eq!(
        unsupported_dialect_target(&path, Some(&root), "http://json-schema.org/draft-07/schema#"),
        format!(
            "The file {link} declares the unsupported JSON Schema dialect `http://json-schema.org/draft-07/schema#`. Only `https://json-schema.org/draft/2020-12/schema` is supported."
        )
    );
    assert_eq!(
        unusable_schema_target(&path, Some(&root), "bad `x`\nat line 2"),
        format!("The file {link} could not be used as a schema: bad \\`x\\` at line 2")
    );
}

#[test]
fn a_clean_miss_lists_the_search_and_the_creation_path() {
    let root = repo();
    let first = root.join("pkg").join("schemas").join("x.yaml");
    let second = root.join("schemas").join("x.yaml");
    let attempted = vec![first.clone(), second];
    let markdown = missing_schema_reference(
        &MissingReference {
            reference: "x.yaml",
            sibling_suggestion: None,
            without_roots: false,
            attempted: &attempted,
            creation: Some(&first),
        },
        Some(&root),
    );
    assert_eq!(
        markdown,
        "`x.yaml` does not match a schema file.\n\nSearched, in order:\n\n- `pkg/schemas/x.yaml`\n- `schemas/x.yaml`\n\nMissing: `pkg/schemas/x.yaml` does not exist yet; following the link opens this path."
    );
    let without_creation = missing_schema_reference(
        &MissingReference { reference: "x.yaml", sibling_suggestion: None, without_roots: false, attempted: &attempted, creation: None },
        Some(&root),
    );
    assert!(!without_creation.contains("Missing:"), "{without_creation}");
}

#[test]
fn a_miss_without_roots_and_a_sibling_only_match_have_their_own_lead() {
    let root = repo();
    let beside = root.join("docs").join("x.yaml");
    let attempted = vec![beside.clone()];
    assert_eq!(
        missing_schema_reference(
            &MissingReference { reference: "x.yaml", sibling_suggestion: None, without_roots: true, attempted: &attempted, creation: Some(&beside) },
            Some(&root),
        ),
        "`x.yaml` was looked up like any other file reference (no schema root folder exists).\n\nSearched, in order:\n\n- `docs/x.yaml`\n\nMissing: `docs/x.yaml` does not exist yet; following the link opens this path."
    );
    let creation = root.join("schemas").join("x.yaml");
    let attempted = vec![creation.clone()];
    let sibling = missing_schema_reference(
        &MissingReference { reference: "x.yaml", sibling_suggestion: Some("./x.yaml"), without_roots: false, attempted: &attempted, creation: Some(&creation) },
        Some(&root),
    );
    assert_eq!(
        sibling,
        "`x.yaml` is not in any schema root, but a file with that name is beside this document. Write `./x.yaml` to reference it.\n\nSearched, in order:\n\n- `schemas/x.yaml`\n\nMissing: `schemas/x.yaml` does not exist yet; following the link opens this path."
    );
    assert!(!sibling.contains("docs/x.yaml") && !sibling.contains("file://"), "{sibling}");
}

#[test]
fn failures_that_resolve_nothing_offer_no_link() {
    assert_eq!(
        unresolvable_schema_reference("~/x.yaml", "needs a home [dir]"),
        "`~/x.yaml` could not be resolved: needs a home \\[dir\\]"
    );
    assert_eq!(
        remote_schema_reference("https://example.com/s.yaml"),
        "`https://example.com/s.yaml` is a remote reference. Remote schemas are not fetched."
    );
    assert_eq!(
        unreadable_schema_roots("x.yaml", "not UTF-8"),
        "`x.yaml` could not be resolved because the schema roots could not be read: not UTF-8"
    );
    assert_eq!(
        no_resolution_context("x.yaml"),
        "This document has no file-resolution context, so `x.yaml` is not resolved."
    );
}

#[test]
fn file_value_hovers_use_the_specified_wording() {
    let root = repo();
    let path = root.join("notes").join("café.md");
    assert_eq!(found_file(&path, Some(&root)), format!("Resolves to [`notes/café.md`]({})", uri(&path)));
    assert_eq!(invalid_file_reference("a::b", "bad *x*"), "`a::b` is not a valid file reference: bad \\*x\\*");
    assert_eq!(
        missing_file("source", Eagerness::Eager),
        "This file reference is a valid file reference but the file does not exist yet in the filesystem. The `source` is defined as being an **eager** file reference so that means the file reference must refer to a real file at runtime."
    );
    assert_eq!(
        missing_file("docs.main", Eagerness::Lazy),
        "This file reference is a valid file reference but the file does not exist yet in the filesystem. Because `docs.main` is not defined as being **eager** then the file reference is not required to exist even at run time."
    );
    assert_eq!(
        missing_file("source", Eagerness::EagerFileArm),
        "This file reference is a valid file reference but the file does not exist yet in the filesystem. The `source` has an **eager** `file` arm, so when this value is read as that file reference it must refer to a real file at runtime."
    );
    for failure in [ResolutionFailure::MissingContext, ResolutionFailure::Io, ResolutionFailure::UnsupportedRemote] {
        let text = unchecked_file(failure).expect("its own text");
        assert!(!text.contains("does not exist") && !text.contains("file://"), "{text}");
    }
    assert_eq!(unchecked_file(ResolutionFailure::NoMatch), None);
    assert_eq!(unchecked_file(ResolutionFailure::InvalidReference), None);
}
