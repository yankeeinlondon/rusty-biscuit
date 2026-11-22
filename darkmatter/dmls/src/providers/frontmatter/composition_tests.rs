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
    let overlay = OverlayState::default().for_document(&uri, text, &path, &config, &roots, &resolution);
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
    let (text, offset) = at_cursor(document);
    with_document(&[("audit.yaml", AUDIT)], name, &text, |ctx| {
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
