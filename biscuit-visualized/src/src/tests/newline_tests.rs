//! The interim `\n` → `<br/>` label rewrite (`mermaid::newline`).

use std::borrow::Cow;

use crate::mermaid::newline::rewrite_label_newlines;

fn rewritten(source: &str) -> String {
    rewrite_label_newlines(source).into_owned()
}

fn assert_unchanged(source: &str) {
    let result = rewrite_label_newlines(source);
    assert!(matches!(result, Cow::Borrowed(_)), "expected no rewrite, got:\n{result}");
    assert_eq!(result, source);
}

#[test]
fn class_labels_break_at_newline() {
    let source = "classDiagram\n  class Animal {\n    +name String\n  }\n  note for Animal \"Alpha one\\nBravo two is longer\"\n  Animal --> Dog : Rel one\\nRel two longer\n";
    assert_eq!(
        rewritten(source),
        "classDiagram\n  class Animal {\n    +name String\n  }\n  note for Animal \"Alpha one<br/>Bravo two is longer\"\n  Animal --> Dog : Rel one<br/>Rel two longer\n"
    );
}

#[test]
fn er_relationship_and_attribute_comment_break_at_newline() {
    let source = "erDiagram\n  CUSTOMER ||--o{ ORDER : \"Alpha one\\nBravo two is longer\"\n  ORDER {\n    string id PK \"Comment one\\nComment two longer\"\n  }\n";
    assert_eq!(
        rewritten(source),
        "erDiagram\n  CUSTOMER ||--o{ ORDER : \"Alpha one<br/>Bravo two is longer\"\n  ORDER {\n    string id PK \"Comment one<br/>Comment two longer\"\n  }\n"
    );
}

#[test]
fn mindmap_quoted_and_bare_nodes_break_at_newline() {
    let source = "mindmap\n  root((Root))\n    [\"Alpha one\\nBravo two is longer\"]\n    Child one\\nChild two longer\n";
    assert_eq!(
        rewritten(source),
        "mindmap\n  root((Root))\n    [\"Alpha one<br/>Bravo two is longer\"]\n    Child one<br/>Child two longer\n"
    );
}

#[test]
fn requirement_text_breaks_at_newline() {
    let source = "requirementDiagram\n  requirement req1 {\n    id: 1\n    text: \"Alpha one\\nBravo two is longer\"\n    risk: high\n    verifymethod: test\n  }\n";
    assert_eq!(
        rewritten(source),
        "requirementDiagram\n  requirement req1 {\n    id: 1\n    text: \"Alpha one<br/>Bravo two is longer\"\n    risk: high\n    verifymethod: test\n  }\n"
    );
}

#[test]
fn class_diagram_v2_header_is_recognized() {
    assert_eq!(rewritten("classDiagram-v2\n  A --> B : a\\nb\n"), "classDiagram-v2\n  A --> B : a<br/>b\n");
}

/// One input per family with `\n` in every construct the rewrite must leave
/// alone, plus one label that must change, so the test fails if the family is
/// skipped altogether.
#[test]
fn class_excluded_constructs_are_untouched() {
    let source = concat!(
        "---\ntitle: Front\\nmatter\n---\n",
        "%%{init: {\"themeVariables\": {\"fontFamily\": \"a\\nb\"}}}%%\n",
        "%% a comment\\nwith newline\n",
        "classDiagram\n",
        "  %%{init: {\n  \"x\": \"y\\nz\"\n  }}%%\n",
        "  %% body comment\\nhere\n",
        "  click Animal href \"https://example.com/a\\nb\" \"Tool\\ntip\"\n",
        "  click Animal call cb(\"a\\nb\") \"Tip\\nhere\"\n",
        "  link Animal \"https://example.com/x\\ny\" \"Tip\\nlink\"\n",
        "  callback Animal \"cb\\nfn\" \"Tip\\ncb\"\n",
        "  classDef hot fill:#f00,font-family:a\\nb\n",
        "  style Animal fill:#f00,stroke:a\\nb\n",
        "  cssClass \"Animal\" a\\nb\n",
        "  accTitle: Title\\nline\n",
        "  accDescr: Desc\\nline\n",
        "  accDescr {\n    Block\\nline one\n    Block\\nline two\n  }\n",
        "  Animal --> Dog : Rel one\\nRel two\n",
    );
    let expected = source.replace("Rel one\\nRel two", "Rel one<br/>Rel two");
    assert_eq!(rewritten(source), expected);
}

#[test]
fn er_excluded_constructs_are_untouched() {
    let source = concat!(
        "%%{init: {\"er\": {\"x\": \"a\\nb\"}}}%%\n",
        "erDiagram\n",
        "  %% comment\\nhere\n",
        "  accTitle: Title\\nline\n",
        "  accDescr {\n    Desc\\nline\n  }\n",
        "  classDef hot fill:#f00,font-family:a\\nb\n",
        "  style CUSTOMER fill:#f00,stroke:a\\nb\n",
        "  CUSTOMER ||--o{ ORDER : \"a\\nb\"\n",
    );
    let expected = source.replace("\"a\\nb\"\n", "\"a<br/>b\"\n");
    assert_eq!(rewritten(source), expected);
}

