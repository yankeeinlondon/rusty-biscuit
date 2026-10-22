//! Raw-source frontmatter repair through the public library entry points,
//! `darkmatter::markdown::cleanup::frontmatter::repair_frontmatter` and the
//! `clean_document` transaction built on it.
//!
//! Every assertion is on observable results: the candidate text, outcome
//! flags, diagnostics and applied repairs in document coordinates, and whether
//! the lazy schema closure ran.

use std::cell::Cell;
use std::convert::Infallible;

use biscuit_file::{YamlDiagnosticCode, YamlRepair};
use darkmatter::markdown::cleanup::diagnostic::{CleanCode, CleanDiagnostic, CleanStage};
use darkmatter::markdown::cleanup::frontmatter::{
    FrontmatterRepair, FrontmatterRepairError, repair_frontmatter,
};
use darkmatter::markdown::cleanup::rules::MarkdownRuleOverrides;
use darkmatter::markdown::cleanup::{CleanOutcome, CleanRequest, CleanResult, clean_document};
use darkmatter::markdown::schemas::CleanSchemaConfig;
use darkmatter::markdown::{Markdown, extract_frontmatter_block};

/// An unquoted scalar opening with the reserved `@` indicator: unparseable,
/// and deterministically repaired by quoting.
const RESERVED: &str = "---\ntitle: @daily-report\n---\n\n# Daily Report\n";

fn config() -> CleanSchemaConfig {
    CleanSchemaConfig::new(crate::request_support::cwd_context())
}

fn repair(source: &str) -> FrontmatterRepair {
    repair_frontmatter(source, None, || Ok::<_, Infallible>(config())).expect("repair succeeds")
}

#[test]
fn unparseable_frontmatter_is_repaired_before_markdown_exists_and_is_a_fixed_point() {
    assert!(
        Markdown::try_from_content(RESERVED.to_string()).is_err(),
        "the fixture must not parse as authored"
    );

    let first = repair(RESERVED);

    assert_eq!(
        first.source,
        "---\ntitle: \"@daily-report\"\n---\n\n# Daily Report\n"
    );
    assert!(first.repaired);
    assert_eq!(first.frontmatter_span, Some(4..25));
    assert_eq!(first.diagnostics.len(), 1);
    let diagnostic = &first.diagnostics[0];
    assert_eq!(diagnostic.stage, CleanStage::Syntax);
    assert_eq!(
        diagnostic.code,
        CleanCode::Yaml(YamlDiagnosticCode::ReservedIndicator)
    );
    assert_eq!(&RESERVED[diagnostic.span.clone()], "@daily-report");
    assert_eq!(diagnostic.repairs[0].span, 11..24);
    assert_eq!(first.applied.len(), 1);
    assert_eq!(first.applied[0].span, 11..24);
    assert_eq!(first.applied[0].replacement, "\"@daily-report\"");
    Markdown::try_from_content(first.source.clone()).expect("the repaired document parses");

    let second = repair(&first.source);
    assert_eq!(second.source, first.source);
    assert!(!second.repaired);
    assert!(second.applied.is_empty());

    let third = repair(&second.source);
    assert_eq!(third.source, first.source);
    assert!(!third.repaired);
}

#[test]
fn discarding_repairs_returns_the_authored_bytes_and_keeps_diagnostics() {
    let original = "\u{feff}---  \r\n# keep this comment\r\ntitle: @daily-report\r\n---\t\r\n\r\nBody\r\n";
    let repaired = repair(original);
    assert!(repaired.repaired, "the fixture must exercise accepted repairs");
    let diagnostics = repaired.diagnostics.clone();

    let discarded = repaired.discard_repairs(original);

    assert_eq!(discarded.source.as_bytes(), original.as_bytes());
    assert!(!discarded.repaired);
    assert!(discarded.applied.is_empty());
    assert_eq!(discarded.diagnostics, diagnostics);
    assert!(!discarded.diagnostics.is_empty());
}

#[test]
fn bom_repair_and_later_diagnostics_use_document_coordinates() {
    let source = format!("\u{feff}{RESERVED}");

    let repaired = repair(&source);

    assert_eq!(
        repaired.source,
        "---\ntitle: \"@daily-report\"\n---\n\n# Daily Report\n"
    );
    assert_eq!(repaired.frontmatter_span, Some(7..28));
    assert_eq!(
        repaired.applied,
        vec![
            YamlRepair {
                span: 0..3,
                replacement: String::new(),
                explanation: "remove the UTF-8 byte-order mark at document start".to_string(),
            },
            YamlRepair {
                span: 14..27,
                ..repaired.applied[1].clone()
            },
        ]
    );
    assert_eq!(repaired.diagnostics.len(), 1);
    assert_eq!(repaired.diagnostics[0].span, 14..27);
    assert_eq!(&source[14..27], "@daily-report");
}

