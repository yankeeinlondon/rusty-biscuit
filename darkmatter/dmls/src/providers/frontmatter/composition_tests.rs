//! Composition operands (`...Name` spreads and `$extends` values) through the
//! frontmatter provider: which named types completion offers, the range each
//! edit replaces, where `$extends` is a directive at all, and hover and
//! go-to-definition on an operand's name.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

use lsp_types::{InitializeParams, Uri};

use super::*;
use crate::capabilities::ClientProfile;
use crate::config::DmlsConfig;
use crate::graph::WorkspaceGraph;
use crate::overlay::OverlayState;
use crate::source_map::{PositionEncoding, SourceMap};

/// Where the cursor sits in a fixture; removed before the text is served.
const CURSOR: &str = "<|>";

/// An imported library: one object, one primitive.
const AUDIT: &str = "kind: schema\ntypes:\n  audit: \"{ at: string }\"\n  stamp: string\n";

/// `document` with its [`CURSOR`] removed, and the cursor's offset.
fn at_cursor(document: &str) -> (String, usize) {
    let offset = document.find(CURSOR).expect("fixture marks the cursor");
    (document.replacen(CURSOR, "", 1), offset)
}

/// Writes `files` and `name` (holding `text`) into a fresh folder and runs
/// `f` with the document's context.
fn with_document<R>(files: &[(&str, &str)], name: &str, text: &str, f: impl FnOnce(&DocumentContext) -> R) -> R {
    with_edited_document(files, name, None, text, f)
}

/// [`with_document`] for a buffer an editor reached from `last_good`, which
/// is served first, so a buffer that does not parse is read over that
/// text's tree as an open document is.
fn with_edited_document<R>(
    files: &[(&str, &str)],
    name: &str,
    last_good: Option<&str>,
    text: &str,
    f: impl FnOnce(&DocumentContext) -> R,
) -> R {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    for (file, source) in files {
        std::fs::write(root.join(file), source).expect("fixture file");
    }
    let path = root.join(name);
    std::fs::write(&path, text).expect("document");
    let uri: Uri = file_path_to_uri(&path).expect("document uri");
    let config = DmlsConfig::default();
    let roots = [root.clone()];
    let resolution = crate::context::test_support::resolution_in_repository(&root, &path);
    let state = OverlayState::default();
    if let Some(last_good) = last_good {
        state.for_document(&uri, last_good, &path, &config, &roots, &resolution);
    }
    let overlay = state.for_document(&uri, text, &path, &config, &roots, &resolution);
    let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
    let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &crate::context::test_support::workspace_contexts());
    let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
    let ctx = DocumentContext {
        uri: &uri,
        path: &path,
        text,
        source_map: &source_map,
        graph: &graph,
        doc_id: None,
        config: &config,
        profile: &profile,
        overlay: overlay.as_ref(),
        resolution: &resolution,
        host: None,
    };
    f(&ctx)
}

/// The labels completion offers at the cursor of `document` (served as
/// `name` beside [`AUDIT`]), and the byte range each item's edit replaces.
fn offered(name: &str, document: &str) -> Vec<(String, Range<usize>)> {
    offered_after(name, None, document)
}

/// [`offered`] for a buffer an editor reached from `last_good`; see
/// [`with_edited_document`].
fn offered_after(name: &str, last_good: Option<&str>, document: &str) -> Vec<(String, Range<usize>)> {
    let (text, offset) = at_cursor(document);
    with_edited_document(&[("audit.yaml", AUDIT)], name, last_good, &text, |ctx| {
        completion(ctx, offset)
            .into_iter()
            .map(|item| {
                let Some(CompletionTextEdit::Edit(edit)) = &item.text_edit else {
                    panic!("eager text edit: {item:?}");
                };
                (item.label, byte_range(&text, edit.range))
            })
            .collect()
    })
}

/// The byte range of an LSP range over `text` (ASCII fixtures, so UTF-16
/// columns are bytes).
fn byte_range(text: &str, range: lsp_types::Range) -> Range<usize> {
    let offset = |position: lsp_types::Position| {
        let line_start: usize = text.split_inclusive('\n').take(position.line as usize).map(str::len).sum();
        line_start + position.character as usize
    };
    offset(range.start)..offset(range.end)
}

fn labels(items: &[(String, Range<usize>)]) -> Vec<&str> {
    items.iter().map(|(label, _)| label.as_str()).collect()
}

/// Asserts the exact labels offered, each replacing `replaced` (a range of
/// the document after its cursor marker is removed).
fn assert_offered(items: &[(String, Range<usize>)], expected: &[&str], replaced: Range<usize>) {
    assert_eq!(labels(items), expected, "{items:#?}");
    for (label, range) in items {
        assert_eq!(range, &replaced, "`{label}` replaces the operand typed so far");
    }
}

/// [`assert_offered`] for a standalone buffer that does not parse, whose
/// import catalog is only as complete as its last parse: the local names
/// are exact, and an import, when offered, is the eligible one.
fn assert_local_offered(items: &[(String, Range<usize>)], expected: &[&str], replaced: Range<usize>) {
    let (local, imported): (Vec<_>, Vec<_>) = items.iter().cloned().partition(|(label, _)| !label.contains("@./"));
    assert_offered(&local, expected, replaced.clone());
    assert!(labels(&imported).iter().all(|label| *label == "audit@./audit.yaml"), "{imported:#?}");
    assert!(imported.iter().all(|(_, range)| *range == replaced), "{imported:#?}");
}

/// Every eligible local name, in authored order.
const ELIGIBLE_LOCAL: [&str; 5] = ["base", "chain", "string@this", "remote", "imported"];

/// A standalone library whose `types` table holds every eligibility case,
/// with `event: {event}` last.
fn library(event: &str) -> String {
    format!(
        "kind: schema\ntypes:\n  base: \"{{ id: string }}\"\n  label: string\n  label_alias: label\n  either: [string, number]\n  \
         many: \"base[]\"\n  opaque: object\n  chain: base\n  chained_union: either\n  loop_a: loop_b\n  \
         loop_b: loop_a\n  string: \"{{ s: string }}\"\n  remote: \"far@./missing.yaml\"\n  \
         imported: \"audit@./audit.yaml\"\n  composes_event: \"{{ ...event }}\"\n  event: {event}\n"
    )
}

