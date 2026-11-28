//! A removed key, an unknown key's hint, and near-miss advice as DMLS
//! publishes them: Simplified Schema's code, its message followed by the
//! hint, the hint in `data`, and the range of the key token in the
//! negotiated position encoding, the same with LF and CRLF line endings.
//!
//! Boundary: a real LSP session (`LspFixture`) over a temporary workspace.

use crate::common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

/// `cfg`'s `setup` sits after a two-byte `ü` on its line, so its UTF-8 and
/// UTF-16 columns differ by one.
const DOCUMENT: &str = "---\n$schema:\n  cfg:\n    ü: number\n    setup: \"removed(use a preceding `side_effect:` task)\"\n  name: string\n  $constraints: { closed: true }\nname: x\nnmae: y\ncfg: { ü: 1, setup: 2 }\n---\nBody\n";

const OPEN: &str = "---\n$schema:\n  prompt: string\n  prompts: string\n  \"<string>\": unknown\n  $advise-unknown-keys: [promptss]\npromt: x.md\npromptss: y\ncolour: blue\n---\nBody\n";

fn published(text: &str, encoding: &str) -> Vec<Value> {
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("doc.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
    let root_uri = url::Url::from_directory_path(workspace.path()).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": [encoding] } },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "keys" } ]
    }));
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
    );
    fixture.flush_server();
    let mut latest = fixture.wait_for_diagnostics_params(&uri);
    while let Some(params) = fixture.take_buffered_diagnostics(&uri) {
        latest = params;
    }
    fixture.shutdown();
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

fn by_code<'a>(diagnostics: &'a [Value], code: &str) -> &'a Value {
    diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == code)
        .unwrap_or_else(|| panic!("no `{code}` in {diagnostics:#?}"))
}

/// The `(line, character)` range of the `occurrence`-th `needle` in `text`,
/// in `encoding` units.
fn range_of(text: &str, needle: &str, occurrence: usize, encoding: &str) -> Value {
    let start = text.match_indices(needle).nth(occurrence).unwrap_or_else(|| panic!("`{needle}`")).0;
    let point = |offset: usize| {
        let before = &text[..offset];
        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        let column = &text[line_start..offset];
        let character = if encoding == "utf-8" { column.len() } else { column.encode_utf16().count() };
        json!({ "line": before.matches('\n').count(), "character": character })
    };
    json!({ "start": point(start), "end": point(start + needle.len()) })
}

macro_rules! removed_key_cases {
    ($($test:ident => ($encoding:literal, $crlf:literal);)*) => {$(
        #[test]
        fn $test() {
            let text = if $crlf { DOCUMENT.replace('\n', "\r\n") } else { DOCUMENT.to_string() };
            let diagnostics = published(&text, $encoding);

            let removed = by_code(&diagnostics, "dm.schema.removed_key");
            assert_eq!(removed["severity"], 1, "{removed:#}");
            assert_eq!(
                removed["message"],
                "key `setup` has been removed; use a preceding `side_effect:` task",
                "{removed:#}"
            );
            assert_eq!(removed["data"]["hint"], "use a preceding `side_effect:` task");
            // The second `setup` is the authored key (the first is the schema's).
            assert_eq!(removed["range"], range_of(&text, "setup", 1, $encoding), "{removed:#}");

            let unknown = by_code(&diagnostics, "dm.schema.unknown_key");
            assert!(unknown["message"].as_str().unwrap().ends_with("; did you mean `name`?"), "{unknown:#}");
            assert_eq!(unknown["data"]["hint"], "did you mean `name`?");
            assert_eq!(unknown["range"], range_of(&text, "nmae", 0, $encoding), "{unknown:#}");
        }
    )*};
}

removed_key_cases! {
    removed_key_utf8_lf => ("utf-8", false);
    removed_key_utf8_crlf => ("utf-8", true);
    removed_key_utf16_lf => ("utf-16", false);
    removed_key_utf16_crlf => ("utf-16", true);
}

#[test]
fn near_miss_advice_is_a_warning_at_the_key_and_an_excluded_key_is_silent() {
    let diagnostics = published(OPEN, "utf-16");
    let advice: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| diagnostic["code"] == "dm.schema.near_miss_key").collect();
    let [advice] = advice.as_slice() else { panic!("{diagnostics:#?}") };
    assert_eq!(advice["severity"], 2, "{advice:#}");
    assert!(advice["message"].as_str().unwrap().contains("`promt`"), "{advice:#}");
    assert_eq!(advice["data"]["hint"], "did you mean `prompt`?");
    assert_eq!(advice["range"], range_of(OPEN, "promt", 0, "utf-16"));
}

/// Completion lists a schema's declared keys but never a removed one: the key
/// is legal to name only so that its presence can be refused.
#[test]
fn completion_never_offers_a_removed_key() {
    let text = "---\n$schema:\n  name: string\n  setup: \"removed(use a preceding `side_effect:` task)\"\n  $constraints: { closed: true }\n\n---\nBody\n";
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("doc.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
    let root_uri = url::Url::from_directory_path(workspace.path()).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(json!({
        "processId": null,
        "capabilities": {},
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "keys" } ]
    }));
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
    );
    fixture.flush_server();
    let response = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 5, "character": 0 } }),
    );
    fixture.shutdown();
    let result = response.result.unwrap_or(Value::Null);
    let items = result.as_array().cloned().or_else(|| result["items"].as_array().cloned()).unwrap_or_default();
    let labels: Vec<&str> = items.iter().filter_map(|item| item["label"].as_str()).collect();
    assert!(labels.contains(&"name"), "the declared key is offered: {labels:?}");
    assert!(!labels.contains(&"setup"), "a removed key is never offered: {labels:?}");
}
