//! DMLS's half of the schema-key parity fixture
//! (`claudine/lib/tests/fixtures/schema_keys/`): Claudine's registered host
//! publishes each finding `expected.json` lists, the one Claudine's embedded
//! bundle reports at run time (`claudine/lib/tests/l1/schema_key_parity.rs`):
//! the same code, severity, message, hint, and instance path, with a range
//! starting at the same authored key under UTF-8 and UTF-16 position
//! encodings, for LF and CRLF line endings.

use crate::claudine_schema_discovery::{Fixture, open, uri_of};
use crate::common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

const DOCUMENT: &str = include_str!("../../../../claudine/lib/tests/fixtures/schema_keys/sequence.md");
const EXPECTED: &str = include_str!("../../../../claudine/lib/tests/fixtures/schema_keys/expected.json");

/// The schema diagnostics DMLS publishes for `text`, negotiating `encoding`.
fn published(text: &str, encoding: &str) -> Vec<Value> {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let mut session = LspFixture::start_with_snapshot(&workspace, fixture.snapshot());
    session.initialize(json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": [encoding] }, "textDocument": {} },
        "workspaceFolders": [ { "uri": url::Url::from_directory_path(fixture.repo()).unwrap().as_str(), "name": "repo" } ]
    }));
    let uri = uri_of(&fixture.document("sequence.md", text));
    open(&session, &uri, text);
    let diagnostics = session.wait_for_diagnostics(&uri);
    session.shutdown();
    diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic["code"].as_str().is_some_and(|code| code.starts_with("dm.schema.")))
        .collect()
}

/// `expected`'s one-based line and UTF-8 byte column as an LSP start
/// position in `encoding`.
fn start(expected: &Value, encoding: &str) -> Value {
    let line = expected["line"].as_u64().unwrap() as usize - 1;
    let byte = expected["column"].as_u64().unwrap() as usize - 1;
    let text = DOCUMENT.lines().nth(line).unwrap();
    let character = match encoding {
        "utf-16" => text[..byte].encode_utf16().count(),
        _ => byte,
    };
    json!({ "line": line, "character": character })
}

fn assert_parity(encoding: &str, line_ending: &str) {
    let expected: Vec<Value> = serde_json::from_str(EXPECTED).unwrap();
    let text = DOCUMENT.replace('\n', line_ending);
    let published = published(&text, encoding);
    assert_eq!(published.len(), expected.len(), "{encoding} {line_ending:?}: {published:#?}");
    for finding in &expected {
        let diagnostic = published
            .iter()
            .find(|diagnostic| {
                diagnostic["code"] == finding["code"]
                    && diagnostic["data"]["hint"] == finding["hint"]
                    && diagnostic["data"]["instance_path"] == finding["instance_path"]
            })
            .unwrap_or_else(|| panic!("{encoding} {line_ending:?}: {finding:#} in {published:#?}"));
        // The editor keeps its own grade for an unknown frontmatter key (a
        // warning outside strict mode); every other finding has the runtime's.
        let severity = if finding["severity"] == "warning" || finding["code"] == "dm.schema.unknown_key" { 2 } else { 1 };
        assert_eq!(diagnostic["severity"], severity, "{diagnostic:#}");
        let message = format!("{}; {}", finding["message"].as_str().unwrap(), finding["hint"].as_str().unwrap());
        assert_eq!(diagnostic["message"], message.as_str(), "{diagnostic:#}");
        assert_eq!(diagnostic["range"]["start"], start(finding, encoding), "{encoding} {line_ending:?}: {diagnostic:#}");
    }
}

#[test]
fn utf8_lf_publishes_the_runtime_findings() {
    assert_parity("utf-8", "\n");
}

#[test]
fn utf8_crlf_publishes_the_runtime_findings() {
    assert_parity("utf-8", "\r\n");
}

#[test]
fn utf16_lf_publishes_the_runtime_findings() {
    assert_parity("utf-16", "\n");
}

#[test]
fn utf16_crlf_publishes_the_runtime_findings() {
    assert_parity("utf-16", "\r\n");
}