/// Every eligible local name in authored order, then the eligible export of
/// the imported library.
const ELIGIBLE: [&str; 6] = ["base", "chain", "string@this", "remote", "imported", "audit@./audit.yaml"];

/// The range of `needle`'s `occurrence`-th match in `text`.
fn span_of(text: &str, needle: &str, occurrence: usize) -> Range<usize> {
    let start = text.match_indices(needle).nth(occurrence).expect("needle in text").0;
    start..start + needle.len()
}

#[test]
fn composition_spread_offers_only_eligible_objects() {
    // The empty operand is a grammar error, so the namespace is read entry
    // by entry from the buffer.
    let document = library("\"{ ...<|> }\"");
    let (_, offset) = at_cursor(&document);
    assert_local_offered(&offered("lib.yaml", &document), &ELIGIBLE_LOCAL, offset..offset);

    // A partial name parses, so the current declaration supplies it; the
    // edit replaces the whole operand typed so far.
    let document = library("\"{ ...ch<|> }\"");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["chain"], offset - 2..offset);
}

#[test]
fn composition_partial_name_replaces_whole_operand_with_an_import() {
    let document = library("\"{ ...au<|> }\"");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["audit@./audit.yaml"], offset - 2..offset);
}

#[test]
fn composition_reference_half_offers_this_and_defining_files() {
    let document = library("\"{ ...base@<|> }\"");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["this"], offset..offset);

    let document = library("\"{ ...audit@./a<|> }\"");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["./audit.yaml"], offset - 3..offset);

    // A primitive is offered after `@` from neither namespace.
    let document = library("\"{ ...stamp@<|> }\"");
    assert_offered(&offered("lib.yaml", &document), &[], 0..0);
}

#[test]
fn composition_nested_spread_may_name_the_enclosing_type() {
    // A child value is a descending hop, so `event` is no cycle there.
    let document = library("\"{ child: { ...ev<|> } }\"");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["event"], offset - 2..offset);
}

#[test]
fn composition_spread_in_an_array_item_object() {
    let document = library("\"{ ...ba<|>}[]\"");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["base"], offset - 2..offset);
}

#[test]
fn composition_extends_scalar_plain_and_quoted() {
    for value in ["ba<|>", "\"ba<|>", "'ba<|>'"] {
        let document = library(&format!("\n    $extends: {value}\n    price: number"));
        let (_, offset) = at_cursor(&document);
        assert_offered(&offered("lib.yaml", &document), &["base"], offset - 2..offset);
    }
    // An empty value offers every eligible object except the receiver.
    let document = library("\n    $extends: <|>\n    price: number");
    let (_, offset) = at_cursor(&document);
    assert_local_offered(&offered("lib.yaml", &document), &ELIGIBLE_LOCAL, offset..offset);
}

#[test]
fn composition_extends_with_a_quoted_key() {
    for key in ["\"$extends\"", "'$extends'"] {
        let document = library(&format!("\n    {key}: ba<|>"));
        let (_, offset) = at_cursor(&document);
        assert_offered(&offered("lib.yaml", &document), &["base"], offset - 2..offset);
    }
}

#[test]
fn composition_extends_flow_sequence_items() {
    let document = library("\n    $extends: [<|>");
    let (_, offset) = at_cursor(&document);
    assert_local_offered(&offered("lib.yaml", &document), &ELIGIBLE_LOCAL, offset..offset);

    let document = library("\n    $extends: [ch<|>]");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["chain"], offset - 2..offset);

    let document = library("\n    $extends: [base, au<|>]");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["audit@./audit.yaml"], offset - 2..offset);

    let document = library("\n    $extends: [base, audit@./<|>]");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["./audit.yaml"], offset - 2..offset);
}

#[test]
fn composition_extends_block_sequence_item() {
    let document = library("\n    $extends:\n      - base\n      - au<|>");
    let (_, offset) = at_cursor(&document);
    assert_offered(&offered("lib.yaml", &document), &["audit@./audit.yaml"], offset - 2..offset);

    let document = library("\n    $extends:\n      - <|>");
    let (_, offset) = at_cursor(&document);
    assert_local_offered(&offered("lib.yaml", &document), &ELIGIBLE_LOCAL, offset..offset);
}

#[test]
fn composition_frontmatter_offers_every_eligible_local_and_imported_object() {
    // The frontmatter catalog is read entry by entry, so the import is
    // known while the operand is still empty.
    for value in ["\"{ ...<|> }\"", "\n    $extends: <|>", "\n    $extends:\n      - base\n      - <|>"] {
        let document = library(value).replacen("kind: schema\ntypes:\n", "---\n$schema:\n", 1) + "---\n\nBody\n";
        let (_, offset) = at_cursor(&document);
        assert_offered(&offered("doc.md", &document), &ELIGIBLE, offset..offset);
    }
}

#[test]
fn composition_in_a_frontmatter_declaration() {
    let document = "---\n$schema:\n  base: \"{ id: string }\"\n  tag: string\n  event:\n    $extends: <|>\n---\n\nBody\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("doc.md", document), &["base"], offset..offset);

    let document = "---\n$schema:\n  base: \"{ id: string }\"\n  tag: string\n  event: \"{ ...b<|> }\"\n---\n\nBody\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("doc.md", document), &["base"], offset - 1..offset);
}

#[test]
fn composition_in_a_value_typed_as_a_type_definition() {
    let document = "---\n$schema:\n  shape: type-definition\n  base: \"{ id: string }\"\nshape:\n  $extends: ba<|>\n---\n\nBody\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("doc.md", document), &["base"], offset - 2..offset);
}

#[test]
fn composition_root_union_arm_scope_is_the_cursor_arm() {
    let document = "kind: schema\n$schema:\n  - left: \"{ l: string }\"\n    pick: \"{ ...<|> }\"\n  - right: \"{ r: string }\"\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("arms.yaml", document), &["left"], offset..offset);

    let document = "kind: schema\n$schema:\n  - left: \"{ l: string }\"\n  - right: \"{ r: string }\"\n    pick:\n      $extends: <|>\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("arms.yaml", document), &["right"], offset..offset);

    // A parsed declaration selects the arm by its authored span.
    let document = "---\n$schema:\n  - left: \"{ l: string }\"\n    pick: \"{ ...l<|> }\"\n  - lone: \"{ r: string }\"\n---\n\nBody\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("doc.md", document), &["left"], offset - 1..offset);
}

