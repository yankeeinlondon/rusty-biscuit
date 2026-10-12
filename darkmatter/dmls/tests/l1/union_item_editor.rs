//! Arrays whose items are a union (`policy[](min(1))@./content-policy.yaml`)
//! in DMLS: the overlay prepares them, hover names an item's alternative only
//! when exactly one accepts it, completion combines the alternatives unless a
//! literal tag selects one, and schema problems inside an item are published
//! as the library reports them. All through the in-memory LSP harness.
//!
//! The unsupported-postfix error (`choice(max(3))`) is covered at the
//! reference, with its definition as related information, by
//! `named_type_references::an_unsupported_union_postfix_is_reported_at_the_reference_with_its_definition`.

use crate::common;

use std::path::Path;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

#[path = "../../../lib/tests/union_fixtures/mod.rs"]
mod union_fixtures;

use union_fixtures::{
    CONTENT_POLICY_FILE, CONTENT_POLICY_LIST, NESTED_UNION_FILE, NESTED_UNION_TYPES,
    OVERLAPPING_TYPES, TAGGED_TYPES, content_policy_union_schema,
};

fn initialize_params(root: &Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "clientInfo": { "name": "Union Items", "version": "1.0" },
        "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
        "workspaceFolders": [{ "uri": root_uri.as_str(), "name": "scratch" }]
    })
}

/// An open document and its URI.
struct Document {
    uri: String,
    text: String,
}

/// A session over a workspace holding the Content Policy acceptance fixture
/// (`shipped` is the shipped schema text), the overlapping and tagged union
/// schemas, and `extra` files.
fn session<'w>(workspace: &'w LspWorkspace, shipped: &str, extra: &[(&str, &str)]) -> LspFixture<'w> {
    let root = workspace.path();
    std::fs::write(root.join(CONTENT_POLICY_FILE), content_policy_union_schema(shipped)).unwrap();
    std::fs::write(root.join("overlap.yaml"), OVERLAPPING_TYPES).unwrap();
    std::fs::write(root.join("tagged.yaml"), TAGGED_TYPES).unwrap();
    for (name, contents) in extra {
        std::fs::write(root.join(name), contents).unwrap();
    }
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(root));
    fixture
}

/// Opens `frontmatter` (the lines between the delimiters) as `name`, declaring
/// `content_policy`, `entries`, and `steps` as arrays of the fixture unions,
/// and returns it with its diagnostics.
fn open(fixture: &mut LspFixture<'_>, workspace: &LspWorkspace, name: &str, frontmatter: &str) -> (Document, Vec<Value>) {
    let text = format!(
        "---\n$schema:\n  content_policy: '{CONTENT_POLICY_LIST}'\n  entries: 'entry[]@./overlap.yaml'\n  steps: 'step[]@./tagged.yaml'\n{frontmatter}---\n\nBody\n"
    );
    open_text(fixture, workspace, name, &text)
}

fn open_text(fixture: &mut LspFixture<'_>, workspace: &LspWorkspace, name: &str, text: &str) -> (Document, Vec<Value>) {
    let path = workspace.path().join(name);
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().to_string();
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
    );
    let diagnostics = fixture.wait_for_diagnostics(&uri);
    (Document { uri, text: text.to_string() }, diagnostics)
}

/// The `(line, character)` of byte `offset` in ASCII `text`.
fn position(text: &str, offset: usize) -> Value {
    let before = &text[..offset];
    let line = before.matches('\n').count();
    let character = offset - before.rfind('\n').map_or(0, |index| index + 1);
    json!({ "line": line, "character": character })
}

/// The byte offset of the first `needle`: just past it when `after`, else at its start.
fn offset_of(text: &str, needle: &str, after: bool) -> usize {
    let start = text.find(needle).unwrap_or_else(|| panic!("`{needle}` in {text:?}"));
    if after { start + needle.len() } else { start }
}

fn request(fixture: &mut LspFixture<'_>, method: &str, document: &Document, offset: usize) -> Value {
    fixture
        .request(
            method,
            json!({
                "textDocument": { "uri": document.uri },
                "position": position(&document.text, offset)
            }),
        )
        .result
        .unwrap_or(Value::Null)
}

