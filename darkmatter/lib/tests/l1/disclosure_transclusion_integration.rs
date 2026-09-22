//! Integration tests for `::file` / `::code` `disclosure="..."` transclusion
//! unification with the render-time disclosure DSL.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};
use std::fs;
use tempfile::tempdir;

#[test]
fn file_transclusion_with_disclosure_summary_emits_dsl() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    fs::write(
        &child,
        "# Child heading\n\nChild paragraph.\n",
    )
    .unwrap();
    fs::write(
        &root,
        "# Root\n\n::file child.md disclosure=\"More\"\n",
    )
    .unwrap();

    let options = ComposeOptions::new()
        .with_source_file(&root)
        .only(&[ComposeOperation::BlockTransclusion, ComposeOperation::Cleanup]);

    let md = Markdown::try_from(root.as_path()).unwrap();
    let (composed, _report) = md.compose_with(options).unwrap();
    let text = composed.content();

    assert!(
        text.contains("::disclosure"),
        "must emit ::disclosure opener: {text}"
    );
    assert!(
        text.contains("::details"),
        "must emit ::details separator: {text}"
    );
    assert!(
        text.contains("::end-disclosure"),
        "must emit ::end-disclosure closer: {text}"
    );
    assert!(
        text.contains("More"),
        "must include summary text: {text}"
    );
    assert!(
        text.contains("Child heading"),
        "must include transcluded content: {text}"
    );
    assert!(
        !text.contains("<details>"),
        "must not emit HTML details element: {text}"
    );
    assert!(
        !text.contains("<summary>"),
        "must not emit HTML summary element: {text}"
    );
}

#[test]
fn file_transclusion_with_disclosure_true_uses_default_summary() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    fs::write(&child, "# Child heading\n\nChild paragraph.\n").unwrap();
    fs::write(&root, "# Root\n\n::file child.md disclosure=true\n").unwrap();

    let options = ComposeOptions::new()
        .with_source_file(&root)
        .only(&[ComposeOperation::BlockTransclusion, ComposeOperation::Cleanup]);

    let md = Markdown::try_from(root.as_path()).unwrap();
    let (composed, _report) = md.compose_with(options).unwrap();
    let text = composed.content();

    assert!(
        text.contains("::disclosure\nDetails\n::details"),
        "disclosure=true must normalize to default 'Details' summary: {text}"
    );
    assert!(
        text.contains("Child heading"),
        "must include transcluded content: {text}"
    );
    assert!(
        !text.contains("<details>"),
        "must not emit HTML details element: {text}"
    );
}

#[test]
fn code_transclusion_with_disclosure_summary_emits_dsl() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root.md");
    let code = dir.path().join("example.rs");

    fs::write(&code,
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .unwrap();
    fs::write(
        &root,
        "# Root\n\n::code example.rs disclosure=\"Implementation\"\n",
    )
    .unwrap();

    let options = ComposeOptions::new()
        .with_source_file(&root)
        .only(&[ComposeOperation::CodeTransclusion, ComposeOperation::Cleanup]);

    let md = Markdown::try_from(root.as_path()).unwrap();
    let (composed, _report) = md.compose_with(options).unwrap();
    let text = composed.content();

    assert!(
        text.contains("::disclosure"),
        "must emit ::disclosure opener: {text}"
    );
    assert!(
        text.contains("::details"),
        "must emit ::details separator: {text}"
    );
    assert!(
        text.contains("::end-disclosure"),
        "must emit ::end-disclosure closer: {text}"
    );
    assert!(
        text.contains("Implementation"),
        "must include summary text: {text}"
    );
    assert!(
        text.contains("fn main()"),
        "must include transcluded code: {text}"
    );
    assert!(
        !text.contains("<details>"),
        "must not emit HTML details element: {text}"
    );
    assert!(
        !text.contains("<summary>"),
        "must not emit HTML summary element: {text}"
    );
}

#[test]
fn code_transclusion_with_disclosure_true_uses_default_summary() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root.md");
    let code = dir.path().join("example.rs");

    fs::write(&code,
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .unwrap();
    fs::write(&root,
        "# Root\n\n::code example.rs disclosure=true\n",
    )
    .unwrap();

    let options = ComposeOptions::new()
        .with_source_file(&root)
        .only(&[ComposeOperation::CodeTransclusion, ComposeOperation::Cleanup]);

    let md = Markdown::try_from(root.as_path()).unwrap();
    let (composed, _report) = md.compose_with(options).unwrap();
    let text = composed.content();

    assert!(
        text.contains("::disclosure\nDetails\n::details"),
        "disclosure=true must normalize to default 'Details' summary: {text}"
    );
    assert!(
        text.contains("fn main()"),
        "must include transcluded code: {text}"
    );
    assert!(
        !text.contains("<details>"),
        "must not emit HTML details element: {text}"
    );
}
