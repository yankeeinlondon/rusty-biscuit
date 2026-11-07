//! Type-definition authoring inside a string argument whose catalog
//! parameter is a `type-definition` (`load_data`'s `as`): hover and
//! completion in body and frontmatter expressions, positions mapped through
//! string escapes and YAML quoting, unfinished strings and calls, and the
//! callback members the passive call check types from a literal contract.
//! A named-reference or computed contract types as unknown and opens no
//! file.

use crate::common::{LspFixture, LspWorkspace};

use serde_json::{Value, json};

fn initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "scratch" } ]
    })
}

/// `(line, UTF-16 character)` of a byte offset.
fn position(text: &str, offset: usize) -> (u64, u64) {
    let line = text[..offset].matches('\n').count();
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    (line as u64, text[line_start..offset].encode_utf16().count() as u64)
}

/// Byte offset just past the first `needle`.
fn after(text: &str, needle: &str) -> usize {
    text.find(needle).unwrap_or_else(|| panic!("no `{needle}` in {text:?}")) + needle.len()
}

/// Byte offset inside the first `needle`, `inner` bytes into it.
fn inside(text: &str, needle: &str, inner: usize) -> usize {
    text.find(needle).unwrap_or_else(|| panic!("no `{needle}` in {text:?}")) + inner
}

/// One open document in an in-process session.
struct Session<'w> {
    fixture: LspFixture<'w>,
    uri: String,
    text: String,
    diagnostics: Vec<Value>,
}

impl<'w> Session<'w> {
    fn open(workspace: &'w LspWorkspace, text: &str) -> Self {
        let path = workspace.path().join("doc.md");
        std::fs::write(&path, text).unwrap();
        let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
        let mut fixture = LspFixture::start(workspace);
        fixture.initialize(initialize_params(workspace.path()));
        fixture.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
        );
        fixture.flush_server();
        let latest = fixture.wait_for_diagnostics_params(&uri);
        while fixture.take_buffered_diagnostics(&uri).is_some() {}
        let diagnostics = latest["diagnostics"].as_array().cloned().unwrap_or_default();
        Self { fixture, uri, text: text.to_string(), diagnostics }
    }

    fn params(&self, offset: usize) -> Value {
        let (line, character) = position(&self.text, offset);
        json!({ "textDocument": { "uri": self.uri }, "position": { "line": line, "character": character } })
    }

    /// The hover's Markdown and range, or an empty string and `Null`.
    fn hover(&mut self, offset: usize) -> (String, Value) {
        let params = self.params(offset);
        let result = self.fixture.request("textDocument/hover", params).result.unwrap_or(Value::Null);
        (result["contents"]["value"].as_str().unwrap_or_default().to_string(), result["range"].clone())
    }

    fn completion(&mut self, offset: usize) -> Vec<Value> {
        let params = self.params(offset);
        match self.fixture.request("textDocument/completion", params).result {
            Some(Value::Array(items)) => items,
            Some(result) => result["items"].as_array().cloned().unwrap_or_default(),
            None => Vec::new(),
        }
    }

    /// The LSP range of the byte span `span`.
    fn range(&self, span: std::ops::Range<usize>) -> Value {
        let (start_line, start_character) = position(&self.text, span.start);
        let (end_line, end_character) = position(&self.text, span.end);
        json!({
            "start": { "line": start_line, "character": start_character },
            "end": { "line": end_line, "character": end_character },
        })
    }

    fn shutdown(self) {
        self.fixture.shutdown();
    }
}

fn labels(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect()
}

fn item<'a>(items: &'a [Value], label: &str) -> &'a Value {
    items
        .iter()
        .find(|item| item["label"] == json!(label))
        .unwrap_or_else(|| panic!("no `{label}` in {:?}", labels(items)))
}

/// Asserts `hover` is the type-definition hover of the `as` parameter.
fn assert_type_definition_hover(hover: &str, declares: &str) {
    assert!(hover.contains("**`as`**"), "{hover}");
    assert!(hover.contains("Type: **type-definition**"), "{hover}");
    assert!(hover.contains(&format!("Declares: **{declares}**")), "{hover}");
}

/// Asserts `items` are type-definition candidates, not expression ones.
fn assert_type_candidates(items: &[Value]) {
    let labels = labels(items);
    assert!(labels.iter().any(|label| label == "string"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "string[]"), "{labels:?}");
    assert!(!labels.iter().any(|label| label.contains('(') || label.starts_with("ctx.")), "{labels:?}");
    assert!(!labels.iter().any(|label| label.ends_with("@this")), "{labels:?}");
}