#[test]
fn mindmap_excluded_constructs_are_untouched() {
    let source = concat!(
        "%%{init: {\"mindmap\": {\"x\": \"a\\nb\"}}}%%\n",
        "mindmap\n",
        "  %% comment\\nhere\n",
        "  accTitle: Title\\nline\n",
        "  accDescr: Desc\\nline\n",
        "  root((Root))\n",
        "    Child one\\nChild two\n",
        "  classDef hot fill:#f00,font-family:a\\nb\n",
        "  style root fill:#f00,stroke:a\\nb\n",
        "  click root \"https://example.com/a\\nb\" \"Tip\\nhere\"\n",
    );
    let expected = source.replace("Child one\\nChild two", "Child one<br/>Child two");
    assert_eq!(rewritten(source), expected);
}

#[test]
fn requirement_excluded_constructs_are_untouched() {
    let source = concat!(
        "%%{init: {\"requirement\": {\"x\": \"a\\nb\"}}}%%\n",
        "requirementDiagram\n",
        "  %% comment\\nhere\n",
        "  accTitle: Title\\nline\n",
        "  accDescr {\n    Desc\\nline\n  }\n",
        "  classDef hot fill:#f00,font-family:a\\nb\n",
        "  style req1 fill:#f00,stroke:a\\nb\n",
        "  requirement req1 {\n    text: \"a\\nb\"\n  }\n",
    );
    let expected = source.replace("text: \"a\\nb\"", "text: \"a<br/>b\"");
    assert_eq!(rewritten(source), expected);
}

/// The S2 `\n` corpus input of every family outside the four; each must come
/// back byte-identical and borrowed.
#[test]
fn other_families_are_unchanged() {
    for source in [
        "flowchart LR\n  A[\"Alpha one\\nBravo two is longer\"] -->|\"Edge one\\nEdge two longer\"| B[\"Plain\"]\n",
        "stateDiagram-v2\n  state \"Alpha one\\nBravo two is longer\" as S1\n  [*] --> S1\n  S1 --> S2 : Edge one\\nEdge two longer\n",
        "kanban\n  todo[Todo]\n    t1[Alpha one\\nBravo two is longer]\n",
        "sequenceDiagram\n  participant A as Alice\n  A->>B: Alpha one\\nBravo two is longer\n  Note over A: Note one\\nNote two longer\n",
        "journey\n  title My day\n  section Section one\\nSection two longer\n  Alpha one\\nBravo two is longer: 5: Me\n",
        "timeline\n  title History\n  2020 : Alpha one\\nBravo two is longer\n",
        "architecture-beta\n  service s1(server)[Alpha one\\nBravo two is longer]\n",
        "C4Context\n  title System\n  Person(p, \"Alpha one\\nBravo two is longer\", \"Desc one\\nDesc two longer\")\n",
        "gantt\n  title Plan\n  dateFormat YYYY-MM-DD\n  section Sec one\\nSec two longer\n  Alpha one\\nBravo two is longer :a1, 2024-01-01, 30d\n",
        "quadrantChart\n  title Reach\n  \"Alpha one\\nBravo two is longer\": [0.3, 0.6]\n",
        "treemap-beta\n\"Alpha one\\nBravo two is longer\"\n    \"Leaf\": 10\n",
        "xychart-beta\n  title \"Title one\\nTitle two longer\"\n  x-axis [\"Alpha one\\nBravo two is longer\", b]\n  bar [1, 2]\n",
        "pie title Pets\n  \"Alpha one\\nBravo two is longer\" : 40\n",
        "gitGraph\n  commit id: \"Alpha one\\nBravo two is longer\"\n  commit tag: \"Tag one\\nTag two longer\"\n",
        "block-beta\n  columns 2\n  a[\"Alpha one\\nBravo two is longer\"] b[\"Plain\"]\n",
    ] {
        assert_unchanged(source);
    }
}

/// A family keyword that appears only inside a comment or directive before a
/// non-listed header must not switch the rewrite on.
#[test]
fn header_is_the_first_statement_line() {
    assert_unchanged("%% classDiagram\nflowchart LR\n  A[\"a\\nb\"]\n");
    assert_unchanged("---\ntitle: classDiagram\n---\nflowchart LR\n  A[\"a\\nb\"]\n");
    assert_eq!(
        rewritten("\n%% lead\n---\ntitle: T\n---\nerDiagram\n  A ||--o{ B : \"a\\nb\"\n"),
        "\n%% lead\n---\ntitle: T\n---\nerDiagram\n  A ||--o{ B : \"a<br/>b\"\n"
    );
}

#[test]
fn escaped_backslash_before_n_is_kept() {
    assert_eq!(
        rewritten("classDiagram\n  A --> B : path C:\\\\new and a\\nb\n"),
        "classDiagram\n  A --> B : path C:\\\\new and a<br/>b\n"
    );
}

#[test]
fn source_without_newline_escapes_is_borrowed() {
    assert_unchanged("classDiagram\n  A --> B : one line\n");
    assert_unchanged("classDiagram\n  accTitle: only here\\nexcluded\n");
    assert_unchanged("");
}

#[test]
fn crlf_line_endings_are_preserved() {
    assert_eq!(
        rewritten("mindmap\r\n  root\r\n    a\\nb\r\n"),
        "mindmap\r\n  root\r\n    a<br/>b\r\n"
    );
}