/// The hover Markdown at the start of `needle`.
fn hover(fixture: &mut LspFixture<'_>, document: &Document, needle: &str) -> String {
    let hover = request(fixture, "textDocument/hover", document, offset_of(&document.text, needle, false));
    hover["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("no hover at `{needle}`: {hover:?}"))
        .to_string()
}

/// `(label, kind, detail)` of every completion item just past `needle`.
fn completion(fixture: &mut LspFixture<'_>, document: &Document, needle: &str) -> Vec<(String, u64, Option<String>)> {
    let items = request(fixture, "textDocument/completion", document, offset_of(&document.text, needle, true));
    let items = items.as_array().cloned().unwrap_or_else(|| items["items"].as_array().cloned().unwrap_or_default());
    items
        .iter()
        .map(|item| {
            (
                item["label"].as_str().unwrap().to_string(),
                item["kind"].as_u64().unwrap_or_default(),
                item["detail"].as_str().map(str::to_string),
            )
        })
        .collect()
}

fn labels(items: &[(String, u64, Option<String>)]) -> Vec<&str> {
    items.iter().map(|(label, _, _)| label.as_str()).collect()
}

const FIELD: u64 = 5;
const VALUE: u64 = 12;

#[test]
fn the_overlay_prepares_an_array_of_a_union_and_hover_lists_its_alternatives() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, shipped, &[]);
    let (document, diagnostics) = open(
        &mut fixture,
        &workspace,
        "valid.md",
        "content_policy:\n  - ValidFor(3mo)\n  - rule: \"ValidUntil(2027-01-01)\"\n    action: archive\n",
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    let markdown = hover(&mut fixture, &document, "content_policy:\n");
    assert!(markdown.contains("Type: **array**, each item one of:"), "{markdown}");
    assert!(!markdown.contains("union"), "the alternatives replace the bare label: {markdown}");
    // Eight flattened `short_form` strings, then `long_form`, each with its description.
    assert_eq!(markdown.matches("\n- **").count(), 9, "{markdown}");
    assert!(markdown.contains("- **string** — Never triggers"), "{markdown}");
    assert!(markdown.contains("- **object** with `rule`, `action`"), "{markdown}");

    fixture.shutdown();
}

#[test]
fn hover_on_an_item_names_the_alternative_only_for_a_unique_match() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, shipped, &[]);
    let (document, _) = open(
        &mut fixture,
        &workspace,
        "items.md",
        "content_policy:\n  - ValidFor(3mo)\n  - Duration(3mo)\n  - null\n  - rule: Evergreen\n    action: archive\nentries:\n  - name: both\n  - name: second\n    size: 2\n",
    );

    // A scalar item valid against exactly one alternative shows that one.
    let matching = hover(&mut fixture, &document, "ValidFor(3mo)");
    assert!(matching.contains("**`content_policy[0]`**"), "{matching}");
    assert!(matching.contains("Matches alternative 3 of 9"), "{matching}");
    assert!(matching.contains("Interval from the default date property"), "{matching}");
    assert!(!matching.contains("Never triggers"), "only the matching alternative: {matching}");

    // An invalid item, and a missing one, list every alternative.
    let invalid = hover(&mut fixture, &document, "Duration(3mo)");
    assert!(invalid.contains("matches none of the 9 alternatives:"), "{invalid}");
    assert!(invalid.contains("Never triggers") && invalid.contains("Fixed deadline"), "{invalid}");
    let missing = hover(&mut fixture, &document, "null");
    assert!(missing.contains("no value; one of 9 alternatives:"), "{missing}");
    assert!(missing.contains("- **object** with `rule`, `action`"), "{missing}");

    // A field of an object item that matches one alternative names it.
    let field = hover(&mut fixture, &document, "action: archive");
    assert!(field.contains("**`action`**"), "{field}");
    assert!(field.contains("Values: _refresh_, _archive_, _remove_"), "{field}");
    assert!(field.contains("Item `content_policy[3]` matches alternative 9 of 9:"), "{field}");

    // Overlapping alternatives: `{ name }` is accepted by both, so the hover
    // lists them; adding `size` leaves exactly one.
    let ambiguous = hover(&mut fixture, &document, "name: both");
    assert!(ambiguous.contains("Item `entries[0]`: matches 2 of the 2 alternatives:"), "{ambiguous}");
    assert!(ambiguous.contains("- **object** with `name`, `size`"), "{ambiguous}");
    let unique = hover(&mut fixture, &document, "name: second");
    assert!(unique.contains("Item `entries[1]` matches alternative 2 of 2:"), "{unique}");

    fixture.shutdown();
}