#[test]
fn composition_extends_outside_a_schema_object_keeps_ordinary_completion() {
    // Frontmatter data, not a declaration.
    let document = "---\n$schema:\n  base: \"{ id: string }\"\n$extends: ba<|>\n---\n\nBody\n";
    let items = offered("doc.md", document);
    assert!(labels(&items).iter().all(|label| *label != "base"), "{items:#?}");

    // A `types` table names a type `$extends`; its value is a type definition.
    let document = "kind: schema\ntypes:\n  base: \"{ id: string }\"\n  $extends: str<|>\n";
    let items = offered("lib.yaml", document);
    assert!(labels(&items).contains(&"string"), "{items:#?}");
    assert!(labels(&items).contains(&"string[]"), "{items:#?}");

    // So does a frontmatter value typed `schema(mapping)`, which is read as
    // a table; one typed `schema` is a schema object, where it composes.
    for (declaration, composes) in [("schema(mapping)", false), ("schema", true)] {
        let document =
            format!("---\n$schema:\n  defs: {declaration}\ndefs:\n  base: \"{{ id: string }}\"\n  $extends: <|>\n---\n\nBody\n");
        let items = offered("doc.md", &document);
        assert_eq!(!labels(&items).contains(&"string"), composes, "{declaration}: {items:#?}");
    }
}

#[test]
fn composition_operands_offer_no_keywords_or_scaffolds() {
    for value in ["\"{ ...<|> }\"", "\n    $extends: <|>"] {
        let items = offered("lib.yaml", &library(value));
        for keyword in ["string", "number", "object", "{}", "[]", "Name@./types.yaml", "required"] {
            assert!(labels(&items).iter().all(|label| *label != keyword), "`{keyword}` offered: {items:#?}");
        }
    }
}

/// A complete library whose operands all name `base` or the import.
const NAVIGABLE: &str = "kind: schema\ntypes:\n  base: \"{ id: string }\"\n  spread: \"{ ...base, price: number }\"\n  \
                         scalar:\n    $extends: base\n  listed:\n    $extends: [base, audit@./audit.yaml]\n";

fn hover_markup(ctx: &DocumentContext, offset: usize) -> String {
    let hover = hover(ctx, offset).expect("hover on the operand");
    let HoverContents::Markup(MarkupContent { value, .. }) = hover.contents else {
        panic!("markdown hover");
    };
    value
}

#[test]
fn composition_operand_hover_and_definition() {
    let text = NAVIGABLE;
    with_document(&[("audit.yaml", AUDIT)], "lib.yaml", text, |ctx| {
        let base_key = ctx.source_map.byte_range_to_lsp(span_of(text, "base", 0)).unwrap();
        // The spread, the scalar, and the first sequence item.
        for occurrence in 1..=3 {
            let offset = span_of(text, "base", occurrence).start + 1;
            assert!(hover_markup(ctx, offset).contains("Defined in: this schema"), "occurrence {occurrence}");
            let locations = definition(ctx, offset);
            assert_eq!(locations.len(), 1, "occurrence {occurrence}: {locations:?}");
            assert_eq!(locations[0].uri, *ctx.uri);
            assert_eq!(locations[0].range, base_key, "occurrence {occurrence}");
        }

        let offset = span_of(text, "audit@", 0).start + 1;
        assert!(hover_markup(ctx, offset).contains("Defined in: `./audit.yaml`"));
        let locations = definition(ctx, offset);
        assert_eq!(locations.len(), 1, "{locations:?}");
        assert!(locations[0].uri.as_str().ends_with("/audit.yaml"), "{:?}", locations[0].uri);
        let audit_key = span_of(AUDIT, "audit", 0);
        let line = u32::try_from(AUDIT[..audit_key.start].matches('\n').count()).unwrap();
        assert_eq!(locations[0].range.start, lsp_types::Position::new(line, 2));
    });
}

#[test]
fn composition_operand_definition_in_a_root_union_arm() {
    let text = "---\n$schema:\n  - a: \"{ x: string }\"\n    b: \"{ ...a }\"\n  - a: \"{ y: number }\"\n---\n\nBody\n";
    with_document(&[], "doc.md", text, |ctx| {
        let offset = span_of(text, "...a", 0).start + 3;
        let markup = hover_markup(ctx, offset);
        assert!(markup.contains("Defined in: this schema"), "{markup}");
        let locations = definition(ctx, offset);
        assert_eq!(locations.len(), 1, "{locations:?}");
        let first_a = span_of(text, "- a:", 0).start + 2;
        let first_a = ctx.source_map.byte_range_to_lsp(first_a..first_a + 1);
        assert_eq!(Some(locations[0].range), first_a);
    });
}

#[test]
fn composition_flow_item_framing() {
    let text = "[base, 'x, y', \"a\\\"b\", {c: d}, au";
    assert_eq!(flow_sequence_item_start(text, 0, text.len()), Some(text.len() - 2));
    assert_eq!(flow_sequence_item_start("[  ", 0, 3), Some(3));
    assert_eq!(flow_sequence_item_start("[a] ", 0, 4), None);
    let commented = "[a, # b, c\n  d";
    assert_eq!(flow_sequence_item_start(commented, 0, commented.len()), Some(commented.len() - 1));
}

