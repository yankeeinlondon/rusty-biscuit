//! Named-type references in DMLS: bare local names, `Name@this`, and
//! `Name@./file.yaml`, through completion, hover, go-to-definition, resolution
//! diagnostics, and dependency refresh, over the in-memory LSP harness.

use crate::common;

use std::path::{Path, PathBuf};

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

fn initialize_params(root: &Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "clientInfo": { "name": "Named Types", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": [{ "uri": root_uri.as_str(), "name": "scratch" }]
    })
}

/// A workspace file with its URI as the client opens it.
struct File {
    path: PathBuf,
    uri: String,
    text: String,
}

fn write(workspace: &LspWorkspace, name: &str, text: &str) -> File {
    let path = workspace.path().join(name);
    std::fs::write(&path, text).unwrap();
    File {
        uri: url::Url::from_file_path(&path).unwrap().to_string(),
        path,
        text: text.to_string(),
    }
}

/// The URI the server reports for a file it resolved itself: the resolver
/// identifies schema files by canonical path.
fn canonical_uri(path: &Path) -> String {
    let canonical = biscuit_file::canonicalize_simplified(path).unwrap();
    url::Url::from_file_path(canonical).unwrap().to_string()
}

fn open(fixture: &LspFixture<'_>, file: &File) {
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": file.uri,
                "languageId": "markdown",
                "version": 1,
                "text": file.text
            }
        }),
    );
}

/// The `(line, character)` of byte `offset` in ASCII `text`.
fn position(text: &str, offset: usize) -> (u32, u32) {
    let before = &text[..offset];
    let line = before.matches('\n').count();
    let character = offset - before.rfind('\n').map_or(0, |index| index + 1);
    (line as u32, character as u32)
}

/// The LSP range of the `occurrence`-th (0-based) `needle` in `text`.
fn range_of(text: &str, needle: &str, occurrence: usize) -> Value {
    let start = text
        .match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("`{needle}` #{occurrence} in {text:?}"))
        .0;
    let (start_line, start_character) = position(text, start);
    let (end_line, end_character) = position(text, start + needle.len());
    json!({
        "start": { "line": start_line, "character": start_character },
        "end": { "line": end_line, "character": end_character }
    })
}

fn request_at(fixture: &mut LspFixture<'_>, method: &str, file: &File, offset: usize) -> Value {
    let (line, character) = position(&file.text, offset);
    fixture
        .request(
            method,
            json!({
                "textDocument": { "uri": file.uri },
                "position": { "line": line, "character": character }
            }),
        )
        .result
        .unwrap_or(Value::Null)
}

