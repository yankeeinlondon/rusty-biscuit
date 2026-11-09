//! A slot failure is reported at the slot as authored, in the file it was
//! written in: the document for an inline `$schema` (whatever YAML carrier
//! holds the type string), the trigger file for a trigger's inline payload,
//! and the schema or imported file otherwise. A position is given only when
//! it is proven; the record's `span` is the slot's bytes in that file.
//!
//! Each case is a Git repository in a temp dir with an empty home, so no
//! persisted grant applies; data files left untracked get no automatic
//! grant. Composition runs as `md compose` does, and validation prepares the
//! schemas `md schema validate` prepares (baseline, discovered triggers, a
//! slot session), so both reports come from the library's own projections
//! (`error_diagnostics`, `template_diagnostics`).

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::diagnostics::Diagnostic;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, template_diagnostics};
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, ComposeSource, RequestSnapshot};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::SlotSession;

use crate::dynamic_slots_fixtures::git;

const WRONG_SHAPE: &str = "dm.schema.slot_wrong_shape";
const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const DOC: &str = "docs/doc.md";
const TRIGGER: &str = "schemas/a.trigger.yaml";

/// A repository holding `files` (committed) and `untracked`, and a home
/// with no grant store.
struct Case {
    _temp: tempfile::TempDir,
    root: PathBuf,
    home: tempfile::TempDir,
}

impl Case {
    fn new(files: &[(&str, &str)], untracked: &[(&str, &str)]) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir").join("repo");
        let case = Self { _temp: temp, root, home: tempfile::tempdir().expect("temp home") };
        for (name, text) in files {
            case.write(name, text);
        }
        git(&case.root, &["init", "-q", "-b", "main"]);
        git(&case.root, &["add", "-A"]);
        git(&case.root, &["commit", "-q", "-m", "fixture"]);
        for (name, text) in untracked {
            case.write(name, text);
        }
        case
    }

    /// A document `doc.md` whose frontmatter is `frontmatter`, alone.
    fn document(frontmatter: &str) -> Self {
        Self::new(&[(DOC, &format!("---\n{frontmatter}---\nBody.\n"))], &[])
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn text(&self, name: &str) -> String {
        std::fs::read_to_string(self.path(name)).expect("fixture text")
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, text).expect("write");
    }

    fn request(&self, document: &Path) -> ComposeRequest {
        let snapshot =
            RequestSnapshot::new(&self.root).with_home(Some(self.home.path().to_path_buf())).with_env(HashMap::new());
        let options = ComposeOptions::new().with_source_file(document).with_trigger_schemas(true);
        ComposeRequest::prepare(options, &snapshot).expect("request")
    }

    /// The records of composing `doc`, which must fail.
    fn compose_records(&self, doc: &str) -> Vec<Diagnostic> {
        let document = self.path(doc);
        let failure = Markdown::try_from(document.as_path())
            .expect("document")
            .compose_with_warnings(&self.request(&document))
            .expect_err("the slot fails the composition");
        error_diagnostics(&failure.error)
    }

    /// The records of validating `doc` as `md schema validate` does (in the
    /// document's own context), whose schema must fail to resolve.
    fn validate_records(&self, doc: &str) -> Vec<Diagnostic> {
        let document = self.path(doc);
        let request = self.request(&document);
        let context = request.resolution_context().for_cwd(document.parent().expect("document folder"));
        let schemas = DarkmatterSchemas::new(context)
            .with_darkmatter_baseline_json_schema()
            .expect("baseline")
            .with_trigger_discovery()
            .expect("triggers")
            .with_slot_session(Arc::new(SlotSession::for_request(&request)));
        assert!(schemas.discovery_findings().is_empty(), "{:?}", schemas.discovery_findings());
        let markdown =
            Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document.clone()));
        let Err(error) = schemas.validate(&markdown) else { panic!("the schema does not resolve") };
        template_diagnostics(&error, &document).unwrap_or_else(|| panic!("not a template failure: {error:?}"))
    }
}