#[test]
fn composition_extends_flow_sequence_continued_onto_later_lines() {
    // The review's shape: the receiver is the root `$schema`.
    let document = "kind: schema\n$schema:\n  $extends: [base,\n    b<|>]\ntypes:\n  base: '{ id: string }'\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("root.yaml", document), &["base"], offset - 1..offset);

    for (case, item, expected, typed) in [
        ("partial", "b<|>]", &["base"][..], 1),
        ("quoted", "'b<|>']", &["base"][..], 1),
        ("reference half", "base@<|>]", &["this"][..], 0),
        ("after a comment line", "# note\n      b<|>]", &["base"][..], 1),
        ("third line", "chain,\n      b<|>]", &["base"][..], 1),
    ] {
        let document = library(&format!("\n    $extends: [base,\n      {item}"));
        let (_, offset) = at_cursor(&document);
        let items = offered("lib.yaml", &document);
        assert_eq!(labels(&items), expected, "{case}: {items:#?}");
        assert!(items.iter().all(|(_, range)| *range == (offset - typed..offset)), "{case}: {items:#?}");
    }

    // An empty continued item offers every eligible object, not just names.
    let document = library("\n    $extends: [base,\n      <|>]");
    let (_, offset) = at_cursor(&document);
    assert_local_offered(&offered("lib.yaml", &document), &ELIGIBLE_LOCAL, offset..offset);
    // A closed same-line sequence still keeps the receiver out.
    let document = library("\n    $extends: [base, <|>]");
    let (_, offset) = at_cursor(&document);
    assert_local_offered(&offered("lib.yaml", &document), &ELIGIBLE_LOCAL, offset..offset);
    // Still eligible-only while the sequence is unclosed and the line is the last.
    let document = library("\n    $extends: [base,\n      <|>");
    let (_, offset) = at_cursor(&document);
    assert_local_offered(&offered("lib.yaml", &document), &ELIGIBLE_LOCAL, offset..offset);
}

#[test]
fn composition_continued_flow_sequence_in_frontmatter_and_union_arms() {
    let document = "---\n$schema:\n  base: \"{ id: string }\"\n  tag: string\n  event:\n    $extends: [base,\n      b<|>]\n---\n\nBody\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("doc.md", document), &["base"], offset - 1..offset);

    // Root-union arm: only the cursor arm's names are local.
    let document = "kind: schema\n$schema:\n  - left: \"{ l: string }\"\n  - right: \"{ r: string }\"\n    pick:\n      $extends: [right,\n        <|>]\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("arms.yaml", document), &["right"], offset..offset);
    // An arm that is itself `- $extends: [`.
    let document = "kind: schema\n$schema:\n  - $extends: [base,\n      b<|>]\n  - other: string\ntypes:\n  base: \"{ id: string }\"\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("arms.yaml", document), &["base"], offset - 1..offset);

    // A value typed as a type definition, and a flow-mapping receiver.
    let document = "---\n$schema:\n  shape: type-definition\n  base: \"{ id: string }\"\nshape:\n  $extends: [base,\n    ba<|>]\n---\n\nBody\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("doc.md", document), &["base"], offset - 2..offset);
    let document = "kind: schema\n$schema: {$extends: [base,\n    b<|>]}\ntypes:\n  base: \"{ id: string }\"\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("flow.yaml", document), &["base"], offset - 1..offset);

    // Property-union mapping arm, names from `types`.
    let document = "kind: schema\ntypes:\n  base: \"{ id: string }\"\n  pick:\n    - string\n    - $extends: [base,\n        b<|>]\n      x: number\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("lib.yaml", document), &["base"], offset - 1..offset);
}

#[test]
fn composition_continued_flow_sequence_elsewhere_is_not_an_operand() {
    // A continued frontmatter data list has no composition meaning.
    let document = "---\n$schema:\n  base: \"{ id: string }\"\n  tags: string[]\ntags: [a,\n  b<|>]\n---\n\nBody\n";
    let items = offered("doc.md", document);
    assert!(labels(&items).iter().all(|label| *label != "base"), "{items:#?}");

    // A continued union in `types` and a continued property list stay type
    // definitions, which offer keywords an operand never does.
    for document in [
        "kind: schema\ntypes:\n  base: \"{ id: string }\"\n  either: [string,\n    <|>]\n",
        "kind: schema\n$schema:\n  tags: [string,\n    <|>]\ntypes:\n  base: \"{ id: string }\"\n",
    ] {
        let items = offered("lib.yaml", document);
        assert!(labels(&items).contains(&"number"), "{items:#?}");
    }

    // A continued `$constraints` list offers object constraints only.
    let document = "kind: schema\n$schema:\n  $constraints: [closed,\n    <|>]\ntypes:\n  base: \"{ id: string }\"\n";
    let items = offered("lib.yaml", document);
    assert!(labels(&items).contains(&"min-keys") && !labels(&items).contains(&"base"), "{items:#?}");
}

/// A `types` table holds named declarations only: the library refuses
/// `$constraints` and pattern keys written directly in it, so neither is
/// offered as a type name, while `$extends` there is one.
#[test]
fn composition_types_table_object_only_keys_are_not_type_names() {
    let document = "kind: schema\ntypes:\n  $constraints:\n    min-keys: 1\n  '<string>': \"{ a: string }\"\n  \
                    $extends: \"{ b: string }\"\n  base: \"{ id: string }\"\n  event: \"{ ...<|> }\"\n";
    let (_, offset) = at_cursor(document);
    assert_local_offered(&offered("lib.yaml", document), &["$extends", "base"], offset..offset);

    // Nor does a `$constraints` block written directly in the table offer
    // constraint keys, while one inside a declaration's object still does.
    for (name, document, offers) in [
        ("lib.yaml", "kind: schema\ntypes:\n  base: \"{ id: string }\"\n  $constraints:\n    <|>\n", false),
        ("lib.yaml", "kind: schema\ntypes:\n  base:\n    id: string\n    $constraints:\n      <|>\n", true),
        ("doc.md", "---\n$schema:\n  defs: schema(mapping)\ndefs:\n  $constraints:\n    <|>\n---\n\nBody\n", false),
        ("doc.md", "---\n$schema:\n  defs: schema\ndefs:\n  $constraints:\n    <|>\n---\n\nBody\n", true),
    ] {
        let items = offered(name, document);
        assert_eq!(labels(&items).contains(&"min-keys"), offers, "{document}: {items:#?}");
    }
}