fn completion_items(fixture: &mut LspFixture<'_>, file: &File, offset: usize) -> Vec<Value> {
    request_at(fixture, "textDocument/completion", file, offset)
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// Labels of `REFERENCE` (18) completion items: the named-type candidates.
fn reference_labels(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter(|item| item["kind"] == json!(18))
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect()
}

fn named_type_diagnostics(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_type_definition"))
        .collect()
}

fn offset_of(text: &str, needle: &str) -> usize {
    text.find(needle).unwrap_or_else(|| panic!("`{needle}` in {text:?}"))
}

#[test]
fn completion_offers_local_names_bare_collisions_with_this_and_imports_with_their_target() {
    let workspace = LspWorkspace::new();
    write(
        &workspace,
        "types.yaml",
        "kind: schema\ntypes:\n  shared: string\n  other: number\n",
    );
    write(&workspace, "union.yaml", "$schema:\n  - { a: string }\n  - { b: number }\n");
    write(&workspace, "raw.yaml", "type: object\nproperties:\n  c:\n    type: string\n");
    write(&workspace, "whole.yaml", "$schema: ./types.yaml\n");
    let schema = write(
        &workspace,
        "local.yaml",
        "kind: schema\ntypes:\n  alias: string\n  base: string\n  string: number\n  imported: 'shared@./types.yaml'\n  arm: 'x@./union.yaml'\n  raw: 'x@./raw.yaml'\n  whole: 'x@./whole.yaml'\n",
    );
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &schema);

    // A forward reference: `alias` is written before the names it may use.
    // The cursor sits before the value, so nothing filters the candidates.
    let items = completion_items(&mut fixture, &schema, offset_of(&schema.text, "alias: ") + 7);
    let labels = reference_labels(&items);
    for expected in ["base", "string@this", "imported", "alias", "shared@./types.yaml", "other@./types.yaml"] {
        assert!(labels.iter().any(|label| label == expected), "`{expected}` is offered: {labels:?}");
    }
    for unexpected in ["base@this", "string"] {
        assert!(labels.iter().all(|label| label != unexpected), "`{unexpected}` is not offered: {labels:?}");
    }
    // A root union, a raw JSON Schema, and a whole-file reference export no
    // names.
    for target in ["@./union.yaml", "@./raw.yaml", "@./whole.yaml"] {
        assert!(labels.iter().all(|label| !label.ends_with(target)), "nothing from `{target}`: {labels:?}");
    }
    let colliding = items
        .iter()
        .find(|item| item["label"] == json!("string@this"))
        .unwrap();
    assert_eq!(colliding["textEdit"]["newText"], json!("string@this"));

    // The same namespace from an inline `$schema` in Markdown.
    let document = write(
        &workspace,
        "doc.md",
        "---\n$schema:\n  title: la\n  label: string\n  number: string\n---\n\nBody\n",
    );
    open(&fixture, &document);
    let items = completion_items(&mut fixture, &document, offset_of(&document.text, "title: la") + 9);
    let labels = reference_labels(&items);
    assert!(labels.iter().any(|label| label == "label"), "forward local name, bare: {labels:?}");
    let items = completion_items(&mut fixture, &document, offset_of(&document.text, "title: ") + 7);
    let labels = reference_labels(&items);
    assert!(labels.iter().any(|label| label == "number@this"), "colliding local name: {labels:?}");
    assert!(labels.iter().all(|label| label != "number"), "no bare colliding name: {labels:?}");

    // An unfinished sibling definition does not hide an import's names.
    let editing = write(
        &workspace,
        "editing.md",
        "---\n$schema:\n  count: 'shared@./types.yaml'\n  next: \n---\n\nBody\n",
    );
    open(&fixture, &editing);
    let items = completion_items(&mut fixture, &editing, offset_of(&editing.text, "next: ") + 6);
    let labels = reference_labels(&items);
    assert!(labels.iter().any(|label| label == "other@./types.yaml"), "{labels:?}");

    fixture.shutdown();
}

#[test]
fn hover_on_a_reference_shows_the_definition_it_selects() {
    let workspace = LspWorkspace::new();
    write(
        &workspace,
        "types.yaml",
        "kind: schema\ntypes:\n  shared: 'number(min(1)) -> a shared count'\n",
    );
    let document = write(
        &workspace,
        "doc.md",
        "---\n$schema:\n  title: label\n  explicit: 'label@this'\n  count: 'shared@./types.yaml'\n  label: 'string(required)'\n---\n\nBody\n",
    );
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &document);

    for (needle, occurrence) in [("label", 0), ("label", 1)] {
        let start = document.text.match_indices(needle).nth(occurrence).unwrap().0;
        let hover = request_at(&mut fixture, "textDocument/hover", &document, start + 2);
        let markup = hover["contents"]["value"].as_str().unwrap_or_default();
        assert!(markup.contains("**`label`**"), "{needle} #{occurrence}: {markup}");
        assert!(markup.contains("Declares: **string**"), "the local definition: {markup}");
        assert!(markup.contains("Required"), "the definition's presence: {markup}");
        assert!(markup.contains("Defined in: this schema"), "{markup}");
        assert_eq!(hover["range"], range_of(&document.text, needle, occurrence));
    }

    let start = offset_of(&document.text, "shared@");
    let hover = request_at(&mut fixture, "textDocument/hover", &document, start);
    let markup = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(markup.contains("Declares: **number**"), "the imported definition: {markup}");
    assert!(markup.contains("Defined in: `./types.yaml`"), "{markup}");
    assert_eq!(hover["range"], range_of(&document.text, "shared", 0));

    // The definition hover of a key that holds a bare reference names it bare.
    let hover = request_at(&mut fixture, "textDocument/hover", &document, offset_of(&document.text, "title"));
    let markup = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(markup.contains("Declares: **label**"), "bare spelling in hover: {markup}");
    // A definition below the first `$schema` line keeps its exact key range.
    let hover = request_at(&mut fixture, "textDocument/hover", &document, offset_of(&document.text, "explicit"));
    assert!(hover["contents"]["value"].as_str().unwrap().contains("**`explicit`**"), "{hover}");
    assert_eq!(hover["range"], range_of(&document.text, "explicit", 0));

    fixture.shutdown();
}