#[test]
fn completion_in_an_item_combines_alternatives_unless_a_literal_tag_selects_one() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, shipped, &[]);

    // A new item: every object alternative's keys and every scalar value.
    let (document, _) = open(
        &mut fixture,
        &workspace,
        "new-item.md",
        "content_policy:\n  - ValidFor(3mo)\n  - \nsteps:\n  - \n",
    );
    let policy = completion(&mut fixture, &document, "content_policy:\n  - ValidFor(3mo)\n  - ");
    assert!(policy.contains(&("Evergreen".to_string(), VALUE, None)), "{policy:?}");
    assert!(policy.contains(&("rule".to_string(), FIELD, Some("(required)".to_string()))), "{policy:?}");
    assert!(policy.contains(&("action".to_string(), FIELD, Some("(required)".to_string()))), "{policy:?}");
    let steps = completion(&mut fixture, &document, "steps:\n  - ");
    assert_eq!(labels(&steps), ["kind", "run", "text"], "{steps:?}");

    // Inside an object item: an untagged item keeps every alternative's keys,
    // a tag selecting one alternative narrows them, and an unknown tag does not.
    let (document, _) = open(
        &mut fixture,
        &workspace,
        "object-items.md",
        "entries:\n  - name: x\n    \nsteps:\n  - kind: say\n    \n  - kind: other\n    \n",
    );
    let untagged = completion(&mut fixture, &document, "name: x\n    ");
    assert_eq!(labels(&untagged), ["size"], "authored `name` is excluded: {untagged:?}");
    let tagged = completion(&mut fixture, &document, "kind: say\n    ");
    assert_eq!(labels(&tagged), ["text"], "{tagged:?}");
    let unknown = completion(&mut fixture, &document, "kind: other\n    ");
    assert_eq!(labels(&unknown), ["run", "text"], "{unknown:?}");

    // A scalar item being typed: the values of the scalar alternatives.
    let (document, _) = open(&mut fixture, &workspace, "scalar-item.md", "content_policy:\n  - Ev\n");
    let values = completion(&mut fixture, &document, "  - Ev");
    assert_eq!(labels(&values), ["Evergreen"], "{values:?}");

    fixture.shutdown();
}

#[test]
fn a_problem_inside_an_item_is_published_as_the_library_reports_it() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let workspace = LspWorkspace::new();
    let declaration = format!("$schema:\n  content_policy: '{CONTENT_POLICY_LIST}'\n");
    let mut fixture = session(&workspace, shipped, &[("doc-schema.yaml", &declaration)]);
    let (document, diagnostics) = open_text(
        &mut fixture,
        &workspace,
        "invalid.md",
        "---\n$schema: ./doc-schema.yaml\ncontent_policy:\n  - ValidFor(3mo)\n  - rule: \"ValidFor(3mo)\"\n    action: delete\n  - action: archive\n---\n\nBody\n",
    );
    let reported: Vec<(u64, u64, &str, &str)> = diagnostics
        .iter()
        .map(|diagnostic| {
            let related = diagnostic["relatedInformation"][0]["location"]["uri"].as_str().unwrap_or_default();
            assert!(related.ends_with("/doc-schema.yaml"), "related information names the declaring schema: {diagnostic:?}");
            (
                diagnostic["range"]["start"]["line"].as_u64().unwrap(),
                diagnostic["range"]["start"]["character"].as_u64().unwrap(),
                diagnostic["code"].as_str().unwrap(),
                diagnostic["message"].as_str().unwrap(),
            )
        })
        .collect();
    // The valid scalar item reports nothing. In the object item the library
    // reports only the object alternative's problem, at `action`'s value,
    // never the scalar alternatives' type errors; the item missing `rule`
    // matches no alternative and is reported on the item.
    assert_eq!(
        reported,
        [
            (5, 12, "dm.schema.constraint", "\"delete\" is not one of \"refresh\", \"archive\" or \"remove\""),
            (
                6,
                4,
                "dm.schema.constraint",
                "{\"action\":\"archive\"} is not valid under any of the schemas listed in the 'anyOf' keyword"
            ),
        ],
        "{diagnostics:#?}"
    );
    assert_eq!(document.text.lines().nth(5).map(|line| &line[12..]), Some("delete"));

    fixture.shutdown();
}

#[test]
fn a_tagged_item_inside_a_postfixed_nested_union_publishes_what_the_direct_form_does() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, shipped, &[(NESTED_UNION_FILE, NESTED_UNION_TYPES)]);
    let mut published = |name: &str, type_expr: &str, value: &str| -> Vec<(Value, Value, Value)> {
        let text = format!("---\n$schema:\n  p: '{type_expr}@./types.yaml'\np:{value}---\n\nBody\n");
        let (_, diagnostics) = open_text(&mut fixture, &workspace, name, &text);
        diagnostics
            .iter()
            .map(|diagnostic| {
                (diagnostic["range"].clone(), diagnostic["code"].clone(), diagnostic["message"].clone())
            })
            .collect()
    };

    for (form, value, direct, nested) in [
        ("property", " {kind: a}\n", "direct_tag", "nested_tag"),
        ("item", "\n  - kind: a\n", "direct_tag[]", "nested_tag[]"),
    ] {
        let direct_published = published(&format!("direct-{form}.md"), direct, value);
        let nested_published = published(&format!("nested-{form}.md"), nested, value);
        assert!(
            nested_published
                .iter()
                .all(|(_, _, message)| !message.as_str().unwrap_or_default().contains("anyOf")),
            "the tag selects the arm, so no generic union failure: {nested_published:?}"
        );
        assert_eq!(nested_published, direct_published, "{nested}");
    }

    fixture.shutdown();
}