#[test]
fn composition_spread_in_a_wrapped_quoted_definition() {
    for quote in ["\"", "'"] {
        let document = library(&format!("{quote}{{ at: string,\n    ...ba<|> }}{quote}"));
        let (_, offset) = at_cursor(&document);
        assert_offered(&offered("lib.yaml", &document), &["base"], offset - 2..offset);

        // An empty operand: the namespace is read with the whole wrapped
        // definition blanked, so the receiver and ineligible names stay out.
        let document = library(&format!("{quote}{{ at: string,\n    ...<|> }}{quote}"));
        let (_, offset) = at_cursor(&document);
        assert_local_offered(&offered("lib.yaml", &document), &ELIGIBLE_LOCAL, offset..offset);
    }

    let document = "---\n$schema:\n  base: \"{ id: string }\"\n  tag: string\n  event: \"{ at: string,\n    ...b<|> }\"\n---\n\nBody\n";
    let (_, offset) = at_cursor(document);
    assert_offered(&offered("doc.md", document), &["base"], offset - 1..offset);

    // Past the closing quote the cursor is no longer in the definition.
    let document = library("\"{ at: string,\n    ...base }\" <|>");
    let items = offered("lib.yaml", &document);
    assert!(labels(&items).iter().all(|label| *label != "base"), "{items:#?}");
}

/// A [`library`] document presented as Markdown frontmatter.
fn in_frontmatter(standalone: &str) -> String {
    standalone.replacen("kind: schema\ntypes:\n", "---\n$schema:\n", 1) + "---\n\nBody\n"
}

/// How the completion offered at the cursor of `document` differs from the
/// exact `expected` labels, each replacing the `typed` bytes before the
/// cursor; `None` when it does not. `last_good` is served first; see
/// [`with_edited_document`].
fn operand_mismatch(
    case: &str,
    name: &str,
    last_good: Option<&str>,
    document: &str,
    expected: &[&str],
    typed: usize,
) -> Option<String> {
    let (_, offset) = at_cursor(document);
    let items = offered_after(name, last_good, document);
    let replaced = offset - typed..offset;
    (labels(&items) != expected || items.iter().any(|(_, range)| *range != replaced))
        .then(|| format!("{case}: expected {expected:?} replacing {replaced:?}, got {items:?}"))
}