#[test]
fn definition_selects_the_defining_file_and_key_for_every_reference_spelling() {
    let workspace = LspWorkspace::new();
    let types = write(
        &workspace,
        "types.yaml",
        "kind: schema\ntypes:\n  first: string\n  shared: number\n",
    );
    let schema = write(
        &workspace,
        "schema.yaml",
        "$schema:\n  bare: label\n  explicit: 'label@this'\n  cross: 'shared@./types.yaml'\n  missing: 'nope@./types.yaml'\n  label: string\n",
    );
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &schema);

    let local = json!([{ "uri": schema.uri, "range": range_of(&schema.text, "label", 2) }]);
    for occurrence in [0, 1] {
        let start = schema.text.match_indices("label").nth(occurrence).unwrap().0;
        let locations = request_at(&mut fixture, "textDocument/definition", &schema, start + 1);
        assert_eq!(locations, local, "reference #{occurrence}");
    }
    let locations = request_at(
        &mut fixture,
        "textDocument/definition",
        &schema,
        offset_of(&schema.text, "shared@"),
    );
    assert_eq!(
        locations,
        json!([{ "uri": canonical_uri(&types.path), "range": range_of(&types.text, "shared", 0) }])
    );
    let locations = request_at(
        &mut fixture,
        "textDocument/definition",
        &schema,
        offset_of(&schema.text, "nope@"),
    );
    assert!(
        locations.as_array().is_none_or(Vec::is_empty),
        "a name its target does not define has no definition: {locations}"
    );

    fixture.shutdown();
}

