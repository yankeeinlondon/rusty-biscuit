//! Object spread (`{ ...base, key: value }`) through a real LSP session:
//! hover and completion inside a spread operand see the same scope as any
//! other value position, an operand whose keys a later field overwrites is
//! still call-checked, and a spread of a value that is neither an object nor
//! `null` is an error ranged exactly on the operand, in the body and in a
//! frontmatter `{{ … }}` value.
//!
//! Every finding comes from Darkmatter's call check; these tests pin what the
//! editor shows for it.

use crate::common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

use dmls::diagnostics::codes::code::{EXPRESSION_INCOMPATIBLE_CALL as INCOMPATIBLE, EXPRESSION_INVALID_SPREAD_OPERAND as INVALID_SPREAD};

const ERROR: u64 = 1;

/// `base` is a closed object and `items` a list of them.
const FRONTMATTER: &str = concat!(
    "---\n$schema:\n",
    "  base: '{ name: string(required) }(required)'\n",
    "  items: '{ name: string(required) }[](required)'\n",
    "---\n\n",
);

fn initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": ["utf-8"] } },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "spread" } ]
    })
}

/// `(line, UTF-8 character)` of a byte offset.
fn position(text: &str, offset: usize) -> (u64, u64) {
    let line = text[..offset].matches('\n').count();
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    (line as u64, (offset - line_start) as u64)
}

/// One open document in an in-process session, with its latest published
/// diagnostics.
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
        let mut latest = fixture.wait_for_diagnostics_params(&uri);
        while let Some(params) = fixture.take_buffered_diagnostics(&uri) {
            latest = params;
        }
        let diagnostics = latest["diagnostics"].as_array().cloned().unwrap_or_default();
        Self { fixture, uri, text: text.to_string(), diagnostics }
    }

    fn params(&self, offset: usize) -> Value {
        let (line, character) = position(&self.text, offset);
        json!({ "textDocument": { "uri": self.uri }, "position": { "line": line, "character": character } })
    }

    fn hover(&mut self, offset: usize) -> String {
        let params = self.params(offset);
        let response = self.fixture.request("textDocument/hover", params);
        response
            .result
            .and_then(|result| result["contents"]["value"].as_str().map(str::to_string))
            .unwrap_or_default()
    }

    /// The labels of the completion items at `offset`, in served order.
    fn completion_labels(&mut self, offset: usize) -> Vec<String> {
        let params = self.params(offset);
        let response = self.fixture.request("textDocument/completion", params);
        let items = match response.result {
            Some(Value::Array(items)) => items,
            Some(result) => result["items"].as_array().cloned().unwrap_or_default(),
            None => Vec::new(),
        };
        items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect()
    }

    /// The diagnostics with `code`.
    fn with_code(&self, code: &str) -> Vec<&Value> {
        self.diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!(code)).collect()
    }

    fn shutdown(self) {
        self.fixture.shutdown();
    }
}

/// `((start line, start character), (end line, end character))` of `range`.
fn range_of(diagnostic: &Value) -> ((u64, u64), (u64, u64)) {
    let range = &diagnostic["range"];
    let point = |at: &Value| (at["line"].as_u64().unwrap(), at["character"].as_u64().unwrap());
    (point(&range["start"]), point(&range["end"]))
}

/// The authored range of the `nth` occurrence of `needle` in `text`.
fn authored(text: &str, needle: &str, nth: usize) -> ((u64, u64), (u64, u64)) {
    let at = text.match_indices(needle).nth(nth).unwrap_or_else(|| panic!("no `{needle}` #{nth}")).0;
    (position(text, at), position(text, at + needle.len()))
}

/// An expression hover without its `**Expression**` header and erased
/// expression line, which differ between the two spellings.
fn details(hover: &str) -> &str {
    hover.splitn(3, "\n\n").nth(2).unwrap_or_else(|| panic!("no hover details: {hover}"))
}

#[test]
fn object_spread_operand_hover_shows_the_declared_frontmatter_type() {
    let text = format!("{FRONTMATTER}{{{{ {{ ...base, k: 1 }} }}}}\n");
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);

    let operand = text.find("...base").unwrap() + 4;
    let spread = session.hover(operand);
    let plain_text = format!("{FRONTMATTER}{{{{ {{ k: base }} }}}}\n");
    session.shutdown();

    let mut plain = Session::open(&workspace, &plain_text);
    let field = plain_text.find("k: base").unwrap() + 4;
    let value = plain.hover(field);
    plain.shutdown();

    assert!(spread.contains("Type: **object**"), "the declared type: {spread}");
    assert_eq!(details(&spread), details(&value), "a spread operand hovers as a field value does");
}