#[test]
fn body_hover_over_a_type_keyword_shows_the_type_definition_hover() {
    let workspace = LspWorkspace::new();
    let text = "# Doc\n\n{{ load_data(\"x.yaml\", \"string[]\") }}\n";
    let mut session = Session::open(&workspace, text);

    let (hover, range) = session.hover(inside(text, "string[]", 2));
    assert_type_definition_hover(&hover, "string[]");
    assert!(hover.contains("**`string`**"), "{hover}");
    let literal = text.find("\"string[]\"").unwrap();
    assert_eq!(range, session.range(literal..literal + "\"string[]\"".len()));

    session.shutdown();
}

#[test]
fn body_completion_inside_the_string_offers_type_names_over_the_partial() {
    let workspace = LspWorkspace::new();
    let text = "# Doc\n\n{{ load_data(\"x.yaml\", \"str[]\") }}\n";
    let mut session = Session::open(&workspace, text);

    let cursor = after(text, "\"str");
    let items = session.completion(cursor);
    assert_type_candidates(&items);
    let edit = &item(&items, "string")["textEdit"];
    assert_eq!(edit["range"], session.range(cursor - 3..cursor));
    assert_eq!(edit["newText"], json!("string"));

    session.shutdown();
}

#[test]
fn escaped_quotes_before_the_cursor_map_to_authored_bytes() {
    let workspace = LspWorkspace::new();
    let text = "# Doc\n\n{{ load_data(\"x.yaml\", \"string(pattern(\\\"^a\\\\d\\\")) | num\") }}\n";
    let mut session = Session::open(&workspace, text);

    let cursor = after(text, "| num");
    let items = session.completion(cursor);
    let edit = &item(&items, "number")["textEdit"];
    assert_eq!(edit["range"], session.range(cursor - 3..cursor));
    assert_eq!(edit["newText"], json!("number"));

    session.shutdown();
}

#[test]
fn a_nested_type_string_holding_braces_keeps_the_cursor_in_the_argument() {
    let workspace = LspWorkspace::new();
    let text = "# Doc\n\n{{ load_data(\"x.yaml\", \"{ a: { b: string }}[]\") }}\n\n{{ load_data(\"x.yaml\", \"{ a: { b: str }}[]\") }}\n";
    let mut session = Session::open(&workspace, text);

    let (hover, _) = session.hover(inside(text, "b: string", 4));
    assert_type_definition_hover(&hover, "object[]");
    let cursor = after(text, "b: str ") - 1;
    let items = session.completion(cursor);
    assert_type_candidates(&items);
    assert_eq!(item(&items, "string")["textEdit"]["range"], session.range(cursor - 3..cursor));

    session.shutdown();
}

#[test]
fn completion_tolerates_an_unfinished_string_and_call() {
    let workspace = LspWorkspace::new();
    let text = "# Doc\n\n{{ load_data(\"x.yaml\", \"str\nMore prose.\n";
    let mut session = Session::open(&workspace, text);

    let cursor = after(text, "\"str");
    let items = session.completion(cursor);
    assert_type_candidates(&items);
    assert_eq!(item(&items, "string")["textEdit"]["range"], session.range(cursor - 3..cursor));
    // A constraint list opened in the unfinished string completes too.
    let text = "# Doc\n\n{{ map(load_data(\"x.yaml\", \"string(m";
    let mut unfinished = Session::open(&workspace, text);
    let constraints = labels(&unfinished.completion(text.len()));
    assert!(constraints.iter().any(|label| label == "min"), "{constraints:?}");
    assert!(!constraints.iter().any(|label| label == "string"), "{constraints:?}");

    session.shutdown();
    unfinished.shutdown();
}

#[test]
fn frontmatter_expressions_project_through_yaml_quoting() {
    let workspace = LspWorkspace::new();
    let text = concat!(
        "---\n",
        "hosts: \"{{ load_data('h.yaml', '{ name: string }[]') }}\"\n",
        "names: '{{ load_data(''h.yaml'', ''{ name: str }[]'') }}'\n",
        "---\n# Doc\n",
    );
    let mut session = Session::open(&workspace, text);

    let (hover, range) = session.hover(inside(text, "name: string", 7));
    assert_type_definition_hover(&hover, "object[]");
    let literal = text.find("'{ name: string }[]'").unwrap();
    assert_eq!(range, session.range(literal..literal + "'{ name: string }[]'".len()));

    let cursor = after(text, "name: str ") - 1;
    let items = session.completion(cursor);
    assert_type_candidates(&items);
    assert_eq!(item(&items, "string")["textEdit"]["range"], session.range(cursor - 3..cursor));
    assert_eq!(labels(&items).iter().filter(|label| *label == "string").count(), 1);

    session.shutdown();
}

