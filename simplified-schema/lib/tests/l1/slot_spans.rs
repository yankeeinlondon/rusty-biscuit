//! Source spans of `{{ … }}` slots: the slot's `span` and `expression_span`
//! point at the authored text in the YAML or Markdown document, through every
//! scalar carrier, and the structural scanners read a slot as one argument
//! whatever punctuation it holds.

use std::ops::Range;

use serde_yaml_ng::Value as YamlValue;
use simplified_schema::simplified::{
    DefinitionTable, SchemaDeclaration, SchemaSourcePath, SchemaSpanKind, Slot, parse_schema_declaration_with_source,
    parse_yaml_schema_with_source,
};
use simplified_schema::slots::collect_slots;
use simplified_schema::SimplifiedSchema;

/// The slots of `yaml` parsed with source projection, `offset` bytes into a
/// containing document.
fn projected_slots(yaml: &str, offset: usize) -> Vec<Slot> {
    let value: YamlValue = serde_yaml_ng::from_str(yaml).expect("test YAML parses");
    let root = parse_yaml_schema_with_source(&value, yaml, offset).unwrap_or_else(|error| panic!("{error}\n{yaml}"));
    collect_slots(&root, &DefinitionTable::default())
}

#[track_caller]
fn assert_slot_text(document: &str, slot: &Slot, whole: &str, expression: &str) {
    assert_eq!(&document[slot.span.clone()], whole, "slot span of {slot:?}");
    assert_eq!(&document[slot.expression_span.clone()], expression, "expression span of {slot:?}");
}

#[test]
fn a_plain_scalar_slot_points_at_its_braces_and_expression() {
    let yaml = "os: enum(a, {{ load_data(\"x.yaml\", \"string[]\") }}; required)\n";
    let slots = projected_slots(yaml, 0);
    assert_eq!(slots.len(), 1);
    assert_slot_text(
        yaml,
        &slots[0],
        "{{ load_data(\"x.yaml\", \"string[]\") }}",
        "load_data(\"x.yaml\", \"string[]\")",
    );
}

#[test]
fn quoted_carriers_project_through_their_escapes() {
    let single = "os: 'enum(a, {{ load_data(''x.yaml'') }})'\n";
    let slots = projected_slots(single, 0);
    assert_eq!(slots[0].expression, "load_data('x.yaml')", "the decoded expression");
    assert_slot_text(single, &slots[0], "{{ load_data(''x.yaml'') }}", "load_data(''x.yaml'')");

    let double = "os: \"enum(a, {{ load_data(\\\"x.yaml\\\") }})\"\n";
    let slots = projected_slots(double, 0);
    assert_eq!(slots[0].expression, "load_data(\"x.yaml\")");
    assert_slot_text(double, &slots[0], "{{ load_data(\\\"x.yaml\\\") }}", "load_data(\\\"x.yaml\\\")");
}

/// The slots of a standalone schema document as the factory reads it: the
/// located path every schema file takes, which follows block scalars.
fn document_slots(text: &str) -> Vec<Slot> {
    let schema = SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("{error}\n{text}"));
    schema.pending_slots().to_vec()
}

#[test]
fn block_scalar_carriers_keep_authored_positions() {
    let literal = "$schema:\n  os: |\n    enum(a, {{ x }}; required)\n";
    assert_slot_text(literal, &document_slots(literal)[0], "{{ x }}", "x");

    let folded = "$schema:\n  os: >\n    enum(a,\n    {{ y }})\n";
    assert_slot_text(folded, &document_slots(folded)[0], "{{ y }}", "y");

    let quoted = "$schema:\n  os: 'enum(a, {{ f(''q'') }})'\n";
    assert_slot_text(quoted, &document_slots(quoted)[0], "{{ f(''q'') }}", "f(''q'')");

    let crlf = "$schema:\r\n  first: string\r\n  os: enum(a, {{ z }})\r\n";
    assert_slot_text(crlf, &document_slots(crlf)[0], "{{ z }}", "z");
    let crlf = "first: string\r\nos: enum(a, {{ z }})\r\n";
    assert_slot_text(crlf, &projected_slots(crlf, 0)[0], "{{ z }}", "z");
}