#[test]
fn a_missing_local_name_in_a_standalone_schema_is_reported_at_the_reference() {
    let workspace = LspWorkspace::new();
    let tagged = write(
        &workspace,
        "tagged.yaml",
        "kind: schema\ntypes:\n  title: nope\n  fine: base\n  base: string\n",
    );
    let pure = write(&workspace, "pure.yaml", "$schema:\n  title: \"nope(max(3))\"\n");
    let valid = write(&workspace, "valid.yaml", "kind: schema\ntypes:\n  title: base\n  base: string\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));

    for file in [&tagged, &pure] {
        open(&fixture, file);
        let diagnostics = fixture.wait_for_diagnostics(&file.uri);
        let named = named_type_diagnostics(&diagnostics);
        assert_eq!(named.len(), 1, "{}: {diagnostics:?}", file.path.display());
        assert_eq!(named[0]["range"], range_of(&file.text, "nope", 0));
        assert_eq!(named[0]["source"], json!("darkmatter.schema"));
        let message = named[0]["message"].as_str().unwrap();
        assert!(message.contains("unknown named type `nope`"), "{message}");
    }

    open(&fixture, &valid);
    let diagnostics = fixture.wait_for_diagnostics(&valid.uri);
    assert!(named_type_diagnostics(&diagnostics).is_empty(), "{diagnostics:?}");

    fixture.shutdown();
}

#[test]
fn a_missing_name_in_an_inline_schema_is_a_schema_definition_error_at_the_reference() {
    let workspace = LspWorkspace::new();
    write(&workspace, "types.yaml", "kind: schema\ntypes:\n  shared: string\n");
    let cases = [
        ("plain.md", "---\n$schema:\n  title: nope\n---\n\nBody\n", "nope"),
        ("quoted.md", "---\n$schema:\n  title: 'nope(max(3))'\n---\n\nBody\n", "nope"),
        ("flow.md", "---\n$schema: { title: \"other@./types.yaml\" }\n---\n\nBody\n", "other"),
    ];
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    for (name, text, reference) in cases {
        let file = write(&workspace, name, text);
        open(&fixture, &file);
        let diagnostics = fixture.wait_for_diagnostics(&file.uri);
        let named = named_type_diagnostics(&diagnostics);
        assert_eq!(named.len(), 1, "{name}: {diagnostics:?}");
        assert_eq!(named[0]["range"], range_of(text, reference, 0), "{name}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic["code"] != json!("dm.schema.type_mismatch")),
            "{name}: not a document-value error: {diagnostics:?}"
        );
    }
    fixture.shutdown();
}

/// A block scalar is inside the source-aware grammar, so a reference written
/// in one is ranged on its own name.
#[test]
fn a_block_scalar_reference_is_ranged_on_its_name() {
    let workspace = LspWorkspace::new();
    let text = "---\n$schema:\n  title: >-\n    nope\n---\n\nBody\n";
    let file = write(&workspace, "block.md", text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &file);
    let diagnostics = fixture.wait_for_diagnostics(&file.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "{diagnostics:?}");
    assert_eq!(
        named[0]["range"],
        range_of(text, "nope", 0),
        "the reference inside the block scalar: {:?}",
        named[0]
    );
    fixture.shutdown();
}

#[test]
fn a_multi_hop_contradiction_names_the_reference_and_both_declarations() {
    let workspace = LspWorkspace::new();
    let types = write(
        &workspace,
        "types.yaml",
        "kind: schema\ntypes:\n  label: \"number(min(10))\"\n  title_base: label\n",
    );
    let document = write(
        &workspace,
        "doc.md",
        "---\n$schema:\n  title: 'title_base(max(5))@./types.yaml'\n---\n\nBody\n",
    );
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &document);
    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "{diagnostics:?}");
    assert_eq!(named[0]["range"], range_of(&document.text, "title_base", 0));
    assert!(named[0]["message"].as_str().unwrap().contains("contradictory"), "{:?}", named[0]);
    let related: Vec<(Value, Value)> = named[0]["relatedInformation"]
        .as_array()
        .expect("related declarations")
        .iter()
        .map(|related| (related["location"]["uri"].clone(), related["location"]["range"].clone()))
        .collect();
    assert_eq!(
        related,
        vec![
            (json!(canonical_uri(&types.path)), range_of(&types.text, "min(10)", 0)),
            (json!(document.uri), range_of(&document.text, "max(5)", 0)),
        ]
    );
    fixture.shutdown();
}

