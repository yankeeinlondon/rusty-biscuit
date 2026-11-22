//! Object composition (`...Name` in an inline object, `$extends` in a YAML
//! schema mapping) through an in-memory LSP session: operand completion while
//! the operand is still being typed, in both forms and in quoted, flow, and
//! block sequence items, with ineligible targets filtered and implicit
//! root-union arms kept apart; hover and go-to-definition on operands; a
//! diagnostic ranged on the one failing operand when two name the same file;
//! and refresh of the importer when the imported definition changes in an
//! unsaved buffer or a missing imported file appears.
//!
//! No real editor or terminal is involved.

use crate::common;

use std::path::Path;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

use crate::union_item_editor::{Document, initialize_params, open_text, position, request};

fn markdown(frontmatter: &str) -> String {
    format!("---\n{frontmatter}---\n\nBody\n")
}

fn session(workspace: &LspWorkspace) -> LspFixture<'_> {
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(workspace.path()));
    fixture
}

/// A session whose client watches files, so a created file is reported.
fn watched_session(workspace: &LspWorkspace) -> LspFixture<'_> {
    let mut params = initialize_params(workspace.path());
    params["capabilities"]["workspace"] = json!({ "didChangeWatchedFiles": { "dynamicRegistration": true } });
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(params);
    fixture
}

fn write(workspace: &LspWorkspace, name: &str, text: &str) {
    let path = workspace.path().join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

/// The URI the server reports for a file it resolved itself: the resolver
/// identifies schema files by canonical path.
fn canonical_uri(path: &Path) -> String {
    let canonical = biscuit_file::canonicalize_simplified(path).unwrap();
    url::Url::from_file_path(canonical).unwrap().to_string()
}

/// The byte offset of the `occurrence`-th (0-based) `needle`, plus `skip`.
fn offset(text: &str, needle: &str, occurrence: usize, skip: usize) -> usize {
    text.match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("`{needle}` #{occurrence} in {text:?}"))
        .0
        + skip
}

fn range(text: &str, start: usize, end: usize) -> Value {
    json!({ "start": position(text, start), "end": position(text, end) })
}

/// The LSP range of the `occurrence`-th `needle` in `text`.
fn range_of(text: &str, needle: &str, occurrence: usize) -> Value {
    let start = offset(text, needle, occurrence, 0);
    range(text, start, start + needle.len())
}

/// Every completion item at `offset`, as the server sent it.
fn completion_items(fixture: &mut LspFixture<'_>, document: &Document, offset: usize) -> Vec<Value> {
    let items = request(fixture, "textDocument/completion", document, offset);
    items
        .as_array()
        .cloned()
        .unwrap_or_else(|| items["items"].as_array().cloned().unwrap_or_default())
}

fn sorted_labels(items: &[Value]) -> Vec<String> {
    let mut labels: Vec<String> = items.iter().map(|item| item["label"].as_str().unwrap().to_string()).collect();
    labels.sort_unstable();
    labels
}

fn item<'a>(items: &'a [Value], label: &str) -> &'a Value {
    items
        .iter()
        .find(|item| item["label"] == json!(label))
        .unwrap_or_else(|| panic!("no `{label}` in {items:#?}"))
}

