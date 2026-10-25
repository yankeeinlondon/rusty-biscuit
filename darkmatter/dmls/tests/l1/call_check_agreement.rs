//! Composition and the editor agree on proven call errors: for every
//! document of one shared corpus, `md`'s composition rejects it because of a
//! call exactly when DMLS reports a `dm.expression.incompatible_call` error.
//!
//! Each document is composed in process in the workspace and opened in one
//! real LSP session over the same files.

use crate::common;

use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};
use serde_json::{Value, json};

/// The worked example's schema, every variable set.
const FRONTMATTER: &str = "$schema:
  count: string
  maybe: number
  size: number(required)
  flag: boolean(required)
  foo:
    - string(required)
    - number(required)
count: \"3\"
maybe: 3
size: 3
flag: true
foo: 4
untyped_var: 1
";

/// `(name, extra frontmatter, body, rejected)`.
const CORPUS: &[(&str, &str, &str, bool)] = &[
    ("row_a", "", "{{ min(size, 5) }}", false),
    ("row_b", "", "{{ min(maybe, 5) }}", false),
    ("row_c", "", "{{ min(count, 5) }}", false),
    ("row_d", "", "{{ min(flag, 5) }}", true),
    ("row_e", "", "{{ min(untyped_var, 5) }}", false),
    ("row_f", "", "{{ min(length(count), 5) }}", false),
    ("row_g", "", "{{ is_number(foo) ? min(foo, 5) : 0 }}", false),
    ("row_h", "", "{{ is_string(foo) ? 0 : min(foo, 5) }}", false),
    ("row_i", "", "{{ min(maybe || 0, 5) }}", false),
    ("dead_ternary", "", "{{ true ? 'ok' : min(flag, 5) }}", true),
    ("dead_fallback", "", "{{ size || min(flag, 5) }}", true),
    ("dead_and", "", "{{ and(false, min(flag, 5)) }}", true),
    ("false_block", "", "::block when=\"false\"\n{{ min(flag, 5) }}\n::end-block", true),
    ("block_condition", "", "::block when=\"min(flag, 5) > 1\"\nyes\n::end-block", true),
    ("fm_namespace", "", "{{ min(fm.flag, 5) + min(fm['size'], 5) }}", true),
    ("frontmatter_value", "label: \"{{ min(flag, 5) }}\"\n", "body", true),
    ("frontmatter_fine", "label: \"{{ min(size, 5) }}\"\n", "body", false),
    ("arity", "", "{{ min(1) }}", true),
    ("recovery", "", "{{ number('pear', 7) }} {{ round(flag, 7) }}", false),
    ("bad_fallback", "", "{{ number('4', 'x') }}", true),
    ("literal_bad", "", "{{ number('pear') }}", true),
    ("fenced", "", "```text\n{{ min(flag, 5) }}\n```", false),
    ("literal_braces", "", "{{{ min(flag, 5) }}}", false),
    ("file_exists_number", "", "{{ file_exists(42) }}", false),
];

/// Retained text: a `boolish` may hold `"1"` and a `numberlike` always
/// admits as a number, so neither is a proven error; a `boolean` still is.
const RETAINED_FRONTMATTER: &str = "$schema:
  word: boolish(required)
  amount: numberlike(required)
  switch: boolean(required)
word: \"1\"
amount: \"4\"
switch: true
";

const RETAINED_CORPUS: &[(&str, &str, &str, bool)] = &[
    ("boolish_min", "", "{{ min(word, 5) }}", false),
    ("boolish_length", "", "{{ length(word) }}", false),
    ("numberlike_min", "", "{{ min(amount, 5) }}", false),
    ("boolean_length", "", "{{ length(switch) }}", true),
];

fn document(frontmatter: &str, extra: &str, body: &str) -> String {
    format!("---\n{frontmatter}{extra}---\n\n{body}\n")
}

fn composition_rejects(workspace: &LspWorkspace, path: &std::path::Path) -> bool {
    let dir = workspace.path();
    let options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.to_path_buf());
    let snapshot = RequestSnapshot::new(dir).with_home(Some(dir.join("home")));
    let request = ComposeRequest::prepare(options, &snapshot).expect("request");
    match Markdown::try_from(path).expect("document loads").compose_with(&request) {
        Ok(_) => false,
        Err(error) => error.rejected_call().is_some(),
    }
}

fn editor_rejects(diagnostics: &[Value]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic["code"] == json!("dm.expression.incompatible_call") && diagnostic["severity"] == json!(1))
}

macro_rules! agreement_groups {
    ($($name:ident: $start:literal..$end:literal),+ $(,)?) => {
        const AGREEMENT_GROUPS: &[(usize, usize)] = &[$(($start, $end)),+];

        $(
            #[test]
            fn $name() {
                agree(FRONTMATTER, &CORPUS[$start..$end]);
            }
        )+
    };
}

agreement_groups! {
    composition_and_the_editor_reject_the_same_documents: 0..6,
    composition_and_the_editor_reject_the_same_documents_second_group: 6..12,
    composition_and_the_editor_reject_the_same_documents_third_group: 12..18,
    composition_and_the_editor_reject_the_same_documents_fourth_group: 18..24,
}

#[test]
fn every_agreement_document_is_scheduled_once_in_a_bounded_group() {
    let mut next = 0;
    for &(start, end) in AGREEMENT_GROUPS {
        assert_eq!(start, next, "groups must cover the corpus in order without gaps or overlap");
        assert!((1..=6).contains(&(end - start)), "each group must contain one to six documents");
        next = end;
    }
    assert_eq!(next, CORPUS.len(), "schedule every corpus row when adding documents");
}

#[test]
fn composition_and_the_editor_agree_on_retained_text() {
    agree(RETAINED_FRONTMATTER, RETAINED_CORPUS);
}

/// Composes and opens every document of `corpus` under `frontmatter`, and
/// fails on any document either side judges differently from its row.
fn agree(frontmatter: &str, corpus: &[(&str, &str, &str, bool)]) {
    let workspace = LspWorkspace::new();
    let root_uri = url::Url::from_directory_path(workspace.path()).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": ["utf-8"] } },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "agreement" } ]
    }));

    let mut disagreements = Vec::new();
    for (name, extra, body, rejected) in corpus {
        let text = document(frontmatter, extra, body);
        let path = workspace.path().join(format!("{name}.md"));
        std::fs::write(&path, &text).unwrap();
        let composed = composition_rejects(&workspace, &path);

        let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
        fixture.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
        );
        fixture.flush_server();
        let mut latest = fixture.wait_for_diagnostics_params(&uri);
        while let Some(params) = fixture.take_buffered_diagnostics(&uri) {
            latest = params;
        }
        let edited = editor_rejects(latest["diagnostics"].as_array().map_or(&[][..], Vec::as_slice));

        if composed != *rejected || edited != *rejected {
            disagreements.push(format!("{name}: expected {rejected}, composition {composed}, editor {edited}"));
        }
    }
    fixture.shutdown();
    assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}
