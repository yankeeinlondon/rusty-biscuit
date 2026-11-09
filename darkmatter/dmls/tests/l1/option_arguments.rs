//! Editing `load_data`'s options object: key and enum-value completion from
//! the catalog's declared shape, hover, diagnostics ranged on the third
//! argument, and inferred result types at both arities.

use crate::common::{LspFixture, LspWorkspace};

use serde_json::{Value, json};
use simplified_schema::{PropertyDef, TypeExpr};

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

    fn hover(&mut self, offset: usize) -> String {
        let params = self.params(offset);
        let result = self.fixture.request("textDocument/hover", params).result.unwrap_or(Value::Null);
        result["contents"]["value"].as_str().unwrap_or_default().to_string()
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

    /// The one error diagnostic starting on the line of byte `offset`.
    fn error_on_line_of(&self, offset: usize) -> &Value {
        let line = position(&self.text, offset).0;
        let errors: Vec<&Value> = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["severity"] == json!(1) && diagnostic["range"]["start"]["line"] == json!(line))
            .collect();
        let [error] = errors.as_slice() else { panic!("one error on line {line}: {:#?}", self.diagnostics) };
        error
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

/// The property names the catalog declares for `load_data`'s third
/// parameter, and each `enum` property's members.
fn declared_options() -> (Vec<String>, Vec<String>) {
    let descriptor = dmls::overlay::expressions::function_overloads("load_data")
        .iter()
        .find(|descriptor| descriptor.function_type.parameters.as_slice().len() == 3)
        .expect("a three-parameter load_data overload");
    let parameter = &descriptor.function_type.parameters.as_slice()[2];
    let atoms = match &parameter.def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(arms) => arms.as_slice(),
    };
    let shape = atoms
        .iter()
        .find_map(|atom| match &atom.ty {
            TypeExpr::InlineObject(shape) => Some(shape),
            _ => None,
        })
        .expect("an inline-object options parameter");
    let format = match &shape.properties["format"] {
        PropertyDef::Single(atom) => atom.constraints.iter().find_map(|constraint| match constraint {
            simplified_schema::Constraint::Members(members) => Some(members.clone()),
            _ => None,
        }),
        PropertyDef::Union(_) => None,
    };
    (shape.properties.keys().cloned().collect(), format.expect("format members"))
}

#[test]
fn body_completion_offers_declared_keys_at_the_brace_and_after_a_comma() {
    let workspace = LspWorkspace::new();
    let text = concat!(
        "# Doc\n\n",
        "{{ load_data(\"rows.txt\", \"tuple([string])[]\", {}) }}\n\n",
        "{{ load_data(\"rows.txt\", \"tuple([string])[]\", {format: \"csv\", }) }}\n\n",
        "{{ load_data(\"rows.txt\", \"tuple([string])[]\", {format: \"csv\", hea}) }}\n",
    );
    let mut session = Session::open(&workspace, text);
    let (declared, _) = declared_options();

    let at_brace = session.completion(after(text, "[]\", {"));
    assert_eq!(labels(&at_brace), declared, "the offered keys are the catalog's, in declared order");
    assert_eq!(declared, ["format", "delimiter", "header"]);
    let format = item(&at_brace, "format");
    assert_eq!(format["kind"], json!(10), "PROPERTY");
    assert_eq!(format["textEdit"]["newText"], json!("format: "));
    let cursor = after(text, "[]\", {");
    assert_eq!(format["textEdit"]["range"], session.range(cursor..cursor));

    let after_comma = session.completion(after(text, "\"csv\", "));
    assert_eq!(labels(&after_comma), ["delimiter", "header"], "a key already written is not offered again");

    let cursor = after(text, "hea");
    let partial = session.completion(cursor);
    assert_eq!(labels(&partial), ["header"]);
    assert_eq!(item(&partial, "header")["textEdit"]["range"], session.range(cursor - 3..cursor));

    session.shutdown();
}