#[test]
fn absent_and_empty_frontmatter_never_invoke_the_schema_closure() {
    for source in ["# Body only\n", "---\n---\n\n# Body\n"] {
        let calls = Cell::new(0_usize);
        let counted = repair_frontmatter(source, None, || {
            calls.set(calls.get() + 1);
            Ok::<_, Infallible>(config())
        })
        .expect("repair succeeds");
        assert_eq!(calls.get(), 0, "schema closure ran for {source:?}");
        assert_eq!(counted.source, source);
        assert!(!counted.repaired);
        assert!(counted.diagnostics.is_empty());

        let failing = repair_frontmatter(source, None, || {
            Err::<CleanSchemaConfig, _>("context must not be built")
        });
        assert!(failing.is_ok(), "a failing closure surfaced for {source:?}");
    }
}

#[test]
fn parseable_frontmatter_invokes_the_schema_closure_exactly_once() {
    let calls = Cell::new(0_usize);

    repair_frontmatter("---\ntitle: Fine\n---\n\n# Body\n", None, || {
        calls.set(calls.get() + 1);
        Ok::<_, Infallible>(config())
    })
    .expect("repair succeeds");

    assert_eq!(calls.get(), 1);
}

#[test]
fn closure_failure_is_returned_as_a_typed_context_error() {
    let result = repair_frontmatter("---\ntitle: Fine\n---\n", None, || {
        Err::<CleanSchemaConfig, _>("no context")
    });

    assert!(matches!(result, Err(FrontmatterRepairError::Context("no context"))));
}

#[test]
fn missing_schema_override_is_a_typed_schema_error_for_stdin_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nope.yaml");
    let missing_text = missing.to_string_lossy().into_owned();

    let result = repair_frontmatter("---\ntitle: Fine\n---\n\n# Body\n", None, || {
        Ok::<_, Infallible>(
            config().with_schema_override(serde_json::Value::String(missing_text.clone())),
        )
    });

    match result {
        Err(FrontmatterRepairError::Schema(error)) => assert!(
            error.to_string().contains("nope.yaml"),
            "the error must name the missing schema: {error}"
        ),
        Err(other) => panic!("expected a schema error, got {other}"),
        Ok(_) => panic!("a missing schema must not resolve to a guessed one"),
    }
}

#[test]
fn unrepairable_yaml_is_returned_unchanged_with_its_diagnostic() {
    let source = "---\ntitle: one\ntitle: two\n---\n\n# Body\n";

    let result = repair_frontmatter(source, None, || {
        Err::<CleanSchemaConfig, _>("the schema tier cannot run on unparseable YAML")
    })
    .expect("unrepairable YAML is not an error");

    assert_eq!(result.source, source);
    assert!(!result.repaired);
    assert!(result.applied.is_empty());
    let duplicate = result
        .diagnostics
        .iter()
        .find(|entry| entry.code == CleanCode::Yaml(YamlDiagnosticCode::DuplicateKey))
        .expect("duplicate-key diagnostic");
    assert_eq!(duplicate.code.as_str(), "yaml.duplicate-key");
    assert_eq!(&source[duplicate.span.clone()], "title");
    assert!(result.settings_yaml(source).is_none());
}

#[test]
fn settings_yaml_reads_the_authored_yaml_when_it_parses() {
    let source = "\u{feff}---\ntitle: Fine\nempty:\n---\n\n# Body\n";
    let repaired = repair(source);

    let settings = repaired.settings_yaml(source).expect("authored YAML parses");

    assert!(!settings.from_candidate);
    assert_eq!(settings.yaml, "title: Fine\nempty:\n");
    assert_eq!(settings.authored_offset, 7);
    assert_eq!(settings.authored_span(0..5), 7..12);
    assert_eq!(&source[7..12], "title");
}

#[test]
fn settings_yaml_falls_back_to_the_candidate_and_projects_to_authored_spans() {
    let source = "---\ntitle: @daily-report\nempty: x\n---\n\n# Body\n";
    let repaired = repair(source);

    let settings = repaired
        .settings_yaml(source)
        .expect("the repaired candidate parses");

    assert!(settings.from_candidate);
    assert_eq!(settings.yaml, "title: \"@daily-report\"\nempty: x\n");
    let quoted = settings.yaml.find("\"@daily-report\"").unwrap();
    let quoted_span = quoted..quoted + "\"@daily-report\"".len();
    assert_eq!(&source[settings.authored_span(quoted_span)], "@daily-report");
    let key = settings.yaml.find("empty").unwrap();
    assert_eq!(&source[settings.authored_span(key..key + 5)], "empty");

    // Discarding repairs keeps the candidate readable for settings.
    let discarded = repaired.clone().discard_repairs(source);
    let after_discard = discarded.settings_yaml(source).expect("candidate retained");
    assert!(after_discard.from_candidate);
    assert_eq!(after_discard.yaml, settings.yaml);
}