/// The byte range of the `nth` (zero-based) `{{ … }}` in `text`.
fn nth_slot(text: &str, nth: usize) -> Range<usize> {
    let start = text.match_indices("{{").nth(nth).expect("a slot").0;
    let end = start + text[start..].find("}}").expect("a closed slot") + 2;
    start..end
}

/// Asserts `records` is one record with `code`, at `file`, `line`, and
/// `column`, spanning `span`.
fn assert_at(records: &[Diagnostic], code: &str, file: &Path, line: usize, column: usize, span: Range<usize>) {
    assert_eq!(records.len(), 1, "{records:#?}");
    let record = &records[0];
    assert_eq!(record.code, code, "{record:#?}");
    let source = record.source.as_ref().expect("a source");
    assert_eq!(source.file.as_deref(), Some(file), "{record:#?}");
    assert_eq!((source.line, source.column), (Some(line), Some(column)), "{record:#?}");
    assert_eq!(record.span, Some(span), "{record:#?}");
}

/// The one-based line and column (in characters) of byte `offset` in `text`.
fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    (before.matches('\n').count() + 1, before[line_start..].chars().count() + 1)
}

const INLINE: &str = "title: Inline\n$schema:\n  os: enum({{ 123 }})\nos: Linux\n";

/// R1-F3.1, R1-F3.2: an inline document `$schema`'s slot is in the document.
mod inline_document {
    use super::*;

    #[test]
    fn compose_reports_a_wrong_result_at_the_slot_in_the_document() {
        let case = Case::document(INLINE);
        let span = nth_slot(&case.text(DOC), 0);
        assert_at(&case.compose_records(DOC), WRONG_SHAPE, &case.path(DOC), 4, 12, span);
    }

    #[test]
    fn schema_validate_reports_a_wrong_result_at_the_slot_in_the_document() {
        let case = Case::document(INLINE);
        let span = nth_slot(&case.text(DOC), 0);
        assert_at(&case.validate_records(DOC), WRONG_SHAPE, &case.path(DOC), 4, 12, span);
    }

    /// R1-F3.12: an unapproved read is at the slot too.
    #[test]
    fn an_unapproved_read_is_at_the_slot_in_the_document() {
        let case = Case::new(
            &[(DOC, "---\n$schema:\n  os: enum({{ load_data(\"../data/os.yaml\", \"string[]\") }})\nos: Linux\n---\n")],
            &[("data/os.yaml", "- Linux\n")],
        );
        let span = nth_slot(&case.text(DOC), 0);
        assert_at(&case.compose_records(DOC), APPROVAL_REQUIRED, &case.path(DOC), 3, 12, span.clone());
        assert_at(&case.validate_records(DOC), APPROVAL_REQUIRED, &case.path(DOC), 3, 12, span);
    }

    /// R1-F3.13: a `load_data` failure keeps its `dm.data.*` code and its
    /// `data_location`, and its primary position is the slot.
    #[test]
    fn a_data_failure_keeps_its_code_and_data_location_at_the_slot() {
        let case = Case::new(
            &[
                (DOC, "---\n$schema:\n  os: enum({{ load_data(\"../data/os.yaml\", \"string[]\") }})\nos: Linux\n---\n"),
                ("data/os.yaml", "- Linux\n- [nested]\n"),
            ],
            &[],
        );
        let span = nth_slot(&case.text(DOC), 0);
        for records in [case.compose_records(DOC), case.validate_records(DOC)] {
            let code = records.first().map(|record| record.code.clone()).unwrap_or_default();
            assert!(code.starts_with("dm.data."), "{records:#?}");
            assert_at(&records, &code, &case.path(DOC), 3, 12, span.clone());
            assert!(records[0].data_location.is_some(), "{records:#?}");
        }
    }

    /// R1-F3.14: a rejected slot pattern is at the slot.
    #[test]
    fn a_rejected_pattern_is_at_the_slot_in_the_document() {
        let case = Case::document("$schema:\n  id: string(pattern({{ '(?=a)' }}))\nid: a\n");
        let span = nth_slot(&case.text(DOC), 0);
        assert_at(&case.validate_records(DOC), "dm.schema.slot_pattern_rejected", &case.path(DOC), 3, 22, span);
    }