#[test]
fn enum_and_boolean_values_complete_and_other_values_fall_through() {
    let workspace = LspWorkspace::new();
    let text = concat!(
        "# Doc\n\n",
        "{{ load_data(\"rows.txt\", \"tuple([string])[]\", {format: }) }}\n\n",
        "{{ load_data(\"rows.txt\", \"tuple([string])[]\", {format: \"cs\"}) }}\n\n",
        "{{ load_data(\"rows.txt\", \"tuple([string])[]\", {header: }) }}\n\n",
        "{{ load_data(\"rows.txt\", \"tuple([string])[]\", {delimiter: }) }}\n",
    );
    let mut session = Session::open(&workspace, text);
    let (_, members) = declared_options();

    let format = session.completion(after(text, "{format: "));
    let quoted: Vec<String> = members.iter().map(|member| format!("\"{member}\"")).collect();
    assert_eq!(labels(&format), quoted);
    assert_eq!(members.len(), 9, "{members:?}");
    assert!(members.iter().any(|member| member == "yml") && members.iter().any(|member| member == "ndjson"));
    assert_eq!(item(&format, "\"csv\"")["kind"], json!(20), "ENUM_MEMBER");
    assert_eq!(item(&format, "\"csv\"")["textEdit"]["newText"], json!("\"csv\""));

    let cursor = after(text, "{format: \"cs");
    let partial = session.completion(cursor);
    assert_eq!(labels(&partial), ["\"csv\""]);
    let edit = &item(&partial, "\"csv\"")["textEdit"];
    assert_eq!(edit["newText"], json!("csv"), "inside the quotes only the member is inserted");
    assert_eq!(edit["range"], session.range(cursor - 2..cursor));

    let header = session.completion(after(text, "{header: "));
    assert_eq!(labels(&header), ["true", "false"]);
    assert_eq!(item(&header, "true")["kind"], json!(12), "VALUE");

    let delimiter = labels(&session.completion(after(text, "{delimiter: ")));
    assert!(!delimiter.iter().any(|label| label.starts_with('"') || label == "format"), "{delimiter:?}");
    assert!(delimiter.iter().any(|label| label.starts_with("load_data(")), "ordinary completion runs: {delimiter:?}");

    session.shutdown();
}

#[test]
fn a_plain_object_parameter_gets_no_declared_key_completion() {
    let workspace = LspWorkspace::new();
    let text = "# Doc\n\n{{ pr_list({}) }}\n\n{{ load_data(\"rows.txt\", {}) }}\n";
    let mut session = Session::open(&workspace, text);

    for cursor in [after(text, "pr_list({"), after(text, "\"rows.txt\", {")] {
        let found = labels(&session.completion(cursor));
        assert!(!found.iter().any(|label| ["format", "delimiter", "header"].contains(&label.as_str())), "{found:?}");
    }

    session.shutdown();
}

#[test]
fn frontmatter_option_completion_projects_through_yaml_quoting() {
    let workspace = LspWorkspace::new();
    let text = concat!(
        "---\n$schema:\n  pick: expression\n",
        "rows: \"{{ load_data('r.csv', 'tuple([string])[]', {format: }) }}\"\n",
        "names: '{{ load_data(''r.csv'', ''tuple([string])[]'', {}) }}'\n",
        "pick: \"load_data('r.csv', 'tuple([string])[]', {header: true, })\"\n",
        "---\n# Doc\n",
    );
    let mut session = Session::open(&workspace, text);

    let format = session.completion(after(text, "{format: "));
    assert_eq!(labels(&format).len(), 9, "{:?}", labels(&format));
    assert_eq!(item(&format, "\"csv\"")["textEdit"]["newText"], json!("\\\"csv\\\""), "escaped for the double-quoted scalar");

    let keys = labels(&session.completion(after(text, "[]'', {")));
    assert_eq!(keys, ["format", "delimiter", "header"]);

    let expression = labels(&session.completion(after(text, "{header: true, ")));
    assert_eq!(expression, ["format", "delimiter"]);

    session.shutdown();
}

#[test]
fn hover_on_load_data_shows_the_options_parameter_and_its_defaults() {
    let workspace = LspWorkspace::new();
    let text = "# Doc\n\n{{ load_data(\"rows.csv\", \"tuple([string])[]\", {header: false}) }}\n";
    let mut session = Session::open(&workspace, text);

    let hover = session.hover(after(text, "{{ load_"));
    assert!(hover.contains("[options: {format: enum(yaml,yml,json,jsonl,ndjson,csv,tsv,toml,json5)"), "{hover}");
    for key in ["format:", "delimiter:", "header:"] {
        assert!(hover.contains(key), "{key} in {hover}");
    }
    assert!(hover.contains("default a comma"), "{hover}");
    assert!(hover.contains("(default `true`)"), "{hover}");

    session.shutdown();
}