// ---------------------------------------------------------------------------
// Indented root mappings: the analyzer's `yaml.root-indentation` repair reaches
// `md clean` and DMLS through the ordinary repair path, with no dispatch of
// its own.
// ---------------------------------------------------------------------------

/// The inline-compose reproduction, widened with a comment, a trailing
/// comment, a blank line inside the block scalar, and nested collections.
const INDENTED_PROMPT: &str = concat!(
    "---\n",
    "    # Inline prompt\n",
    "    prompt: |-\n",
    "        Your task is to summarize.\n",
    "\n",
    "        Keep it short.\n",
    "    model: sonnet   # preferred\n",
    "    options:\n",
    "        retries: 2\n",
    "        tags:\n",
    "            - fast\n",
    "            - cheap\n",
    "---\n",
    "\n",
    "# Body\n",
    "\n",
    "Text.\n",
);

const DEDENTED_PROMPT: &str = concat!(
    "---\n",
    "# Inline prompt\n",
    "prompt: |-\n",
    "    Your task is to summarize.\n",
    "\n",
    "    Keep it short.\n",
    "model: sonnet   # preferred\n",
    "options:\n",
    "    retries: 2\n",
    "    tags:\n",
    "        - fast\n",
    "        - cheap\n",
    "---\n",
    "\n",
    "# Body\n",
    "\n",
    "Text.\n",
);

const ROOT_INDENTATION: CleanCode = CleanCode::Yaml(YamlDiagnosticCode::RootIndentation);

fn frontmatter_value(document: &str) -> serde_yaml_ng::Value {
    let extraction = extract_frontmatter_block(document)
        .expect("frontmatter fences are well formed")
        .expect("the document has frontmatter");
    serde_yaml_ng::from_str(extraction.yaml).expect("frontmatter parses")
}

fn codes(diagnostics: &[CleanDiagnostic]) -> Vec<CleanCode> {
    diagnostics.iter().map(|diagnostic| diagnostic.code).collect()
}

fn clean(source: &str) -> CleanResult {
    let overrides = MarkdownRuleOverrides::default();
    let request = CleanRequest { source, overrides: &overrides, workspace: None, document_path: None };
    clean_document(&request, || Ok::<_, Infallible>(config())).expect("clean runs")
}

/// Every top-level key of `document`'s frontmatter starts at column 0.
#[track_caller]
fn assert_root_keys_at_column_zero(document: &str) {
    let extraction = extract_frontmatter_block(document)
        .expect("frontmatter fences are well formed")
        .expect("the document has frontmatter");
    let first = extraction
        .yaml
        .lines()
        .find(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .expect("a key line");
    assert!(!first.starts_with([' ', '\t']), "first key is indented: {first:?}");
}

/// A second cleanup of `cleaned` reports no root-indentation finding and
/// applies nothing.
#[track_caller]
fn assert_second_cleanup_is_stable(cleaned: &str) {
    let again = repair(cleaned);
    assert!(
        !codes(&again.diagnostics).contains(&ROOT_INDENTATION),
        "second repair re-reported the root: {:?}",
        again.diagnostics
    );
    assert!(again.applied.is_empty(), "second repair applied {:?}", again.applied);
    assert!(!again.repaired);
    assert_eq!(again.source, cleaned);

    let document = clean(cleaned);
    assert_eq!(document.outcome, CleanOutcome::Unchanged);
    assert!(!codes(&document.diagnostics).contains(&ROOT_INDENTATION));
    assert!(document.applied.is_empty());
}

#[test]
fn indented_root_reproduction_is_dedented_by_one_root_indentation_finding() {
    let repaired = repair(INDENTED_PROMPT);

    assert_eq!(repaired.source, DEDENTED_PROMPT);
    assert!(repaired.repaired);
    let roots: Vec<_> = repaired
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == ROOT_INDENTATION)
        .collect();
    assert_eq!(roots.len(), 1, "one finding for the whole root: {:?}", repaired.diagnostics);
    let root = roots[0];
    assert_eq!(root.stage, CleanStage::Syntax);
    assert_eq!(root.message, "root mapping is indented by 4 spaces");
    assert!(root.repairs.len() > 1, "one finding carries every prefix edit");
    for edit in &root.repairs {
        assert!(edit.replacement.is_empty(), "prefix edits only remove: {edit:?}");
        assert!(
            INDENTED_PROMPT[edit.span.clone()].bytes().all(|byte| byte == b' '),
            "a prefix edit removes spaces only, in document coordinates: {:?}",
            &INDENTED_PROMPT[edit.span.clone()]
        );
        assert!(repaired.applied.contains(edit), "the finding's edit was applied: {edit:?}");
    }
    assert_eq!(frontmatter_value(&repaired.source), frontmatter_value(INDENTED_PROMPT));
    Markdown::try_from_content(repaired.source.clone()).expect("the cleaned document parses");
}