/// One operand presentation: its case name, the `event` value of
/// [`library`] holding the cursor, the text that followed the cursor in the
/// last-good buffer when this one does not parse, the exact labels expected,
/// and the bytes the edit replaces.
type OperandCase = (&'static str, &'static str, &'static str, &'static [&'static str], usize);

/// Every scalar presentation a composition operand may continue through:
/// spreads inside a wrapped definition, scalar `$extends` values, and
/// sequence items.
const CONTINUED_OPERANDS: [OperandCase; 17] = [
    ("closed single quote", "'{ at: string,\n    ...b<|> }'", "", &["base"], 1),
    ("closed double quote", "\"{ at: string,\n    ...b<|> }\"", "", &["base"], 1),
    ("open single quote", "'{ at: string,\n    ...b<|>", "'", &["base"], 1),
    ("open double quote", "\"{ at: string,\n    ...b<|>", "\"", &["base"], 1),
    ("folded", ">-\n    { at: string,\n      ...b<|> }", "", &["base"], 1),
    ("literal", "|-\n    { at: string,\n      ...b<|> }", "", &["base"], 1),
    ("$extends plain", "\n    $extends: b<|>", "", &["base"], 1),
    ("$extends single quote", "\n    $extends: 'b<|>'", "", &["base"], 1),
    ("$extends double quote", "\n    $extends: \"b<|>\"", "", &["base"], 1),
    ("$extends folded", "\n    $extends: >-\n      b<|>", "", &["base"], 1),
    ("$extends literal", "\n    $extends: |-\n      b<|>", "", &["base"], 1),
    ("$extends wrapped single quote", "\n    $extends: 'base@\n      th<|>'", "", &["this"], 2),
    ("$extends wrapped double quote", "\n    $extends: \"base@\n      th<|>\"", "", &["this"], 2),
    ("flow item wrapped quote", "\n    $extends: ['base@\n      th<|>']", "", &["this"], 2),
    ("block item wrapped quote", "\n    $extends:\n      - 'base@\n        th<|>'", "", &["this"], 2),
    ("flow item continued", "\n    $extends: [base,\n      b<|>]", "", &["base"], 1),
    ("block item", "\n    $extends:\n      - b<|>", "", &["base"], 1),
];

/// Presentations beyond [`CONTINUED_OPERANDS`] that the same recovery
/// reaches: references in a quote still open at the cursor, the other
/// chomping and indentation indicators, block scalars as sequence items,
/// wrapped plain references, and the first content line of a block scalar.
const SWEPT_OPERANDS: [OperandCase; 10] = [
    ("$extends open single quote", "\n    $extends: 'base@\n      th<|>", "'", &["this"], 2),
    ("$extends open double quote", "\n    $extends: \"base@\n      th<|>", "\"", &["this"], 2),
    ("block item open quote", "\n    $extends:\n      - 'base@\n        th<|>", "'", &["this"], 2),
    ("$extends wrapped plain", "\n    $extends: base@\n      th<|>", "", &["this"], 2),
    ("folded keep", ">+\n    { at: string,\n      ...b<|> }\n", "", &["base"], 1),
    ("folded clip", ">\n    { at: string,\n      ...b<|> }", "", &["base"], 1),
    ("literal with an indentation indicator", "|2-\n    { at: string,\n      ...b<|> }", "", &["base"], 1),
    ("first block-scalar line", ">-\n    { ...b<|> }", "", &["base"], 1),
    ("block item folded", "\n    $extends:\n      - >-\n        b<|>", "", &["base"], 1),
    ("$extends literal keep", "\n    $extends: |+\n      b<|>\n", "", &["base"], 1),
];

#[test]
fn composition_operands_in_further_scalar_presentations() {
    let mismatches: Vec<String> =
        SWEPT_OPERANDS.into_iter().flat_map(|case| operand_case_mismatches(case, "\n")).collect();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

/// An operand not yet typed on a continuation line offers every eligible
/// object, or `this` after `name@`. Frontmatter reaches these buffers only
/// over a last-good tree: an open quote does not parse, and neither does a
/// block scalar whose only line is indentation.
#[test]
fn composition_empty_operand_on_a_continuation_line_offers_only_eligible_objects() {
    // Each `event` value, what the last-good buffer held at the cursor, and
    // whether the operand is a reference half.
    for (event, last_good, reference) in [
        ("\n    $extends: >-\n      <|>", "base", false),
        ("\n    $extends: 'base@\n      <|>", "this'", true),
        (">-\n    { at: string,\n      ...<|> }", "", false),
        ("'{ at: string,\n    ...<|>", "base'", false),
    ] {
        let document = library(event);
        let (_, offset) = at_cursor(&document);
        let items = offered("lib.yaml", &document);
        if reference {
            assert_offered(&items, &["this"], offset..offset);
        } else {
            assert_local_offered(&items, &ELIGIBLE_LOCAL, offset..offset);
        }

        let frontmatter = in_frontmatter(&document);
        let (_, offset) = at_cursor(&frontmatter);
        let last_good = (!last_good.is_empty()).then(|| frontmatter.replace(CURSOR, last_good));
        let items = offered_after("doc.md", last_good.as_deref(), &frontmatter);
        assert_offered(&items, if reference { &["this"] } else { &ELIGIBLE }, offset..offset);
    }
}

/// [`operand_mismatch`] for `case` served standalone (never from a last-good
/// model) and as frontmatter, which a buffer that does not parse reaches
/// only over its last-good tree.
fn operand_case_mismatches((case, event, closing, expected, typed): OperandCase, newline: &str) -> Vec<String> {
    let document = library(event);
    let frontmatter = in_frontmatter(&document);
    let [document, frontmatter] = [document, frontmatter].map(|text| text.replace('\n', newline));
    let last_good = (!closing.is_empty()).then(|| frontmatter.replace(CURSOR, closing).replace(CURSOR, ""));
    let mut mismatches = Vec::new();
    mismatches.extend(operand_mismatch(&format!("standalone {case}"), "lib.yaml", None, &document, expected, typed));
    mismatches.extend(operand_mismatch(
        &format!("frontmatter {case}"),
        "doc.md",
        last_good.as_deref(),
        &frontmatter,
        expected,
        typed,
    ));
    mismatches
}

#[test]
fn composition_operands_on_scalar_continuation_lines() {
    let mismatches: Vec<String> =
        CONTINUED_OPERANDS.into_iter().flat_map(|case| operand_case_mismatches(case, "\n")).collect();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn composition_operands_on_crlf_continuation_lines() {
    let mismatches: Vec<String> = CONTINUED_OPERANDS
        .into_iter()
        .filter(|(case, ..)| matches!(*case, "open single quote" | "folded" | "$extends plain" | "$extends literal" | "block item wrapped quote"))
        .flat_map(|case| operand_case_mismatches(case, "\r\n"))
        .collect();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

/// Hover and go-to-definition on an operand name authored on a continuation
/// line of each block-scalar and wrapped presentation reach `base`.
#[test]
fn composition_operand_navigation_on_continuation_lines() {
    let types = "  base: \"{ id: string }\"\n  folded: >-\n    { at: string,\n      ...base }\n  literal: |\n    { at: string,\n      ...base }\n  \
                 scalar:\n    $extends: >-\n      base\n  wrapped:\n    $extends: 'audit@./audit.yaml\n      ' \n  \
                 listed:\n    $extends:\n      - \"base\n        \"\n";
    for (name, text) in [
        ("lib.yaml", format!("kind: schema\ntypes:\n{types}")),
        ("doc.md", format!("---\n$schema:\n{types}---\n\nBody\n")),
    ] {
        with_document(&[("audit.yaml", AUDIT)], name, &text, |ctx| {
            let base_key = ctx.source_map.byte_range_to_lsp(span_of(&text, "base", 0)).unwrap();
            for occurrence in 1..=4 {
                let offset = span_of(&text, "base", occurrence).start + 1;
                let markup = hover_markup(ctx, offset);
                assert!(markup.contains("Defined in: this schema"), "{name} occurrence {occurrence}: {markup}");
                let locations = definition(ctx, offset);
                assert_eq!(locations.len(), 1, "{name} occurrence {occurrence}: {locations:?}");
                assert_eq!(locations[0].range, base_key, "{name} occurrence {occurrence}");
            }
            let offset = span_of(&text, "audit@", 0).start + 1;
            assert!(hover_markup(ctx, offset).contains("Defined in: `./audit.yaml`"), "{name}");
            assert_eq!(definition(ctx, offset).len(), 1, "{name}");
        });
    }
}

/// An ineligible operand authored on a continuation line of a block scalar
/// or a wrapped quote is reported on that operand.
#[test]
fn composition_operand_diagnostics_on_continuation_lines() {
    for value in [">-\n    { at: string,\n      ...label }", "|-\n    { at: string,\n      ...label }", "\n    $extends: >-\n      label", "\n    $extends:\n      - 'label\n        '"] {
        let types = format!("  base: \"{{ id: string }}\"\n  label: string\n  event: {value}\n");
        for (name, text) in [
            ("lib.yaml", format!("kind: schema\ntypes:\n{types}")),
            ("doc.md", format!("---\n$schema:\n{types}---\n\nBody\n")),
        ] {
            with_document(&[], name, &text, |ctx| {
                let label = ctx.source_map.byte_range_to_lsp(span_of(&text, "label", 1)).unwrap();
                let found = diagnostics(ctx);
                let composed = |diagnostic: &Diagnostic| {
                    diagnostic.range == label && diagnostic.message.contains("`label` cannot be composed into an object")
                };
                assert!(found.iter().any(composed), "{name} {value:?}: {found:#?}");
            });
        }
    }
}

/// Continuation recovery stays inside the scalar and its directive: data,
/// a `types` entry named `$extends`, text past a closing quote, and a
/// sibling after a block scalar keep their own completion.
#[test]
fn composition_continuation_recovery_keeps_other_values_ordinary() {
    // Frontmatter data, not a declaration.
    let document = "---\n$schema:\n  base: \"{ id: string }\"\n  note: string\n$extends: >-\n  ba<|>\nnote: >-\n  ba\n---\n\nBody\n";
    let items = offered("doc.md", document);
    assert!(labels(&items).iter().all(|label| *label != "base"), "{items:#?}");
    let document = "---\n$schema:\n  base: \"{ id: string }\"\n  note: string\nnote: >-\n  ba<|>\n---\n\nBody\n";
    let items = offered("doc.md", document);
    assert!(labels(&items).iter().all(|label| *label != "base"), "{items:#?}");

    // A `types` entry named `$extends` is a type definition, wrapped or not.
    let document = "kind: schema\ntypes:\n  base: \"{ id: string }\"\n  $extends: >-\n    str<|>\n";
    let items = offered("lib.yaml", document);
    assert!(labels(&items).contains(&"string"), "{items:#?}");

    // Past the closing quote of a wrapped reference.
    let document = library("\n    $extends: 'base@\n      this' <|>");
    let items = offered("lib.yaml", &document);
    assert!(labels(&items).iter().all(|label| *label != "this"), "{items:#?}");

    // A sibling key after a block scalar is a property, not an operand.
    let document = library("\n    $extends: >-\n      base\n    price: <|>");
    let items = offered("lib.yaml", &document);
    assert!(labels(&items).contains(&"number"), "{items:#?}");
}

/// Spreads after an explicit property on a scalar continuation line, where
/// the property's `key:` must not be read as a YAML mapping entry: rows of
/// [`operand_case_mismatches`] in every presentation, with the property on
/// the cursor's line or an earlier one, after a `#` line that is the
/// scalar's text, and one-line controls.
const EXPLICIT_PROPERTY_OPERANDS: [OperandCase; 18] = [
    ("closed single quote", "'{ at: string,\n    next: number, ...b<|> }'", "", &["base"], 1),
    ("closed double quote", "\"{ at: string,\n    next: number, ...b<|> }\"", "", &["base"], 1),
    ("open single quote", "'{ at: string,\n    next: number, ...b<|>", "'", &["base"], 1),
    ("open double quote", "\"{ at: string,\n    next: number, ...b<|>", "\"", &["base"], 1),
    ("folded", ">-\n    { at: string,\n      next: number, ...b<|> }", "", &["base"], 1),
    ("literal", "|-\n    { at: string,\n      next: number, ...b<|> }", "", &["base"], 1),
    ("folded, property on an earlier line", ">-\n    { at: string,\n      next: number,\n        ...b<|> }", "", &["base"], 1),
    ("closed single quote, property on an earlier line", "'{ at: string,\n    next: number,\n      ...b<|> }'", "", &["base"], 1),
    ("folded array item", ">-\n    { at: string,\n      next: number, ...b<|> }[]", "", &["base"], 1),
    ("folded union arm", ">-\n    { at: string,\n      next: number, ...b<|> } | string", "", &["base"], 1),
    ("folded reference half", ">-\n    { at: string,\n      next: number, ...base@th<|> }", "", &["this"], 2),
    ("one-line single quote", "'{ at: string, next: number, ...b<|> }'", "", &["base"], 1),
    ("one-line double quote", "\"{ at: string, next: number, ...b<|> }\"", "", &["base"], 1),
    ("first folded line", ">-\n    { at: string, next: number, ...b<|> }", "", &["base"], 1),
    ("first literal line", "|-\n    { at: string, next: number, ...b<|> }", "", &["base"], 1),
    ("folded nested object", ">-\n    { nested: { at: string,\n      next: number, ...b<|> } }", "", &["base"], 1),
    ("folded, `#` line before the property", ">-\n    { at: string,\n      # text, not a comment\n      next: number, ...b<|> }", "", &["base"], 1),
    ("closed double quote, `#` line before the property", "\"{ at: string,\n    # text, not a comment\n    next: number, ...b<|> }\"", "", &["base"], 1),
];

#[test]
fn composition_operands_after_an_explicit_property_on_a_continuation_line() {
    let mismatches: Vec<String> =
        EXPLICIT_PROPERTY_OPERANDS.into_iter().flat_map(|case| operand_case_mismatches(case, "\n")).collect();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn composition_operands_after_an_explicit_property_on_crlf_continuation_lines() {
    let mismatches: Vec<String> = EXPLICIT_PROPERTY_OPERANDS
        .into_iter()
        .filter(|(case, ..)| matches!(*case, "open single quote" | "folded" | "folded, property on an earlier line"))
        .flat_map(|case| operand_case_mismatches(case, "\r\n"))
        .collect();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

/// The receiving object's owner decides scope after an explicit property on
/// a folded continuation line: a value typed `type-definition`, an object
/// inside a value typed `schema`, a declaration inside a value typed
/// `schema(mapping)`, and a root-union arm, whose names stay arm-local.
#[test]
fn composition_owner_after_an_explicit_property_on_a_continuation_line() {
    let folded = ">-\n    { at: string,\n      next: number, ...b<|> }";
    let mismatches = [
        (
            "type-definition value",
            "---\n$schema:\n  shape: type-definition\n  base: \"{ id: string }\"\nshape: >-\n  { at: string,\n    next: number, ...b<|> }\n---\n\nBody\n".to_string(),
        ),
        (
            "object in a schema value",
            format!("---\n$schema:\n  shape: schema\n  base: \"{{ id: string }}\"\nshape:\n  event: {folded}\n---\n\nBody\n"),
        ),
        (
            "declaration in a schema(mapping) value",
            format!("---\n$schema:\n  shape: schema(mapping)\n  base: \"{{ id: string }}\"\nshape:\n  event: {folded}\n---\n\nBody\n"),
        ),
        (
            "root-union arm",
            "---\n$schema:\n  - base: \"{ id: string }\"\n    event: >-\n      { at: string,\n        next: number, ...b<|> }\n  - boxed: \"{ r: string }\"\n---\n\nBody\n".to_string(),
        ),
    ]
    .into_iter()
    .filter_map(|(case, document)| operand_mismatch(case, "doc.md", None, &document, &["base"], 1))
    .collect::<Vec<_>>();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

/// The four closed presentations of a definition holding `...{operand}`
/// after an explicit property on its continuation line.
fn after_an_explicit_property(operand: &str) -> [String; 4] {
    [
        format!("'{{ at: string,\n    next: number, ...{operand} }}'"),
        format!("\"{{ at: string,\n    next: number, ...{operand} }}\""),
        format!(">-\n    {{ at: string,\n      next: number, ...{operand} }}"),
        format!("|-\n    {{ at: string,\n      next: number, ...{operand} }}"),
    ]
}

/// Hover and go-to-definition on an operand after an explicit property on a
/// continuation line reach `base`, standalone and in frontmatter.
#[test]
fn composition_operand_navigation_after_an_explicit_property() {
    for value in after_an_explicit_property("base") {
        let types = format!("  base: \"{{ id: string }}\"\n  event: {value}\n");
        for (name, text) in [
            ("lib.yaml", format!("kind: schema\ntypes:\n{types}")),
            ("doc.md", format!("---\n$schema:\n{types}---\n\nBody\n")),
        ] {
            with_document(&[], name, &text, |ctx| {
                let base_key = ctx.source_map.byte_range_to_lsp(span_of(&text, "base", 0)).unwrap();
                let offset = span_of(&text, "base", 1).start + 1;
                let markup = hover_markup(ctx, offset);
                assert!(markup.contains("Defined in: this schema"), "{name} {value:?}: {markup}");
                let locations = definition(ctx, offset);
                assert_eq!(locations.len(), 1, "{name} {value:?}: {locations:?}");
                assert_eq!(locations[0].range, base_key, "{name} {value:?}");
            });
        }
    }
}

/// An ineligible operand after an explicit property on a continuation line
/// is reported on exactly that operand, standalone and in frontmatter.
#[test]
fn composition_operand_diagnostics_after_an_explicit_property() {
    for value in after_an_explicit_property("label") {
        let types = format!("  base: \"{{ id: string }}\"\n  label: string\n  event: {value}\n");
        for (name, text) in [
            ("lib.yaml", format!("kind: schema\ntypes:\n{types}")),
            ("doc.md", format!("---\n$schema:\n{types}---\n\nBody\n")),
        ] {
            with_document(&[], name, &text, |ctx| {
                let label = ctx.source_map.byte_range_to_lsp(span_of(&text, "label", 1)).unwrap();
                let found = diagnostics(ctx);
                let composed = |diagnostic: &Diagnostic| {
                    diagnostic.range == label && diagnostic.message.contains("`label` cannot be composed into an object")
                };
                assert!(found.iter().any(composed), "{name} {value:?}: {found:#?}");
            });
        }
    }
}

/// `$extends` operands after a sibling property whose block or wrapped
/// scalar holds a `key:`-looking line: the directive is still its own key.
const EXTENDS_AFTER_A_PROPERTY: [OperandCase; 4] = [
    ("$extends after a folded property", "\n    note: >-\n      next: number\n    $extends: b<|>", "", &["base"], 1),
    ("$extends folded after a quoted property", "\n    note: 'a,\n      next: number'\n    $extends: >-\n      b<|>", "", &["base"], 1),
    ("$extends wrapped quote after a literal property", "\n    note: |-\n      next: number\n    $extends: 'base@\n      th<|>'", "", &["this"], 2),
    ("$extends block item after a folded property", "\n    note: >-\n      next: number\n    $extends:\n      - b<|>", "", &["base"], 1),
];

#[test]
fn composition_extends_after_a_property_holding_a_key_like_line() {
    let mismatches: Vec<String> =
        EXTENDS_AFTER_A_PROPERTY.into_iter().flat_map(|case| operand_case_mismatches(case, "\n")).collect();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

/// A `key:` line that a scalar does not reach stays its own node: a real
/// sibling after a block scalar ends, after a closed quote, and a sequence
/// sibling after an empty holder; data values keep their completion.
#[test]
fn composition_explicit_property_recovery_keeps_real_siblings_ordinary() {
    // A real sibling property after the folded definition ends.
    for document in [
        "---\n$schema:\n  base: \"{ id: string }\"\n  event: >-\n    { at: string,\n      next: number }\n  next: <|>\n---\n\nBody\n".to_string(),
        "---\n$schema:\n  base: \"{ id: string }\"\n  event: '{ at: string,\n    next: number }'\n  next: <|>\n---\n\nBody\n".to_string(),
        library(">-\n    { at: string,\n      next: number }\n  next: <|>"),
    ] {
        let items = offered(if document.starts_with("---") { "doc.md" } else { "lib.yaml" }, &document);
        assert!(labels(&items).contains(&"number"), "{document:?}: {items:#?}");
    }

    // A sequence after an empty holder is not a plain scalar's content.
    let document = "---\n$schema:\n  base: \"{ id: string }\"\n  event:\n    $extends:\n      - base\n    next: b<|>\n---\n\nBody\n";
    let items = offered("doc.md", document);
    assert!(labels(&items).contains(&"boolean"), "{items:#?}");

    // Frontmatter data whose folded value holds `next: number, ...b`.
    let document = "---\n$schema:\n  base: \"{ id: string }\"\n  note: string\nnote: >-\n  { at: string,\n    next: number, ...b<|> }\n---\n\nBody\n";
    let items = offered("doc.md", document);
    assert!(labels(&items).iter().all(|label| *label != "base"), "{items:#?}");

    // A `types` entry named `$extends`, a type definition, holding a property line.
    let document = "kind: schema\ntypes:\n  base: \"{ id: string }\"\n  $extends: >-\n    { at: string,\n      next: str<|> }\n";
    let items = offered("lib.yaml", document);
    assert!(labels(&items).contains(&"string"), "{items:#?}");

    // Text past the closing quote after an explicit property.
    let document = library("'{ at: string,\n    next: number, ...base }' <|>");
    let items = offered("lib.yaml", &document);
    assert!(labels(&items).iter().all(|label| *label != "base"), "{items:#?}");
}
/// Ordinary frontmatter completion shares the scalar-ownership reading: a
/// `key:` or `- ` line inside a folded or quoted data value is that value's
/// text, while the same key or item authored as a real entry still completes.
#[test]
fn key_like_line_on_a_data_scalar_continuation_offers_no_value_completion() {
    let schema = "---\n$schema:\n  outer: \"{ next: boolean, note: string, flags: boolean[] }\"\nouter:\n";
    for (case, value, offered_true) in [
        ("folded", "  note: >-\n    next: tr<|>\n", false),
        ("single quote", "  note: 'a,\n    next: tr<|>'\n", false),
        ("double quote", "  note: \"a,\n    next: tr<|>\"\n", false),
        ("sibling after the folded value", "  note: >-\n    next: no\n  next: tr<|>\n", true),
        ("real entry", "  next: tr<|>\n", true),
        ("item-like line in a folded value", "  flags: >-\n    - tr<|>\n", false),
        ("real item", "  flags:\n    - tr<|>\n", true),
    ] {
        let document = format!("{schema}{value}---\n\nBody\n");
        let items = offered("doc.md", &document);
        assert_eq!(labels(&items).contains(&"true"), offered_true, "{case}: {items:#?}");
    }
}