#[test]
fn a_standalone_contradiction_and_a_mixed_cycle_are_schema_definition_errors() {
    let workspace = LspWorkspace::new();
    let conflict = write(
        &workspace,
        "conflict.yaml",
        "kind: schema\ntypes:\n  label: number(min(10))\n  title: label(max(5))\n",
    );
    let cycle = write(&workspace, "cycle.yaml", "$schema:\n  a: b\n  b: 'a@this'\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));

    open(&fixture, &conflict);
    let diagnostics = fixture.wait_for_diagnostics(&conflict.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "{diagnostics:?}");
    assert_eq!(named[0]["range"], range_of(&conflict.text, "label", 1));
    let related: Vec<Value> = named[0]["relatedInformation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|related| related["location"].clone())
        .collect();
    assert_eq!(
        related,
        vec![
            json!({ "uri": conflict.uri, "range": range_of(&conflict.text, "min(10)", 0) }),
            json!({ "uri": conflict.uri, "range": range_of(&conflict.text, "max(5)", 0) }),
        ]
    );

    open(&fixture, &cycle);
    let diagnostics = fixture.wait_for_diagnostics(&cycle.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "{diagnostics:?}");
    let message = named[0]["message"].as_str().unwrap();
    assert!(message.contains("import cycle"), "{message}");
    assert!(message.contains("b -> a -> b"), "the chain through both spellings: {message}");

    fixture.shutdown();
}

#[test]
fn an_error_inside_a_referenced_schema_points_into_that_file() {
    let workspace = LspWorkspace::new();
    let wrapper = write(&workspace, "wrapper.yaml", "$schema:\n  title: nope\n");
    let document = write(&workspace, "doc.md", "---\n$schema: ./wrapper.yaml\n---\n\nBody\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &document);
    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "{diagnostics:?}");
    assert_eq!(named[0]["range"], range_of(&document.text, "./wrapper.yaml", 0));
    let wrapper_uri = canonical_uri(&wrapper.path);
    let related: Vec<Value> = named[0]["relatedInformation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|related| related["location"].clone())
        .collect();
    assert_eq!(
        related,
        vec![
            json!({ "uri": wrapper_uri, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } } }),
            json!({ "uri": wrapper_uri, "range": range_of(&wrapper.text, "nope", 0) }),
        ]
    );
    fixture.shutdown();
}

#[test]
fn editing_an_imported_definition_refreshes_every_assistance_without_reopening() {
    let workspace = LspWorkspace::new();
    let types_v1 = "kind: schema\ntypes:\n  shared: string\n";
    let types_v2 = "kind: schema\ntypes:\n  renamed: string\n  shared: number\n";
    let types_v3 = "kind: schema\ntypes:\n  renamed: string\n";
    let types = write(&workspace, "types.yaml", types_v1);
    let document = write(
        &workspace,
        "doc.md",
        "---\n$schema:\n  count: 'shared@./types.yaml'\n  next: string\n---\n\nBody\n",
    );
    let types_uri = canonical_uri(&types.path);
    let reference = offset_of(&document.text, "shared@");
    let completion_offset = offset_of(&document.text, "next: ") + 6;
    let changed = |fixture: &LspFixture<'_>, text: &str| {
        std::fs::write(&types.path, text).unwrap();
        fixture.notify(
            "workspace/didChangeWatchedFiles",
            json!({ "changes": [{ "uri": types.uri, "type": 2 }] }),
        );
    };

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &document);
    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    assert!(named_type_diagnostics(&diagnostics).is_empty(), "{diagnostics:?}");
    let hover = request_at(&mut fixture, "textDocument/hover", &document, reference);
    assert!(hover["contents"]["value"].as_str().unwrap().contains("Declares: **string**"));
    let labels = reference_labels(&completion_items(&mut fixture, &document, completion_offset));
    assert!(labels.iter().all(|label| label != "renamed@./types.yaml"), "{labels:?}");

    changed(&fixture, types_v2);
    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    assert!(named_type_diagnostics(&diagnostics).is_empty(), "{diagnostics:?}");
    let hover = request_at(&mut fixture, "textDocument/hover", &document, reference);
    assert!(
        hover["contents"]["value"].as_str().unwrap().contains("Declares: **number**"),
        "hover follows the edited definition: {hover}"
    );
    let labels = reference_labels(&completion_items(&mut fixture, &document, completion_offset));
    assert!(labels.iter().any(|label| label == "renamed@./types.yaml"), "{labels:?}");
    let locations = request_at(&mut fixture, "textDocument/definition", &document, reference);
    assert_eq!(
        locations,
        json!([{ "uri": types_uri, "range": range_of(types_v2, "shared", 0) }]),
        "definition follows the moved key"
    );

    changed(&fixture, types_v3);
    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "removing the definition reports it: {diagnostics:?}");
    assert_eq!(named[0]["range"], range_of(&document.text, "shared", 0));

    changed(&fixture, types_v1);
    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    assert!(
        named_type_diagnostics(&diagnostics).is_empty(),
        "restoring the definition clears the cached failure: {diagnostics:?}"
    );

    fixture.shutdown();
}