/// Opens `call` as the only body expression and asserts its one error has
/// `code` and starts where `argument` starts.
fn assert_error_at(call: &str, argument: &str, code: &str) {
    let workspace = LspWorkspace::new();
    let text = format!("# Doc\n\n{{{{ {call} }}}}\n");
    let session = Session::open(&workspace, &text);
    let start = text.find(argument).unwrap_or_else(|| panic!("no `{argument}` in {text}"));
    let error = session.error_on_line_of(start);
    assert_eq!(error["code"], json!(code), "{error:#?}");
    let (line, character) = position(&text, start);
    assert_eq!(error["range"]["start"], json!({ "line": line, "character": character }), "{error:#?}");
    session.shutdown();
}

#[test]
fn invalid_delimiter_is_ranged_on_the_third_argument() {
    assert_error_at(
        r#"load_data("rows.csv", "tuple([string, string])[]", {delimiter: ";;"})"#,
        r#"{delimiter"#,
        "dm.data.invalid_options",
    );
}

#[test]
fn unknown_option_key_is_ranged_on_the_third_argument() {
    assert_error_at(r#"load_data("rows.json", "string[]", {quote: "'"})"#, "{quote", "dm.expression.incompatible_call");
}

#[test]
fn tabular_option_on_json_is_ranged_on_the_third_argument() {
    assert_error_at(r#"load_data("rows.json", "string[]", {header: false})"#, "{header", "dm.data.invalid_options");
}

#[test]
fn header_false_with_object_contract_is_ranged_on_as() {
    assert_error_at(
        r#"load_data("rows.csv", "{ name: string }[]", {header: false})"#,
        r#""{ name"#,
        "dm.data.row_shape_conflict",
    );
}

#[test]
fn computed_options_with_a_literal_json_file_produce_no_diagnostic() {
    let workspace = LspWorkspace::new();
    let text = concat!(
        "---\n$schema:\n  opts: object\n  flag: boolean\n---\n# Doc\n\n",
        "{{ load_data(\"rows.json\", \"string[]\", opts) }}\n\n",
        "{{ load_data(\"rows.json\", \"string[]\", {header: flag}) }}\n",
    );
    let session = Session::open(&workspace, text);

    let errors: Vec<&Value> = session.diagnostics.iter().filter(|diagnostic| diagnostic["severity"] == json!(1)).collect();
    assert!(errors.is_empty(), "{errors:#?}");

    session.shutdown();
}

/// The `h.` member labels at the end of the first `needle` in `text`.
fn member_labels(session: &mut Session, text: &str, needle: &str) -> Vec<String> {
    labels(&session.completion(after(text, needle)))
        .into_iter()
        .filter(|label| label.starts_with("h."))
        .collect()
}

#[test]
fn both_arities_keep_callback_member_completion() {
    let workspace = LspWorkspace::new();
    let text = concat!(
        "# Doc\n\n",
        "{{ map(load_data(\"hosts.yaml\", \"{ name: string }[]\"), h => h. }}\n\n",
        "{{ map(load_data(\"hosts.yaml\", \"{ name: string }[]\", {}), h => h. }}\n\n",
        "{{ map(load_data(\"rows.csv\", \"{ name: string }[]\", {delimiter: \";\"}), h => h. }}\n",
    );
    let mut session = Session::open(&workspace, text);

    assert_eq!(member_labels(&mut session, text, "\"), h => h."), ["h.name"]);
    assert_eq!(member_labels(&mut session, text, "{}), h => h."), ["h.name"]);
    assert_eq!(member_labels(&mut session, text, "\";\"}), h => h."), ["h.name"]);

    session.shutdown();
}

#[test]
fn both_arities_keep_null_file_typing() {
    let workspace = LspWorkspace::new();
    let text = concat!(
        "# Doc\n\n",
        "{{ map(load_data(null, \"{ name: string }[]\"), h => h. }}\n\n",
        "{{ map(load_data(null, \"{ name: string }[]\", {format: \"json\"}), h => h. }}\n\n",
        "{{ map(load_data(f, \"{ name: string }[]\"), h => h. }}\n\n",
        "{{ map(load_data(f, \"{ name: string }[]\", opts), h => h. }}\n",
    );
    let mut session = Session::open(&workspace, text);

    let null_two = member_labels(&mut session, text, "null, \"{ name: string }[]\"), h => h.");
    let null_three = member_labels(&mut session, text, "{format: \"json\"}), h => h.");
    assert_eq!(null_two, null_three);
    let computed_two = member_labels(&mut session, text, "(f, \"{ name: string }[]\"), h => h.");
    let computed_three = member_labels(&mut session, text, "opts), h => h.");
    assert_eq!(computed_two, computed_three);
    assert_eq!(computed_three, ["h.name"], "a possibly-null file keeps the contract's members");

    session.shutdown();
}