    /// R1-F3.15: a dynamic default the type rejects is at the slot.
    #[test]
    fn an_inadmissible_dynamic_default_is_at_the_slot_in_the_document() {
        let case = Case::document("$schema:\n  count: number(default({{ 'many' }}))\n");
        let span = nth_slot(&case.text(DOC), 0);
        let records = case.validate_records(DOC);
        let code = records.first().map(|record| record.code.clone()).unwrap_or_default();
        assert!(code.starts_with("dm.schema.slot_"), "{records:#?}");
        assert_at(&records, &code, &case.path(DOC), 3, 25, span);
    }

    /// R1-F3.16: an inline root-union arm's slot is in the document.
    #[test]
    fn a_root_union_arm_slot_is_at_the_slot_in_the_document() {
        let case = Case::document("$schema:\n  - name: string\n  - os: enum({{ 123 }})\nos: Linux\n");
        let span = nth_slot(&case.text(DOC), 0);
        assert_at(&case.validate_records(DOC), WRONG_SHAPE, &case.path(DOC), 4, 14, span.clone());
        assert_at(&case.compose_records(DOC), WRONG_SHAPE, &case.path(DOC), 4, 14, span);
    }

    /// R1-F3.17: a second slot with the same expression in another property
    /// is located at its own bytes, never the first one's.
    #[test]
    fn two_identical_slots_are_each_at_their_own_bytes() {
        let case = Case::document("$schema:\n  a: string\n  os: enum(macOS, {{ 123 }})\n  b: enum({{ 123 }})\nos: macOS\n");
        let text = case.text(DOC);
        let records = case.validate_records(DOC);
        let mut spans: Vec<_> = records.iter().map(|record| record.span.clone()).collect();
        spans.sort_by_key(|span| span.as_ref().map(|span| span.start));
        // The library reports the first unusable value it meets; whichever
        // it is, its span is one of the two authored slots exactly.
        assert!(!records.is_empty(), "{records:#?}");
        for span in spans {
            assert!(span == Some(nth_slot(&text, 0)) || span == Some(nth_slot(&text, 1)), "{records:#?}");
        }
    }
}

/// R1-F3.10, R1-F3.11: the type string's YAML carrier changes where the
/// slot's bytes are, never whether they are found.
mod carriers {
    use super::*;

    /// The validation and composition records of a document whose `os`
    /// definition is written as `definition`, each at the slot.
    fn assert_carrier(definition: &str) {
        let case = Case::document(&format!("$schema:\n  os: {definition}\nos: Linux\n"));
        let text = case.text(DOC);
        let span = nth_slot(&text, 0);
        let (line, column) = line_column(&text, span.start);
        assert_at(&case.validate_records(DOC), WRONG_SHAPE, &case.path(DOC), line, column, span.clone());
        assert_at(&case.compose_records(DOC), WRONG_SHAPE, &case.path(DOC), line, column, span);
    }

    #[test]
    fn single_quoted() {
        assert_carrier("'enum(macOS, {{ ''x'' == ''x'' }})'");
    }

    #[test]
    fn double_quoted_with_escapes() {
        assert_carrier("\"enum(\\x6dacOS, {{ \\\"x\\\" == 'x' }})\"");
    }

    #[test]
    fn literal_block_with_strip_chomping() {
        assert_carrier("|-\n    enum(macOS, {{ 123 }})");
    }

    #[test]
    fn folded_block_continued_onto_a_second_line() {
        assert_carrier(">-\n    enum(macOS,\n    {{ 123 }})");
    }

    #[test]
    fn literal_block_with_an_indentation_indicator_and_keep_chomping() {
        assert_carrier("|2+\n    enum(macOS, {{ 123 }})\n");
    }
}

/// R1-F3.3, R1-F3.4: a trigger's inline payload slot is in the trigger
/// file.
mod trigger_inline {
    use super::*;