#[test]
fn suggestions_on_a_reference_are_checked_against_the_resolved_target() {
    let workspace = LspWorkspace::new();
    let types = write(
        &workspace,
        "types.yaml",
        "kind: schema\ntypes:\n  code: string(max(2))\n  bad: 'string(max(2); suggest(abc))'\n",
    );
    let inline = write(
        &workspace,
        "doc.md",
        "---\n$schema:\n  label: string(max(3))\n  pick: 'label(suggest(ab, toolong))'\n  imported: 'code(suggest(xy, wxyz))@./types.yaml'\n  inherited: 'bad@./types.yaml'\n---\n\nBody\n",
    );
    let standalone = write(
        &workspace,
        "schema.yaml",
        "kind: schema\ntypes:\n  label: string(max(3))\n  pick: 'label(suggest(ab, toolong))'\n",
    );
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    let invalid = |diagnostics: &[Value]| -> Vec<Value> {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_suggestion"))
            .map(|diagnostic| diagnostic["range"].clone())
            .collect()
    };

    open(&fixture, &inline);
    let diagnostics = fixture.wait_for_diagnostics(&inline.uri);
    assert_eq!(
        invalid(&diagnostics),
        vec![range_of(&inline.text, "toolong", 0), range_of(&inline.text, "wxyz", 0)],
        "each use-site candidate is judged against its resolved target, and the \
         inherited `abc` stays with the file that declares it: {diagnostics:?}"
    );

    open(&fixture, &standalone);
    let diagnostics = fixture.wait_for_diagnostics(&standalone.uri);
    assert_eq!(invalid(&diagnostics), vec![range_of(&standalone.text, "toolong", 0)], "{diagnostics:?}");

    open(&fixture, &types);
    let diagnostics = fixture.wait_for_diagnostics(&types.uri);
    assert_eq!(invalid(&diagnostics), vec![range_of(&types.text, "abc", 0)], "{diagnostics:?}");

    fixture.shutdown();
}

#[test]
fn an_unsupported_union_postfix_is_reported_at_the_reference_with_its_definition() {
    let workspace = LspWorkspace::new();
    let types = write(&workspace, "types.yaml", "kind: schema\ntypes:\n  choice: [string, number]\n");
    let document = write(
        &workspace,
        "doc.md",
        "---\n$schema:\n  p: 'choice(max(3))@./types.yaml'\n  local: [string, number]\n---\n\nBody\n",
    );
    let inline = write(
        &workspace,
        "inline.md",
        "---\n$schema:\n  local: [string, number]\n  p: 'local[](pattern(^a))'\n---\n\nBody\n",
    );
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));

    open(&fixture, &document);
    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "{diagnostics:?}");
    assert_eq!(named[0]["range"], range_of(&document.text, "choice", 0));
    let message = named[0]["message"].as_str().unwrap();
    assert!(message.contains("`max`") && message.contains("union-typed"), "{message}");
    let related = named[0]["relatedInformation"].as_array().expect("the definition");
    assert_eq!(related.len(), 1, "{related:?}");
    assert_eq!(related[0]["location"]["uri"], json!(canonical_uri(&types.path)));
    assert_eq!(related[0]["location"]["range"]["start"]["line"], json!(2));
    assert_eq!(related[0]["message"], json!("`choice` is defined here"));

    open(&fixture, &inline);
    let diagnostics = fixture.wait_for_diagnostics(&inline.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "{diagnostics:?}");
    assert_eq!(named[0]["range"], range_of(&inline.text, "local", 1));
    assert!(named[0]["message"].as_str().unwrap().contains("`pattern`"), "{:?}", named[0]);
    let related = named[0]["relatedInformation"].as_array().expect("the definition");
    assert_eq!(related[0]["location"]["uri"], json!(inline.uri));
    assert_eq!(related[0]["location"]["range"]["start"]["line"], json!(2));

    fixture.shutdown();
}

#[test]
fn an_unsupported_union_postfix_inside_a_referenced_schema_points_into_that_file() {
    let workspace = LspWorkspace::new();
    let wrapper = write(
        &workspace,
        "wrapper.yaml",
        "$schema:\n  choice: [string, number]\n  p: choice(max(10))\n",
    );
    let document = write(&workspace, "doc.md", "---\n$schema: ./wrapper.yaml\n---\n\nBody\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &document);
    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "{diagnostics:?}");
    assert_eq!(named[0]["range"], range_of(&document.text, "./wrapper.yaml", 0));
    let message = named[0]["message"].as_str().unwrap();
    assert!(message.contains("`max`") && message.contains("union-typed"), "{message}");
    let wrapper_uri = canonical_uri(&wrapper.path);
    let related: Vec<Value> = named[0]["relatedInformation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|related| related["location"].clone())
        .collect();
    assert_eq!(
        related,
        vec![
            json!({ "uri": wrapper_uri, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } } }),
            json!({ "uri": wrapper_uri, "range": range_of(&wrapper.text, "choice", 1) }),
            json!({ "uri": wrapper_uri, "range": range_of(&wrapper.text, "[string, number]", 0) }),
        ]
    );
    fixture.shutdown();
}