#[test]
fn a_frontmatter_offset_moves_every_span_into_the_document() {
    let yaml = "os: enum(a, {{ x }})\n";
    let document = format!("---\n{yaml}---\n# Body\n");
    let slots = projected_slots(yaml, 4);
    assert_slot_text(&document, &slots[0], "{{ x }}", "x");

    // A Markdown document's `$schema`, through the factory.
    let markdown = "---\ntitle: t\n$schema:\n  os: \"enum(a, {{ x }})\"\n---\n# Body {{ not_a_slot }}\n";
    let slots = document_slots(markdown);
    assert_eq!(slots.len(), 1, "body expressions are not schema slots");
    assert_slot_text(markdown, &slots[0], "{{ x }}", "x");
}

#[test]
fn every_slot_position_and_suggestion_projects() {
    let yaml = "a: string(suggest(lit, {{ s }}); pattern({{ p }}))[](max({{ n }}))\n\
                b: literal({{ v }})\n\
                c: number(min({{ low }}); default({{ d }}))\n";
    let found: Vec<(String, &str)> = projected_slots(yaml, 0)
        .iter()
        .map(|slot| (slot.expression.clone(), &yaml[slot.span.clone()]))
        .collect();
    let expected = ["s", "p", "n", "v", "low", "d"];
    assert_eq!(found.len(), expected.len(), "{found:?}");
    for ((expression, text), wanted) in found.iter().zip(expected) {
        assert_eq!(expression, wanted);
        assert_eq!(*text, format!("{{{{ {wanted} }}}}"));
    }
}

#[test]
fn a_slot_holding_separators_is_one_argument_in_the_source_map() {
    let yaml = "os: enum(a, {{ g(x, [1, 2], {k:'v;)'}) }}; required)\n";
    let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
    let declared = parse_schema_declaration_with_source(&value, yaml, 0).expect("projects");
    let SchemaDeclaration::Schema(root) = &declared.value else { panic!("inline schema") };
    let slots = collect_slots(root, &DefinitionTable::default());
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].expression, "g(x, [1, 2], {k:'v;)'})");
    // The structural map reads the member list as one constraint and the
    // slot as no call of its own: neither its `;` nor its `(` is structure.
    let constraints: Vec<&str> = declared
        .source_map
        .spans(&SchemaSourcePath::root().property("os"), SchemaSpanKind::Constraint)
        .iter()
        .map(|span: &Range<usize>| &yaml[span.clone()])
        .collect();
    assert_eq!(constraints, vec!["a, {{ g(x, [1, 2], {k:'v;)'}) }}", "required"]);
    assert!(
        declared.source_map.spans(&SchemaSourcePath::root().property("os"), SchemaSpanKind::Argument).is_empty(),
        "a slot's parentheses are not a constraint call"
    );
}

#[test]
fn a_factory_built_template_reports_spans_in_its_own_text() {
    let text = include_str!("../fixtures/dynamic_slots/schema-with-slot.yaml");
    let schema = SimplifiedSchema::try_from(text).expect("admitted");
    let slot = &schema.pending_slots()[0];
    assert_slot_text(
        text,
        slot,
        "{{ load_data(\"./os.yaml\", \"string[]\") }}",
        "load_data(\"./os.yaml\", \"string[]\")",
    );
}

#[cfg(feature = "file")]
#[test]
fn a_slot_in_an_imported_type_points_into_the_defining_file() {
    use std::path::Path;

    use biscuit_file::FileResolutionContext;
    use simplified_schema::SchemaSource;

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots");
    let entry = fixtures.join("imported-types.yaml");
    let defining = std::fs::read_to_string(fixtures.join("nested/platform-types.yaml")).unwrap();
    let schema = SimplifiedSchema::try_from_yaml(SchemaSource::from(&entry).bound(FileResolutionContext::new(&fixtures)))
        .expect("an imported template is admitted");
    assert!(!schema.is_resolved());
    let slot = &schema.pending_slots()[0];
    assert_eq!(slot.expression, "load_data(\"./os.yaml\", \"string[]\")");
    assert_slot_text(&defining, slot, "{{ load_data(\"./os.yaml\", \"string[]\") }}", "load_data(\"./os.yaml\", \"string[]\")");
}
