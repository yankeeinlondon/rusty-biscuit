//! Validates content-policy's shipped editor schema through Darkmatter's
//! library validator, the same path `md schema validate` and DMLS take.
//!
//! The schema is a `kind: schema` file that declares `short_form`,
//! `long_form`, and `policy`. Today `policy[]` is rejected because `[]` cannot
//! be applied to a union-typed named type, so each test document types every
//! policy entry as its own `policy@./content-policy.yaml` property instead of
//! one `content_policy` list.

use darkmatter::markdown::{
    Markdown,
    compose::ComposeSource,
    schemas::{DarkmatterSchemas, ValidationReport},
};

/// Validates a document whose property `p{index}` holds `entries[index]`, a
/// YAML flow value typed as one `policy` entry.
///
/// Each test passes `schema` as its own `include_str!`: a read inside a test
/// function lets CI's test-input index schedule exactly these tests when the
/// schema changes, where a shared constant would schedule the whole binary.
fn validate_entries(schema: &str, entries: &[&str]) -> ValidationReport {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("content-policy.yaml"), schema).expect("write schema");

    let mut frontmatter = String::from("$schema:\n");
    for index in 0..entries.len() {
        frontmatter.push_str(&format!("  p{index}: \"policy@./content-policy.yaml\"\n"));
    }
    for (index, entry) in entries.iter().enumerate() {
        frontmatter.push_str(&format!("p{index}: {entry}\n"));
    }
    let doc_path = dir.path().join("doc.md");
    let source = format!("---\n{frontmatter}---\nbody\n");
    std::fs::write(&doc_path, &source).expect("write doc");

    let markdown: Markdown = source.as_str().into();
    DarkmatterSchemas::new()
        .validate(&markdown.with_source(ComposeSource::File(doc_path)))
        .expect("the content-policy schema loads and validates")
}

fn problem_paths(report: &ValidationReport) -> Vec<&str> {
    report
        .problems
        .iter()
        .map(|problem| problem.path.as_str())
        .collect()
}

#[test]
fn mixed_compact_and_long_form_entries_validate() {
    let report = validate_entries(
        include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        &[
            "Evergreen",
            "TimeSensitive",
            "ValidFor(3mo)",
            "ValidFor(10d, @last_updated)",
            "ValidFor(2wk,2026-09-28)",
            "'ValidFor( 1yr , @reviewed-on )'",
            "ValidUntil(2027-01-01)",
            "ValidUntil(@retire_on)",
            "{rule: 'ValidFor(6mo)', action: archive}",
            "{rule: TimeSensitive, action: remove}",
            "{rule: Evergreen, action: refresh}",
            "FileChanged(src/config.rs, @config_fingerprint)",
            "\"FileChanged(&Cargo.toml, @fp)\"",
            "\"FileChanged(../x.md,@fp)\"",
            "\"FileChanged(notes/a #1.md, @notes_fingerprint)\"",
            "{rule: \"FileChanged(^README.md, @fp)\", action: archive}",
        ],
    );

    assert!(report.valid, "problems: {:?}", report.problems);
}

#[test]
fn duration_rule_name_is_flagged() {
    let report = validate_entries(
        include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        &["Evergreen", "Duration(3mo)"],
    );

    assert_eq!(problem_paths(&report), ["/p1"], "{:?}", report.problems);
}

#[test]
fn unknown_action_is_flagged() {
    let report = validate_entries(
        include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        &["{rule: Evergreen, action: delete}"],
    );

    assert_eq!(
        problem_paths(&report),
        ["/p0/action"],
        "{:?}",
        report.problems
    );
}

#[test]
fn long_form_without_action_is_flagged() {
    let report = validate_entries(
        include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        &["{rule: 'ValidFor(3mo)'}"],
    );

    assert_eq!(problem_paths(&report), ["/p0"], "{:?}", report.problems);
}

#[test]
fn compact_rules_outside_the_grammar_are_flagged() {
    let report = validate_entries(
        include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        &[
            "ValidFor(3w)",
            "ValidFor(0mo)",
            "ValidFor(3MO)",
            "'ValidFor(3mo, @a.b)'",
            "' ValidFor(3mo)'",
            "evergreen",
        ],
    );

    assert_eq!(
        problem_paths(&report),
        ["/p0", "/p1", "/p2", "/p3", "/p4", "/p5"],
        "{:?}",
        report.problems
    );
}

#[test]
fn file_changed_rules_outside_the_grammar_are_flagged() {
    let report = validate_entries(
        include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        &[
            "FileChanged(src/config.rs)",
            "\"FileChanged(src,config.rs, @fp)\"",
            r"'FileChanged(src\config.rs, @fp)'",
            "\"FileChanged( src/a.rs, @fp)\"",
            "\"FileChanged(src/a.rs , @fp)\"",
            "\"FileChanged(/etc/hosts, @fp)\"",
            "\"FileChanged(~/x, @fp)\"",
            "\"FileChanged(@x, @fp)\"",
            "\"FileChanged(%x, @fp)\"",
            "\"FileChanged({{HOME}}/x, @fp)\"",
            "\"FileChanged(src/config.rs, @a.b)\"",
        ],
    );

    // Problems arrive sorted by path text, which puts `/p10` before `/p2`.
    let mut flagged = problem_paths(&report);
    flagged.sort_by_key(|path| path.trim_start_matches("/p").parse::<usize>().ok());
    assert_eq!(
        flagged,
        ["/p0", "/p1", "/p2", "/p3", "/p4", "/p5", "/p6", "/p7", "/p8", "/p9", "/p10"],
        "{:?}",
        report.problems
    );
}

#[test]
fn long_form_without_rule_is_not_flagged_by_the_schema() {
    // `rule` cannot carry `(required)` today: a constraint on the union-typed
    // `short_form` reference is rejected when `long_form` is referenced
    // directly and is not enforced through `policy`. Evaluation by the
    // content-policy library reports the missing `rule` instead.
    let report = validate_entries(
        include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        &["{action: refresh}"],
    );

    assert!(report.valid, "problems: {:?}", report.problems);
}