fn hover_at(fixture: &mut LspFixture<'_>, document: &Document, offset: usize) -> String {
    let hover = request(fixture, "textDocument/hover", document, offset);
    hover["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("no hover at {offset}: {hover:?}"))
        .to_string()
}

fn type_definition_errors(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_type_definition"))
        .collect()
}

fn related(diagnostic: &Value) -> Vec<(Value, String)> {
    diagnostic["relatedInformation"]
        .as_array()
        .map(|related| {
            related
                .iter()
                .map(|entry| (entry["location"].clone(), entry["message"].as_str().unwrap_or_default().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn edit(fixture: &LspFixture<'_>, uri: &str, version: i32, text: &str) {
    fixture.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": uri, "version": version }, "contentChanges": [{ "text": text }] }),
    );
}

/// The diagnostics last published for `uri` once the server is idle; the
/// earlier publications still buffered are dropped.
fn latest_diagnostics(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    fixture.flush_server();
    let latest = fixture.wait_for_diagnostics_params(uri);
    while fixture.take_buffered_diagnostics(uri).is_some() {}
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

/// A frontmatter schema with one eligible object of each lookup kind and one
/// ineligible definition of each kind completion can recognize passively.
const NAMES: &str = concat!(
    "  events: \"{ id: string(required), price: number }\"\n",
    "  empty: \"{}\"\n",
    "  alias: events\n",
    "  string: \"{ text: string }\"\n",
    "  label: string\n",
    "  shape: \"{ a: string } | { b: number }\"\n",
    "  rows: \"events[]\"\n",
    "  meta: object\n",
    "  loop_a: loop_b\n",
    "  loop_b: loop_a\n",
);

/// The operand names a frontmatter schema built on [`NAMES`] offers: every
/// eligible object, a keyword-colliding name spelled with `@this`, and the
/// alias, but neither the receiver nor an ineligible definition.
const OFFERED: [&str; 4] = ["alias", "empty", "events", "string@this"];

/// Completion after `...` and in a `$extends` scalar, flow item, and block
/// item offers the same eligible names, replacing the operand typed so far,
/// and never a type keyword, scaffold, or constraint.
#[test]
fn operand_completion_offers_eligible_objects_in_both_forms() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    // (case, receiver declaration, text just before the cursor, typed operand)
    let cases: [(&str, &str, &str, &str); 6] = [
        ("empty spread", "  order: \"{ total: number, ... }\"\n", "...", ""),
        ("partial spread", "  order: \"{ ...ev }\"\n", "...ev", "ev"),
        ("scalar $extends", "  order:\n    $extends: ev\n    total: number\n", "$extends: ev", "ev"),
        ("quoted $extends", "  order:\n    \"$extends\": 'ev'\n    total: number\n", "\"$extends\": 'ev", "ev"),
        ("flow item", "  order:\n    $extends: [events, al]\n    total: number\n", "[events, al", "al"),
        ("block item", "  order:\n    $extends:\n      - events\n      - al\n    total: number\n", "- al", "al"),
    ];
    for (case, receiver, needle, typed) in cases {
        let frontmatter = format!("$schema:\n{NAMES}{receiver}order: {{}}\n");
        let (document, _) = open_text(&mut fixture, &workspace, "operands.md", &markdown(&frontmatter));
        let at = offset(&document.text, needle, 0, needle.len());
        let items = completion_items(&mut fixture, &document, at);
        let expected: Vec<String> =
            OFFERED.iter().filter(|label| label.starts_with(typed)).map(|label| label.to_string()).collect();
        assert_eq!(sorted_labels(&items), expected, "{case}: {items:#?}");
        let first = item(&items, &expected[0]);
        assert_eq!(first["textEdit"]["range"], range(&document.text, at - typed.len(), at), "{case}: {first:#?}");
        assert_eq!(first["textEdit"]["newText"], json!(expected[0]), "{case}");
    }
    fixture.shutdown();
}

/// After `@`, completion replaces only the reference half; an imported
/// file's names are offered with their target, filtered by eligibility.
#[test]
fn operand_completion_offers_imported_names_and_the_reference_half() {
    let workspace = LspWorkspace::new();
    write(
        &workspace,
        "lib/audit.yaml",
        "kind: schema\ntypes:\n  audit: \"{ by: string(required) }\"\n  stamp: number\n  trail: \"audit[]\"\n",
    );
    let mut fixture = session(&workspace);
    let frontmatter = concat!(
        "$schema:\n",
        "  base: 'audit@./lib/audit.yaml'\n",
        "  order:\n",
        "    $extends: [base, au]\n",
        "    total: number\n",
        "order: {}\n",
    );
    let (document, _) = open_text(&mut fixture, &workspace, "imports.md", &markdown(frontmatter));
    let at = offset(&document.text, "[base, au", 0, "[base, au".len());
    let items = completion_items(&mut fixture, &document, at);
    assert_eq!(sorted_labels(&items), ["audit@./lib/audit.yaml"], "{items:#?}");
    let audit = item(&items, "audit@./lib/audit.yaml");
    assert_eq!(audit["textEdit"]["range"], range(&document.text, at - 2, at), "{audit:#?}");

    let spread = frontmatter.replace("    $extends: [base, au]\n", "    p: \"{ ...audit@./lib/a }\"\n");
    let (document, _) = open_text(&mut fixture, &workspace, "reference.md", &markdown(&spread));
    let at = offset(&document.text, "@./lib/a", 0, "@./lib/a".len());
    let items = completion_items(&mut fixture, &document, at);
    for item in &items {
        let edit = &item["textEdit"]["range"];
        assert_eq!(edit["start"], position(&document.text, at - "./lib/a".len()), "only the reference half: {item:#?}");
    }
    fixture.shutdown();
}

/// An implicit root union keeps each inline arm's names to itself, and the
/// names of a standalone document's `types` table are offered everywhere.
#[test]
fn operand_completion_keeps_implicit_root_union_arms_apart() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let frontmatter = concat!(
        "$schema:\n",
        "  - left_base: \"{ a: string }\"\n",
        "    left: \"{ ...le }\"\n",
        "  - right_base: \"{ b: number }\"\n",
        "    right: \"{ ...ri }\"\n",
        "left: {}\n",
    );
    let (document, _) = open_text(&mut fixture, &workspace, "arms.md", &markdown(frontmatter));
    let left = completion_items(&mut fixture, &document, offset(&document.text, "...le", 0, 5));
    assert_eq!(sorted_labels(&left), ["left_base"], "{left:#?}");
    let right = completion_items(&mut fixture, &document, offset(&document.text, "...ri", 0, 5));
    assert_eq!(sorted_labels(&right), ["right_base"], "{right:#?}");

    let standalone = concat!(
        "kind: schema\n",
        "$schema:\n",
        "  $extends: ev\n",
        "  source: string\n",
        "types:\n",
        "  events: \"{ id: string(required) }\"\n",
        "  label: string\n",
    );
    let (document, _) = open_text(&mut fixture, &workspace, "standalone.yaml", standalone);
    let at = offset(&document.text, "$extends: ev", 0, "$extends: ev".len());
    let items = completion_items(&mut fixture, &document, at);
    assert_eq!(sorted_labels(&items), ["events"], "{items:#?}");
    fixture.shutdown();
}

/// Hover and go-to-definition on an operand of either form describe and
/// reach the declaration the operand names, locally and in another file.
#[test]
fn hover_and_definition_follow_operands_of_both_forms() {
    let workspace = LspWorkspace::new();
    let audit = "kind: schema\ntypes:\n  audit: \"{ by: string(required) }\"\n";
    write(&workspace, "lib/audit.yaml", audit);
    let mut fixture = session(&workspace);
    let frontmatter = concat!(
        "$schema:\n",
        "  events: \"{ id: string(required) }\"\n",
        "  snapshot: \"{ ...events, price: number }\"\n",
        "  order:\n",
        "    $extends: [events, 'audit@./lib/audit.yaml']\n",
        "    total: number\n",
        "order:\n",
        "  id: a\n",
        "  by: b\n",
        "snapshot:\n",
        "  id: c\n",
    );
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "navigate.md", &markdown(frontmatter));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let events_key = range_of(&document.text, "events", 0);
    for (case, at) in [
        ("spread", offset(&document.text, "...events", 0, 4)),
        ("flow item", offset(&document.text, "[events", 0, 2)),
    ] {
        let markup = hover_at(&mut fixture, &document, at);
        assert!(markup.contains("Defined in: this schema"), "{case}: {markup}");
        let locations = request(&mut fixture, "textDocument/definition", &document, at);
        assert_eq!(locations, json!([{ "uri": document.uri, "range": events_key }]), "{case}");
    }
    let at = offset(&document.text, "audit@", 0, 1);
    let markup = hover_at(&mut fixture, &document, at);
    assert!(markup.contains("`./lib/audit.yaml`"), "{markup}");
    let locations = request(&mut fixture, "textDocument/definition", &document, at);
    assert_eq!(
        locations,
        json!([{ "uri": canonical_uri(&workspace.path().join("lib/audit.yaml")), "range": range_of(audit, "audit", 0) }])
    );
    fixture.shutdown();
}

/// Two operands name the same file and only the second target cannot be
/// composed: the diagnostic is ranged on the second operand, with the
/// target's declaration as related information.
#[test]
fn an_ineligible_second_operand_into_a_shared_file_is_reported_on_that_operand() {
    let workspace = LspWorkspace::new();
    let types = "kind: schema\ntypes:\n  base: \"{ id: string }\"\n  rows: \"base[]\"\n";
    write(&workspace, "types.yaml", types);
    let mut fixture = session(&workspace);
    let frontmatter = concat!(
        "$schema:\n",
        "  order:\n",
        "    $extends: ['base@./types.yaml', 'rows@./types.yaml']\n",
        "    total: number\n",
        "order: {}\n",
    );
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "shared.md", &markdown(frontmatter));
    let errors = type_definition_errors(&diagnostics);
    assert_eq!(errors.len(), 1, "{diagnostics:#?}");
    let message = errors[0]["message"].as_str().unwrap();
    assert!(message.contains("rows@./types.yaml") && message.contains("list"), "{message}");
    assert_eq!(errors[0]["range"], range_of(&document.text, "rows@./types.yaml", 0), "{:#?}", errors[0]);
    let target = (
        json!({ "uri": canonical_uri(&workspace.path().join("types.yaml")), "range": range_of(types, "\"base[]\"", 0) }),
        String::new(),
    );
    assert!(
        related(errors[0]).iter().any(|(location, _)| *location == target.0),
        "the target declaration is related: {:#?}",
        errors[0]
    );
    fixture.shutdown();
}

/// The importer and the imported file are both open with their disk text.
/// Editing the imported definition in its buffer to a list, and then back to
/// an object with a renamed and a newly required member, reports and clears
/// the composition error, judges the consuming document by the new members,
/// and offers the new keys there with the old ones absent.
#[test]
fn an_unsaved_imported_definition_edit_refreshes_the_importer() {
    let workspace = LspWorkspace::new();
    let saved = "kind: schema\ntypes:\n  base: \"{ id: string, price: number }\"\n";
    write(&workspace, "types.yaml", saved);
    let mut fixture = session(&workspace);
    let types_path = workspace.path().join("types.yaml");
    let types_uri = uri(&types_path);
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": types_uri, "languageId": "yaml", "version": 1, "text": saved } }),
    );
    let frontmatter = concat!(
        "$schema:\n",
        "  order:\n",
        "    $extends: 'base@./types.yaml'\n",
        "    total: number\n",
        "order:\n",
        "  id: a\n",
        "  \n",
    );
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "order.md", &markdown(frontmatter));
    assert!(diagnostics.is_empty(), "saved control: {diagnostics:#?}");
    let key_line = offset(&document.text, "  id: a\n  ", 0, "  id: a\n  ".len());
    let keys = sorted_labels(&completion_items(&mut fixture, &document, key_line));
    assert!(keys.contains(&"price".to_string()) && !keys.contains(&"code".to_string()), "{keys:?}");

    edit(&fixture, &types_uri, 2, "kind: schema\ntypes:\n  base: \"{ id: string }[]\"\n");
    let diagnostics = latest_diagnostics(&mut fixture, &document.uri);
    let errors = type_definition_errors(&diagnostics);
    assert_eq!(errors.len(), 1, "the unsaved list is not composable: {diagnostics:#?}");
    assert_eq!(errors[0]["range"], range_of(&document.text, "base@./types.yaml", 0), "{:#?}", errors[0]);

    let renamed = "kind: schema\ntypes:\n  base: \"{ id: number, code: string(required) }\"\n";
    edit(&fixture, &types_uri, 3, renamed);
    let diagnostics = latest_diagnostics(&mut fixture, &document.uri);
    assert!(type_definition_errors(&diagnostics).is_empty(), "the error clears: {diagnostics:#?}");
    // `id: a` is now a type mismatch and `code` is missing, both judged in
    // the consuming document.
    let codes: Vec<&str> = diagnostics.iter().filter_map(|diagnostic| diagnostic["code"].as_str()).collect();
    assert!(codes.contains(&"dm.schema.type_mismatch"), "{diagnostics:#?}");
    let mismatch = diagnostics.iter().find(|d| d["code"] == json!("dm.schema.type_mismatch")).unwrap();
    let value = offset(&document.text, "id: a", 0, "id: ".len());
    assert_eq!(mismatch["range"], range(&document.text, value, value + 1), "{mismatch:#?}");
    let keys = sorted_labels(&completion_items(&mut fixture, &document, key_line));
    assert!(keys.contains(&"code".to_string()), "the new key is offered: {keys:?}");
    assert!(!keys.contains(&"price".to_string()), "the old key is gone: {keys:?}");

    fixture.shutdown();
}

/// A composition operand into a file that does not exist yet is reported on
/// the operand, and creating the file clears it through the watched-file
/// path.
#[test]
fn a_missing_imported_file_appearing_clears_the_operand_error() {
    let workspace = LspWorkspace::new();
    let mut fixture = watched_session(&workspace);
    let frontmatter = concat!(
        "$schema:\n",
        "  order: \"{ ...base@./later.yaml, total: number }\"\n",
        "order:\n",
        "  total: 1\n",
    );
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "later.md", &markdown(frontmatter));
    let errors = type_definition_errors(&diagnostics);
    assert_eq!(errors.len(), 1, "{diagnostics:#?}");
    assert_eq!(errors[0]["range"], range_of(&document.text, "base@./later.yaml", 0), "{:#?}", errors[0]);

    let later = workspace.path().join("later.yaml");
    std::fs::write(&later, "kind: schema\ntypes:\n  base: \"{ id: string }\"\n").unwrap();
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": uri(&later), "type": 1 }] }),
    );
    let diagnostics = latest_diagnostics(&mut fixture, &document.uri);
    assert!(diagnostics.is_empty(), "the created file resolves the operand: {diagnostics:#?}");
    fixture.shutdown();
}