#[test]
fn an_array_of_a_union_reference_prepares_without_schema_diagnostics() {
    let workspace = LspWorkspace::new();
    write(&workspace, "types.yaml", "kind: schema\ntypes:\n  choice: [string, number]\n");
    let document = write(
        &workspace,
        "doc.md",
        "---\n$schema:\n  p: 'choice[](min(1))@./types.yaml'\n  q: 'choice(required)@./types.yaml'\np: [a, 1]\nq: b\n---\n\nBody\n",
    );
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &document);
    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    fixture.shutdown();
}

/// A list before `[]` on a union reference: the `eager` marker prepares
/// cleanly and still navigates to its definition, while a value constraint
/// there is reported at the reference with the definition attached.
#[test]
fn a_marker_before_the_array_suffix_prepares_and_a_value_constraint_there_is_reported() {
    let workspace = LspWorkspace::new();
    let types = write(&workspace, "types.yaml", "kind: schema\ntypes:\n  choice: [string, number]\n");
    let accepted = write(
        &workspace,
        "accepted.md",
        "---\n$schema:\n  p: 'choice(eager)[]@./types.yaml'\np: [a, 1]\n---\n\nBody\n",
    );
    let rejected = write(
        &workspace,
        "rejected.md",
        "---\n$schema:\n  p: 'choice(max(2))[]@./types.yaml'\n---\n\nBody\n",
    );
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));

    open(&fixture, &accepted);
    let diagnostics = fixture.wait_for_diagnostics(&accepted.uri);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let locations = request_at(
        &mut fixture,
        "textDocument/definition",
        &accepted,
        offset_of(&accepted.text, "choice(eager)") + 1,
    );
    assert_eq!(
        locations,
        json!([{ "uri": canonical_uri(&types.path), "range": range_of(&types.text, "choice", 0) }])
    );

    open(&fixture, &rejected);
    let diagnostics = fixture.wait_for_diagnostics(&rejected.uri);
    let named = named_type_diagnostics(&diagnostics);
    assert_eq!(named.len(), 1, "{diagnostics:?}");
    assert_eq!(named[0]["range"], range_of(&rejected.text, "choice", 0));
    let message = named[0]["message"].as_str().unwrap();
    assert!(message.contains("`max`") && message.contains("union-typed"), "{message}");
    let related = named[0]["relatedInformation"].as_array().expect("the definition");
    assert_eq!(related[0]["location"]["uri"], json!(canonical_uri(&types.path)));

    fixture.shutdown();
}

/// An import target that is not YAML at all reports its YAML error, as a
/// diagnostic and in hover on the reference, instead of reading as a file
/// that exports nothing.
#[test]
fn a_reference_into_a_file_that_is_not_yaml_reports_the_yaml_error() {
    let workspace = LspWorkspace::new();
    write(&workspace, "types.yaml", "kind: schema\nkind: schema\ntypes:\n  shared: string\n");
    let document = write(
        &workspace,
        "doc.md",
        "---\n$schema:\n  count: 'shared@./types.yaml'\n---\n\nBody\n",
    );
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, &document);

    let diagnostics = fixture.wait_for_diagnostics(&document.uri);
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains("not valid YAML"))),
        "the import target's YAML error is a diagnostic: {diagnostics:?}"
    );

    let hover = request_at(&mut fixture, "textDocument/hover", &document, offset_of(&document.text, "shared@"));
    let markup = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(markup.contains("not valid YAML"), "hover names why the target is unusable: {hover}");
    assert!(markup.contains("duplicate entry"), "{markup}");
    assert_eq!(hover["range"], range_of(&document.text, "shared", 0));

    fixture.shutdown();
}
