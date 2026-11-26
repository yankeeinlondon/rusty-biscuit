//! Byte-for-byte baselines of the reference report as `md validate refs` and
//! `md graph --validate` print it: every output target that renders it (plain,
//! Markdown, JSON), the reference listing, a clean and an empty document, and a
//! missing input (JSON only: its error block wraps with the path's length),
//! with each run's exit status and both streams.
//!
//! The report's renderer is shared with other reports; these baselines pin
//! what its two original callers print, so a change to the shared renderer
//! that moves a byte of their output fails here. A baseline is a file under
//! `tests/fixtures/reference_baselines/`, with the fixture workspace written
//! `<WS>`. Set `BLESS_REFERENCE_BASELINES=1` to rewrite one after a reviewed
//! change.

use crate::common;

use common::CliProcessFixture;
use std::path::Path;

const CHILD: &str = "# Child\n\n[gone](./child-gone.md)\n";
const ROOT: &str = "# Root\n\n[gone](./gone.md)\n\n::file child.md\n\n![image](./image.png)\n[mail](mailto:a@example.com)\n";
const CLEAN: &str = "# Clean\n\n[ok](https://example.com) and [child](./child.md)\n";
const EMPTY: &str = "# Nothing to check\n\nPlain prose only.\n";

/// Runs `md` with `args` (`{root}`, `{clean}`, `{empty}`, `{missing}` name
/// the fixture documents) on a plain 80-column terminal, and the run as one
/// normalized text: exit status, stdout, stderr.
fn observe(name: &str, args: &[&str]) -> String {
    let fixture = CliProcessFixture::named(name);
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/docs/child.md", CHILD);
    let root = fixture.write_file("cwd/docs/root.md", ROOT);
    let clean = fixture.write_file("cwd/docs/clean.md", CLEAN);
    let empty = fixture.write_file("cwd/docs/empty.md", EMPTY);
    let missing = fixture.cwd().join("docs").join("missing.md");
    let resolved: Vec<String> = args
        .iter()
        .map(|arg| match *arg {
            "{root}" => root.display().to_string(),
            "{clean}" => clean.display().to_string(),
            "{empty}" => empty.display().to_string(),
            "{missing}" => missing.display().to_string(),
            other => other.to_string(),
        })
        .collect();
    let output = fixture.command_builder().plain_terminal(80, 40).build().args(&resolved).output().unwrap();
    let text = format!(
        "exit: {}\n--- stdout\n{}--- stderr\n{}",
        output.status.code().map_or_else(|| "signal".to_string(), |code| code.to_string()),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    normalize(&text, fixture.workspace_path())
}

/// `text` with every spelling of the fixture workspace written `<WS>`.
fn normalize(text: &str, workspace: &Path) -> String {
    let mut spellings = vec![workspace.to_path_buf()];
    if let Ok(canonical) = biscuit_file::canonicalize_simplified(workspace) {
        spellings.push(canonical);
    }
    let mut normalized = text.to_string();
    for spelling in spellings {
        let forward = biscuit_file::to_forward_slash_string(&spelling);
        if let Ok(uri) = url::Url::from_file_path(&spelling) {
            normalized = normalized.replace(uri.as_str(), "file://<WS>");
        }
        normalized = normalized.replace(&spelling.display().to_string(), "<WS>").replace(&forward, "<WS>");
        // JSON escapes a Windows path's separators.
        normalized = normalized.replace(&spelling.display().to_string().replace('\\', "\\\\"), "<WS>");
    }
    // No fixture output holds a backslash of its own, so any left is a
    // Windows separator (JSON-escaped or not) inside a workspace path.
    // A JSON quote escape (`\"`) is kept.
    let normalized = normalized.replace("\\\\", "/").replace("\\\"", "\u{0}").replace('\\', "/").replace('\u{0}', "\\\"");
    // A reference id starts with a hash of its document's path.
    const ID: &str = "\"id\": \"";
    let mut out = String::with_capacity(normalized.len());
    let mut rest = normalized.as_str();
    while let Some(at) = rest.find(ID) {
        let (before, after) = rest.split_at(at + ID.len());
        out.push_str(before);
        let hash = after.get(..16).filter(|hash| hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
        match hash {
            Some(_) if after[16..].starts_with(':') => {
                out.push_str("<HASH>");
                rest = &after[16..];
            }
            _ => rest = after,
        }
    }
    out.push_str(rest);
    out
}

fn assert_baseline(actual: &str, expected: &str, file: &str) {
    if std::env::var_os("BLESS_REFERENCE_BASELINES").is_some() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reference_baselines").join(file);
        std::fs::write(path, actual).unwrap();
        return;
    }
    // Normalized line endings only: a Windows checkout may carry CRLF.
    assert_eq!(actual, expected.replace("\r\n", "\n"), "reference report baseline {file} changed");
}

macro_rules! baselines {
    ($($name:ident: [$($arg:literal),+ $(,)?] => $file:literal;)+) => {$(
        #[test]
        fn $name() {
            let actual = observe(stringify!($name), &[$($arg),+]);
            assert_baseline(
                &actual,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/reference_baselines/", $file)),
                $file,
            );
        }
    )+};
}

baselines! {
    validate_refs_broken_plain: ["validate", "refs", "{root}", "--output", "plain"] => "validate_refs_broken_plain.txt";
    validate_refs_broken_markdown: ["validate", "refs", "{root}", "--output", "markdown"] => "validate_refs_broken_markdown.txt";
    validate_refs_broken_json: ["validate", "refs", "{root}", "--output", "json"] => "validate_refs_broken_json.txt";
    validate_refs_listing_plain: ["validate", "refs", "{root}", "--show-all", "--output", "plain"] => "validate_refs_listing_plain.txt";
    validate_refs_listing_json: ["validate", "refs", "{root}", "--show-all", "--output", "json"] => "validate_refs_listing_json.txt";
    validate_refs_clean_plain: ["validate", "refs", "{clean}", "--output", "plain"] => "validate_refs_clean_plain.txt";
    validate_refs_clean_json: ["validate", "refs", "{clean}", "--output", "json"] => "validate_refs_clean_json.txt";
    validate_refs_empty_plain: ["validate", "refs", "{empty}", "--output", "plain"] => "validate_refs_empty_plain.txt";
    validate_refs_empty_json: ["validate", "refs", "{empty}", "--output", "json"] => "validate_refs_empty_json.txt";
    validate_refs_missing_input_json: ["validate", "refs", "{missing}", "--output", "json"] => "validate_refs_missing_input_json.txt";
    graph_validate_broken_plain: ["graph", "{root}", "--validate", "--output", "plain"] => "graph_validate_broken_plain.txt";
    graph_validate_broken_markdown: ["graph", "{root}", "--validate", "--output", "markdown"] => "graph_validate_broken_markdown.txt";
    graph_validate_broken_json: ["graph", "{root}", "--validate", "--output", "json"] => "graph_validate_broken_json.txt";
    graph_validate_clean_plain: ["graph", "{clean}", "--validate", "--output", "plain"] => "graph_validate_clean_plain.txt";
}
