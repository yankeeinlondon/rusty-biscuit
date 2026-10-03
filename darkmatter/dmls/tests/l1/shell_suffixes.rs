//! Frontmatter `$( … )` suffixes over a full LSP conversation: completion
//! offers all five, hover describes the suffix under the cursor, and the
//! unrecognized-suffix diagnostic lists all five. Nothing is executed.

use crate::common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

const FIVE: [&str; 5] = ["::ok", "::exit-code", "::result", "::timeout:<seconds>", "::no-cache"];

fn initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": ["utf-8"] } },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "scratch" } ]
    })
}

fn open(fixture: &LspFixture<'_>, uri: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text }
        }),
    );
}

fn labels(fixture: &mut LspFixture<'_>, uri: &str, line: u32, character: u32) -> Vec<String> {
    let items = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
        .result
        .expect("completion result");
    let items = items.get("items").cloned().unwrap_or(items);
    items
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["label"].as_str().map(str::to_string))
        .filter(|label| label.starts_with("::"))
        .collect()
}

#[test]
fn suffix_completion_hover_and_diagnostic_cover_all_five() {
    let workspace = LspWorkspace::new();
    // Line 1: a value ready for a suffix; line 2: a partial `::` after a
    // result suffix (itself an empty suffix until completed); line 3: an
    // unrecognized suffix; line 4: two result suffixes.
    let text = "---\n\
                a: \"$(git status)\"\n\
                b: \"$(git status)::ok::\"\n\
                c: \"$(git status)::okay\"\n\
                d: \"$(git status)::ok::result\"\n\
                ---\n\nBody\n";
    let path = workspace.path().join("doc.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), text);

    let after_close = labels(&mut fixture, uri.as_str(), 1, "a: \"$(git status)".len() as u32);
    assert_eq!(after_close, FIVE, "every suffix after the closing parenthesis");

    let after_result = labels(&mut fixture, uri.as_str(), 2, "b: \"$(git status)::ok::".len() as u32);
    assert_eq!(
        after_result,
        ["::timeout:<seconds>", "::no-cache"],
        "a second result suffix is never offered"
    );

    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 2, "character": "b: \"$(git status)::o".len() }
            }),
        )
        .result
        .expect("hover result");
    let markdown = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(markdown.contains("`::ok`") && markdown.contains("boolean"), "{markdown}");

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    let suffix_messages: Vec<&str> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.shell.invalid_suffix"))
        .filter_map(|diagnostic| diagnostic["message"].as_str())
        .collect();
    assert_eq!(suffix_messages.len(), 3, "{diagnostics:#?}");
    assert!(suffix_messages[0].contains("Empty suffix"), "{}", suffix_messages[0]);
    assert!(suffix_messages[1].contains("`::okay`"), "{}", suffix_messages[1]);
    for message in &suffix_messages[..2] {
        for label in FIVE {
            assert!(message.contains(label), "{message}");
        }
    }
    assert!(
        suffix_messages[2].contains("`::ok`") && suffix_messages[2].contains("`::result`"),
        "{}",
        suffix_messages[2]
    );
    let lines: Vec<u64> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.shell.invalid_suffix"))
        .filter_map(|diagnostic| diagnostic["range"]["start"]["line"].as_u64())
        .collect();
    assert_eq!(lines, [2, 3, 4], "no diagnostic on a value without suffixes");

    fixture.shutdown();
}