    const SANDBOX_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n    os: enum({{ load_data(\"../data/os.yaml\", \"string[]\") }}; required)\n";

    fn case() -> Case {
        Case::new(
            &[(TRIGGER, SANDBOX_TRIGGER), (DOC, "---\ntopic: sandbox\nos: Linux\n---\nBody.\n")],
            &[("data/os.yaml", "- Linux\n")],
        )
    }

    #[test]
    fn compose_reports_an_unapproved_read_at_the_slot_in_the_trigger_file() {
        let case = case();
        let span = nth_slot(SANDBOX_TRIGGER, 0);
        assert_at(&case.compose_records(DOC), APPROVAL_REQUIRED, &case.path(TRIGGER), 5, 14, span);
    }

    #[test]
    fn schema_validate_reports_an_unapproved_read_at_the_slot_in_the_trigger_file() {
        let case = case();
        let span = nth_slot(SANDBOX_TRIGGER, 0);
        assert_at(&case.validate_records(DOC), APPROVAL_REQUIRED, &case.path(TRIGGER), 5, 14, span);
    }

    /// R1-F3.18: an edit above the payload that leaves its value unchanged
    /// moves the reported position with it.
    #[test]
    fn an_edit_above_an_unchanged_payload_moves_the_position() {
        let case = case();
        assert_at(&case.validate_records(DOC), APPROVAL_REQUIRED, &case.path(TRIGGER), 5, 14, nth_slot(SANDBOX_TRIGGER, 0));
        let edited = format!("# a comment\n{SANDBOX_TRIGGER}");
        case.write(TRIGGER, &edited);
        assert_at(&case.validate_records(DOC), APPROVAL_REQUIRED, &case.path(TRIGGER), 6, 14, nth_slot(&edited, 0));
    }

    /// R1-F3.19: a trigger whose payload references a schema file reports
    /// the slot in that file.
    #[test]
    fn a_referenced_payload_slot_is_in_the_schema_file() {
        let schema = "$schema:\n  os: enum({{ 123 }})\n";
        let case = Case::new(
            &[
                (TRIGGER, "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema: ./os.schema.yaml\n"),
                ("schemas/os.schema.yaml", schema),
                (DOC, "---\ntopic: sandbox\nos: Linux\n---\nBody.\n"),
            ],
            &[],
        );
        let file = case.path("schemas/os.schema.yaml");
        assert_at(&case.validate_records(DOC), WRONG_SHAPE, &file, 2, 12, nth_slot(schema, 0));
        assert_at(&case.compose_records(DOC), WRONG_SHAPE, &file, 2, 12, nth_slot(schema, 0));
    }
}

/// R1-F3.5–R1-F3.8: a referenced schema file and an imported named type
/// already keep their text; retain their positions.
mod controls {
    use super::*;

    const SCHEMA: &str = "$schema:\n  os: enum({{ 123 }})\n";
    const TYPES: &str = "kind: schema\ntypes:\n  Name: enum({{ 123 }})\n";

    fn referenced() -> Case {
        Case::new(&[("docs/os.schema.yaml", SCHEMA), (DOC, "---\n$schema: ./os.schema.yaml\nos: Linux\n---\n")], &[])
    }

    fn imported() -> Case {
        Case::new(&[("docs/types.yaml", TYPES), (DOC, "---\n$schema:\n  os: Name@./types.yaml\nos: Linux\n---\n")], &[])
    }

    #[test]
    fn compose_reports_a_referenced_schema_slot_in_its_file() {
        let case = referenced();
        assert_at(&case.compose_records(DOC), WRONG_SHAPE, &case.path("docs/os.schema.yaml"), 2, 12, nth_slot(SCHEMA, 0));
    }

    #[test]
    fn schema_validate_reports_a_referenced_schema_slot_in_its_file() {
        let case = referenced();
        assert_at(&case.validate_records(DOC), WRONG_SHAPE, &case.path("docs/os.schema.yaml"), 2, 12, nth_slot(SCHEMA, 0));
    }

    #[test]
    fn compose_reports_an_imported_type_slot_in_its_file() {
        let case = imported();
        assert_at(&case.compose_records(DOC), WRONG_SHAPE, &case.path("docs/types.yaml"), 3, 14, nth_slot(TYPES, 0));
    }

    #[test]
    fn schema_validate_reports_an_imported_type_slot_in_its_file() {
        let case = imported();
        assert_at(&case.validate_records(DOC), WRONG_SHAPE, &case.path("docs/types.yaml"), 3, 14, nth_slot(TYPES, 0));
    }
}

/// R1-F3.20: what cannot be proven is null, never a guess: a `$schema`
/// given in place of the document's own (an operator override) has no
/// authored text in the document.
mod unproven {
    use super::*;

    #[test]
    fn an_override_schema_slot_names_no_position() {
        let case = Case::document("os: Linux\n");
        let document = case.path(DOC);
        let request = case.request(&document);
        let schemas = DarkmatterSchemas::new(request.resolution_context().clone())
            .with_slot_session(Arc::new(SlotSession::for_request(&request)));
        let markdown =
            Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document.clone()));
        let schema: serde_json::Value = serde_json::json!({ "os": "enum({{ 123 }})" });
        let error = schemas.effective_for_with_override(&markdown, Some(&schema)).err().expect("a slot failure");
        let records = template_diagnostics(&error, &document).expect("a template failure");
        assert_eq!(records.len(), 1, "{records:#?}");
        let source = records[0].source.as_ref();
        assert_eq!(source.and_then(|source| source.line), None, "{records:#?}");
        assert_eq!(records[0].span, None, "{records:#?}");
    }
}

/// R1-F3.21: a cycle between two templates whose slots load data under
/// each other's types is at the outer slot, in its schema file.
mod cycle {
    use super::*;

    const A: &str = "kind: schema\n$schema:\n    os: enum({{ load_data(\"./data.yaml\", \"Os@./b.yaml\") }}; required)\ntypes:\n    Items: enum({{ load_data(\"./data.yaml\", \"Os@./b.yaml\") }})[]\n";
    const B: &str = "kind: schema\ntypes:\n    Os: enum({{ load_data(\"./data.yaml\", \"Items@./a.yaml\") }})[]\n";

    #[test]
    fn a_cycle_is_at_the_outer_slot_in_its_file() {
        let case = Case::new(
            &[
                ("docs/a.yaml", A),
                ("docs/b.yaml", B),
                ("docs/data.yaml", "- macOS\n"),
                (DOC, "---\n$schema: ./a.yaml\nos: macOS\n---\nBody.\n"),
            ],
            &[],
        );
        let file = case.path("docs/a.yaml");
        assert_at(&case.compose_records(DOC), "dm.schema.slot_cycle", &file, 3, 14, nth_slot(A, 0));
        assert_at(&case.validate_records(DOC), "dm.schema.slot_cycle", &file, 3, 14, nth_slot(A, 0));
    }
}

/// R1-F3.22: a whole YAML document's inline root `$schema` slot is in that
/// document.
mod yaml_document {
    use super::*;

    #[test]
    fn a_yaml_document_inline_schema_slot_is_in_the_document() {
        let text = "# settings\n$schema:\n  os: \"enum({{ 123 }})\"\nos: Linux\n";
        let case = Case::new(&[("docs/settings.yaml", text)], &[]);
        let path = case.path("docs/settings.yaml");
        let request = case.request(&path);
        let context = request.resolution_context().for_cwd(path.parent().expect("folder"));
        let schemas =
            DarkmatterSchemas::new(context).with_slot_session(Arc::new(SlotSession::for_request(&request)));
        let Err(error) = schemas.validate_yaml_document(text, &path) else { panic!("the slot fails") };
        let records = template_diagnostics(&error, &path).expect("a template failure");
        let span = nth_slot(text, 0);
        let (line, column) = line_column(text, span.start);
        assert_at(&records, WRONG_SHAPE, &path, line, column, span);
    }
}

/// R1-F3.25: a refused read inside a slot is positioned at the read when
/// the authored expression is its text, and at the whole expression when a
/// YAML escape makes the authored bytes differ.
mod refused {
    use super::*;

    const FORBIDDEN: &str = "dm.schema.slot_forbidden_binding";

    #[test]
    fn a_refused_binding_in_a_plain_carrier_is_at_the_binding() {
        let case = Case::document("$schema:\n  os: enum({{ 'a' == fm.x }})\nos: a\n");
        let text = case.text(DOC);
        let start = text.find("fm.x").expect("the binding");
        let (line, column) = line_column(&text, start);
        assert_at(&case.validate_records(DOC), FORBIDDEN, &case.path(DOC), line, column, start..start + 4);
    }

    #[test]
    fn a_refused_binding_after_an_escape_is_at_the_whole_expression() {
        let case = Case::document("$schema:\n  os: \"enum({{ \\\"a\\\" == fm.x }})\"\nos: a\n");
        let text = case.text(DOC);
        let start = text.find("{{").expect("a slot") + 3;
        let end = text.find(" }}").expect("the slot's end");
        let (line, column) = line_column(&text, start);
        assert_at(&case.validate_records(DOC), FORBIDDEN, &case.path(DOC), line, column, start..end);
    }
}

/// R1-F3.26, R1-F3.28: a `load_data` contract naming a template whose own
/// slot fails is reported at the slot that read the data, in the document,
/// with the code (and data location, and grants needed) the contract slot's
/// failure gets when it happens directly in a slot.
mod contract_template {
    use super::*;

    const DOC_TEXT: &str =
        "---\n$schema:\n  os: enum({{ load_data(\"./os.yaml\", \"Os@./types.yaml\") }})\nos: Linux\n---\n";

    /// The document reading `os.yaml` under `Os@./types.yaml`, whose
    /// definition is `os_type`; `untracked` files get no automatic grant.
    fn case(os_type: &str, files: &[(&str, &str)], untracked: &[(&str, &str)]) -> Case {
        let types = format!("kind: schema\ntypes:\n  Os: {os_type}\n");
        let mut committed = vec![("docs/types.yaml", types.as_str()), ("docs/os.yaml", "- Linux\n"), (DOC, DOC_TEXT)];
        committed.extend_from_slice(files);
        Case::new(&committed, untracked)
    }

    /// Both reports of `case`, each asserted to be one record with `code` at
    /// the document's reading slot.
    fn records_at_the_reading_slot(case: &Case, code: &str) -> Vec<Vec<Diagnostic>> {
        let reports = vec![case.compose_records(DOC), case.validate_records(DOC)];
        for records in &reports {
            assert_at(records, code, &case.path(DOC), 3, 12, nth_slot(DOC_TEXT, 0));
        }
        reports
    }

    #[test]
    fn a_wrong_shape_keeps_its_code() {
        let case = case("enum({{ 123 }})[]", &[], &[]);
        records_at_the_reading_slot(&case, WRONG_SHAPE);
    }

    #[test]
    fn an_unapproved_read_keeps_its_code_and_grants_needed() {
        let case = case("enum({{ load_data(\"./names.yaml\", \"string[]\") }})[]", &[], &[("docs/names.yaml", "- Linux\n")]);
        for records in records_at_the_reading_slot(&case, APPROVAL_REQUIRED) {
            let needed = records[0].grants_needed.as_ref().expect("grants needed");
            assert!(!needed.is_empty(), "{records:#?}");
        }
    }

    #[test]
    fn a_data_failure_keeps_its_code_and_data_location() {
        let case =
            case("enum({{ load_data(\"./names.yaml\", \"string[]\") }})[]", &[("docs/names.yaml", "- Linux\n- [nested]\n")], &[]);
        for records in [case.compose_records(DOC), case.validate_records(DOC)] {
            let code = records.first().map(|record| record.code.clone()).unwrap_or_default();
            assert!(code.starts_with("dm.data."), "{records:#?}");
            assert_at(&records, &code, &case.path(DOC), 3, 12, nth_slot(DOC_TEXT, 0));
            let location = records[0].data_location.as_ref().expect("a data location");
            assert!(location.file.ends_with("names.yaml"), "{records:#?}");
            assert_eq!(location.line, Some(2), "{records:#?}");
        }
    }
}