#[test]
fn object_spread_operand_in_a_callback_body_hovers_the_callback_binding() {
    let text = format!("{FRONTMATTER}{{{{ map(items, item => {{ ...item, seen: true }}) }}}}\n");
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);

    let operand = text.find("...item").unwrap() + 4;
    let hover = session.hover(operand);
    session.shutdown();

    assert!(hover.contains("Local `item`"), "{hover}");
    assert!(hover.contains("Parameter of `map`"), "{hover}");
    assert!(hover.contains("name: string"), "the element type: {hover}");
}

#[test]
fn object_spread_operand_completion_matches_a_field_value_position() {
    let workspace = LspWorkspace::new();
    let complete = |text: String| {
        let mut session = Session::open(&workspace, &text);
        let labels = session.completion_labels(text.len());
        session.shutdown();
        labels
    };

    let spread = complete(format!("{FRONTMATTER}{{{{ {{ ...ba"));
    let field = complete(format!("{FRONTMATTER}{{{{ {{ k: ba"));
    assert!(spread.iter().any(|label| label == "base"), "{spread:?}");
    assert_eq!(spread, field);

    let spread = complete(format!("{FRONTMATTER}{{{{ map(items, item => {{ ...it"));
    let field = complete(format!("{FRONTMATTER}{{{{ map(items, item => {{ k: it"));
    assert!(spread.iter().any(|label| label == "item"), "the callback local: {spread:?}");
    assert_eq!(spread, field);
}

#[test]
fn object_spread_operand_overwritten_by_a_later_field_is_still_call_checked() {
    let text = format!("{FRONTMATTER}{{{{ {{ ...{{ k: min('pear', 1) }}, k: 1 }} }}}}\n");
    let workspace = LspWorkspace::new();
    let session = Session::open(&workspace, &text);
    let found = session.with_code(INCOMPATIBLE);
    let [finding] = found.as_slice() else { panic!("one incompatible call: {:#?}", session.diagnostics) };
    assert_eq!(finding["severity"], json!(ERROR), "{finding:#?}");
    assert_eq!(range_of(finding), authored(&text, "'pear'", 0), "{finding:#?}");
    session.shutdown();
}

/// Under both `expressions.strict` settings: an operand that is a number or
/// an array is an error on exactly its authored bytes, in a frontmatter
/// `{{ … }}` value and in the body; `null` and an undeclared (unknown) operand
/// are not reported.
#[test]
fn object_spread_of_a_non_object_is_an_error_ranged_on_the_operand() {
    let text = concat!(
        "---\ntitle: \"{{ { ...7 } }}\"\n---\n\n",
        "{{ { ...5 } }}\n",
        "{{ { a: 1, ...[1, 2] } }}\n",
        "{{ { ...undeclared_base, ...null } }}\n",
    );
    assert_eq!(INVALID_SPREAD, "dm.expression.invalid_spread_operand", "the published code");
    for strict in [false, true] {
        let workspace = LspWorkspace::new();
        std::fs::write(workspace.path().join(".dmls.toml"), format!("[expressions]\nstrict = {strict}\n")).unwrap();
        let session = Session::open(&workspace, text);
        let mut found = session.with_code(INVALID_SPREAD);
        found.sort_by_key(|diagnostic| range_of(diagnostic));
        let ranges: Vec<_> = found.iter().map(|diagnostic| range_of(diagnostic)).collect();
        assert_eq!(
            ranges,
            vec![authored(text, "7", 0), authored(text, "5", 0), authored(text, "[1, 2]", 0)],
            "strict={strict}: {:#?}",
            session.diagnostics
        );
        for diagnostic in &found {
            assert_eq!(diagnostic["severity"], json!(ERROR), "strict={strict}: {diagnostic:#?}");
        }
        assert_eq!(found[1]["message"], json!("object spread requires an object or null, found number"));
        assert_eq!(found[2]["message"], json!("object spread requires an object or null, found array"));
        session.shutdown();
    }
}