#[test]
fn clean_document_applies_the_root_repair_and_keeps_the_parsed_value() {
    let result = clean(INDENTED_PROMPT);

    assert_eq!(result.outcome, CleanOutcome::Changed);
    assert_eq!(result.text, DEDENTED_PROMPT);
    assert_root_keys_at_column_zero(&result.text);
    assert_eq!(frontmatter_value(&result.text), frontmatter_value(INDENTED_PROMPT));
    let root = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == ROOT_INDENTATION)
        .expect("root-indentation finding");
    for edit in &root.repairs {
        assert!(result.applied.contains(edit), "clean emitted the edit: {edit:?}");
    }
    assert_second_cleanup_is_stable(&result.text);
}

#[test]
fn indented_root_combines_with_trailing_whitespace_and_final_blank_lines() {
    let source = "---\n  title: Notes   \n  \n  meta:\n    owner: ken\t\n\n\n---\nBody\n";

    let result = clean(source);

    assert_eq!(result.outcome, CleanOutcome::Changed);
    assert_eq!(result.text, "---\ntitle: Notes\n\nmeta:\n  owner: ken\n---\nBody\n");
    let found = codes(&result.diagnostics);
    assert!(found.contains(&ROOT_INDENTATION), "{found:?}");
    assert!(found.contains(&CleanCode::Yaml(YamlDiagnosticCode::TrailingWhitespace)), "{found:?}");
    assert_eq!(frontmatter_value(&result.text), frontmatter_value(source));
    assert_second_cleanup_is_stable(&result.text);
}

/// The root repair itself only removes leading spaces, so it never touches a
/// terminator. The frontmatter's LF result comes from the separate
/// `yaml.line-ending` repair; the delimiter lines and the body keep CRLF
/// because cleanup preserves them byte-for-byte.
#[test]
fn crlf_indented_root_keeps_prefix_edits_off_terminators() {
    let source = "---\r\n    a: 1   \r\n    b: |\r\n      text\r\n---\r\n\r\nBody\r\nmore\r\n";

    let result = clean(source);

    assert_eq!(result.outcome, CleanOutcome::Changed);
    assert_eq!(result.text, "---\r\na: 1\nb: |\n  text\n---\r\n\r\nBody\r\nmore\r\n");
    let root = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == ROOT_INDENTATION)
        .expect("root-indentation finding");
    for edit in &root.repairs {
        let removed = &source[edit.span.clone()];
        assert!(edit.replacement.is_empty() && removed.bytes().all(|byte| byte == b' '), "{removed:?}");
    }
    assert!(
        codes(&result.diagnostics).contains(&CleanCode::Yaml(YamlDiagnosticCode::LineEnding)),
        "the LF normalization is its own finding: {:?}",
        result.diagnostics
    );
    assert_eq!(frontmatter_value(&result.text), frontmatter_value(source));
    assert_second_cleanup_is_stable(&result.text);
}

#[test]
fn tab_indented_root_is_recovered_then_dedented_by_the_rescan() {
    let source = "---\n\tprompt: |-\n\t\tbody line\n\tnext: 1\n---\n\n# Body\n";

    let repaired = repair(source);

    assert_eq!(repaired.source, "---\nprompt: |-\n  body line\nnext: 1\n---\n\n# Body\n");
    let found = codes(&repaired.diagnostics);
    let tab = found
        .iter()
        .position(|code| *code == CleanCode::Yaml(YamlDiagnosticCode::TabIndentation))
        .expect("tab recovery ran first");
    let root = found
        .iter()
        .position(|code| *code == ROOT_INDENTATION)
        .expect("the rescan offered the root repair");
    assert!(tab < root, "the root repair is a rescan finding: {found:?}");
    assert_eq!(found.iter().filter(|code| **code == ROOT_INDENTATION).count(), 1);
    // Tab recovery reads each indentation tab as two spaces; the dedented
    // result keeps the values Darkmatter loads from the authored block.
    assert_eq!(
        frontmatter_value(&repaired.source),
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>("prompt: body line\nnext: 1\n").unwrap()
    );

    let result = clean(source);
    assert_eq!(result.outcome, CleanOutcome::Changed);
    assert_eq!(result.text, repaired.source);
    assert_second_cleanup_is_stable(&result.text);
}