#[test]
fn an_expression_typed_value_is_read_as_one_expression() {
    let workspace = LspWorkspace::new();
    let text = concat!(
        "---\n$schema:\n  pick: expression\n",
        "pick: \"load_data('h.yaml', 'number[]') || load_data('h.yaml', 'str')\"\n",
        "---\n# Doc\n",
    );
    let mut session = Session::open(&workspace, text);

    let (hover, range) = session.hover(inside(text, "number[]", 1));
    assert_type_definition_hover(&hover, "number[]");
    let literal = text.find("'number[]'").unwrap();
    assert_eq!(range, session.range(literal..literal + "'number[]'".len()));
    let cursor = after(text, "'str");
    let items = session.completion(cursor);
    assert_type_candidates(&items);
    assert_eq!(item(&items, "string")["textEdit"]["range"], session.range(cursor - 3..cursor));

    session.shutdown();
}

#[test]
fn a_non_type_definition_string_argument_gets_no_type_definition_authoring() {
    let workspace = LspWorkspace::new();
    let text = "# Doc\n\n{{ load_data(\"string.yaml\", \"string[]\") }}\n\n{{ upper(\"str\") }}\n";
    let mut session = Session::open(&workspace, text);

    let (hover, _) = session.hover(inside(text, "string.yaml", 2));
    assert!(!hover.contains("type-definition**\n"), "{hover}");
    assert!(!hover.contains("Declares:"), "{hover}");
    let file = labels(&session.completion(inside(text, "string.yaml", 3)));
    assert!(!file.iter().any(|label| label == "string[]"), "{file:?}");
    let other = labels(&session.completion(after(text, "upper(\"str")));
    assert!(!other.iter().any(|label| label == "string[]"), "{other:?}");

    session.shutdown();
}

/// The `h.` member labels at the end of the first `h => h.` in `text`.
fn member_labels(session: &mut Session, text: &str) -> Vec<String> {
    labels(&session.completion(after(text, "h => h.")))
        .into_iter()
        .filter(|label| label.starts_with("h."))
        .collect()
}

#[test]
fn a_literal_contract_types_callback_members() {
    let workspace = LspWorkspace::new();
    std::fs::write(workspace.path().join("hosts.yaml"), "- name: a\n  port: 1\n").unwrap();
    let text = "# Doc\n\n{{ map(load_data(\"hosts.yaml\", \"{ name: string, port: number }[]\"), h => h. }}\n";
    let mut session = Session::open(&workspace, text);

    assert_eq!(member_labels(&mut session, text), ["h.name", "h.port"]);

    session.shutdown();
}

#[test]
fn a_computed_contract_types_as_unknown_without_a_diagnostic() {
    let workspace = LspWorkspace::new();
    std::fs::write(workspace.path().join("hosts.yaml"), "- name: a\n").unwrap();
    let text = concat!(
        "---\nt: \"{ name: string }[]\"\n---\n# Doc\n\n",
        "{{ map(load_data(\"hosts.yaml\", t), h => h. }}\n\n",
        "{{ map(load_data(\"hosts.yaml\", t), h => h.name) }}\n",
    );
    let mut session = Session::open(&workspace, text);

    assert!(member_labels(&mut session, text).is_empty());
    let complete = text.rfind("{{ map").unwrap();
    let on_complete: Vec<&Value> = session
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["range"]["start"]["line"] == json!(position(text, complete).0))
        .collect();
    assert!(on_complete.is_empty(), "{on_complete:?}");

    session.shutdown();
}

#[test]
fn a_named_reference_types_as_unknown_and_opens_no_file() {
    let workspace = LspWorkspace::new();
    std::fs::write(workspace.path().join("hosts.yaml"), "- secret: a\n").unwrap();
    std::fs::write(workspace.path().join("types.yaml"), "Hosts: '{ secret: string }[]'\n").unwrap();
    let text = "# Doc\n\n{{ map(load_data(\"hosts.yaml\", \"Hosts@./types.yaml\"), h => h. }}\n";
    let mut session = Session::open(&workspace, text);

    let before = dmls::work_counters::work_counts();
    assert!(member_labels(&mut session, text).is_empty());
    let (hover, _) = session.hover(inside(text, "Hosts@", 2));
    assert_type_definition_hover(&hover, "Hosts@./types.yaml");
    let reference = labels(&session.completion(after(text, "\"hosts.yaml\", \"")));
    assert!(reference.iter().any(|label| label == "Name@./types.yaml"), "{reference:?}");
    assert_eq!(dmls::work_counters::work_counts().since(before).reads, 0);

    session.shutdown();
}
