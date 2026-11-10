use crate::common;

use common::layout::*;
use common::{CliProcessFixture, MdCommandBuilder, md_file};

fn rendering_command(fixture: &CliProcessFixture) -> assert_cmd::Command {
    rendering_builder(fixture).build()
}

/// The same pinned rendering frame before `build()`, for a test that also has
/// an application input to declare.
fn rendering_builder(fixture: &CliProcessFixture) -> MdCommandBuilder<'_> {
    fixture.command_builder().plain_terminal(80, 24)
}

#[test]
fn style_fixture_cli_pipe_smoke_passes() {
    let fixture = CliProcessFixture::named("style_fixture_cli_pipe_smoke_passes");
    // Smoke check that `md style-prop.md` exits successfully and emits a
    // non-empty stdout when stdout is a pipe (the CLI test runner captures
    // stdout, so the default `terminal` target prints what `plain` prints
    // here — this test does NOT exercise a styled terminal). Terminal
    // layout coverage lives in the Level 2 WezTerm pane tests
    // (`darkmatter/cli/tests/level2_layout.rs::level2_style_fixture_*`).
    let output = rendering_command(&fixture)
        .arg(style_prop_fixture())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "rendered output must not be empty");
    assert!(
        stdout.contains("Testing the `style` Property") || stdout.contains("Testing the"),
        "rendered output should contain the page title"
    );
}

#[test]
fn style_fixture_renders_html_successfully() {
    let fixture = CliProcessFixture::named("style_fixture_renders_html_successfully");
    // Acceptance: `md --output html style-prop.md` uses the same page-level
    // frontmatter values through `render_to_browser_document` (a complete
    // standalone document). MD_DRY_RUN avoids launching a browser.
    let output = rendering_builder(&fixture)
        .application_input("MD_DRY_RUN", "1")
        .build()
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_fixture_html_document_has_non_empty_ordered_head() {
    let fixture =
        CliProcessFixture::named("style_fixture_html_document_has_non_empty_ordered_head");
    // The decorated `style-prop.md` fixture (it configures page margins) drives
    // `render_to_browser_document`'s decorated branch. The emitted standalone
    // document must carry a REAL, non-empty `<head>` — charset/viewport/title
    // then the design-token `:root` block — not the old empty `<head></head>`,
    // and its `<body>` holds the `.darkmatter-page` frame. Without `--show`, the
    // CLI prints the HTML artifact to stdout, so we assert on it directly.
    let output = rendering_command(&fixture)
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let html = String::from_utf8_lossy(&output.stdout);

    assert!(
        html.starts_with("<!DOCTYPE html><html><head>"),
        "HTML artifact must open a standalone document, got: {html}"
    );
    let head = html
        .split_once("<head>")
        .and_then(|(_, rest)| rest.split_once("</head>"))
        .map(|(head, _)| head)
        .expect("document must have a <head>…</head>");
    assert!(
        !head.is_empty(),
        "the standalone document head must NOT be empty (regression: empty <head></head>)"
    );
    let charset_at = head
        .find("<meta charset")
        .expect("head carries a charset meta");
    let root_at = head
        .find(":root")
        .expect("head carries the design-token :root block");
    assert!(
        charset_at < root_at,
        "charset/title precede the design-token block in the head, got: {head}"
    );

    let body = html
        .split_once("<body>")
        .and_then(|(_, rest)| rest.split_once("</body>"))
        .map(|(body, _)| body)
        .expect("document must have a <body>…</body>");
    assert!(
        body.contains(r#"<div class="darkmatter-page""#),
        "the decorated body holds the page frame, got: {body}"
    );
    assert!(
        !body.contains("<meta "),
        "page <meta> tags live in <head>, not <body>, got: {body}"
    );
}

#[test]
fn style_fixture_deny_style_passes_on_schema_clean_doc() {
    let fixture = CliProcessFixture::named("style_fixture_deny_style_passes_on_schema_clean_doc");
    // The fixture only generates `KnownButInactive` warnings (the ul / ol
    // keys are wired in later sub-specs). `--deny style` must NOT fail on
    // `KnownButInactive`.
    let output = rendering_command(&fixture)
        .arg(style_prop_fixture())
        .args(["--deny", "style"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "--deny style must succeed on schema-clean fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn deny_style_fails_on_unknown_key() {
    let fixture = CliProcessFixture::named("deny_style_fails_on_unknown_key");
    // Spec test #5: `--deny style` fails on `UnknownKey`. We route through
    // `--output html` so the frontmatter pipeline (which lives in the
    // terminal / HTML render paths) runs. The markdown-only artifact path
    // intentionally short-circuits to source pass-through.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   page:\n\
        \x20       made-up-key: 2ch\n\
        ---\n\n# Doc\n",
    );

    let output = rendering_builder(&fixture)
        .application_input("MD_DRY_RUN", "1")
        .build()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .args(["--deny", "style"])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--deny style must fail on unknown key"
    );
}

#[test]
fn deny_style_fails_on_deprecated_key() {
    let fixture = CliProcessFixture::named("deny_style_fails_on_deprecated_key");
    // `--deny style` promotes `Deprecated` warnings to errors. The
    // canonical key is `style.page.max-width`; the alias
    // `style.page.max_width` should trigger a Deprecated warning, which
    // strict mode turns into an error. Route through `--output html` to
    // exercise the frontmatter pipeline.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   page:\n\
        \x20       max_width: 80ch\n\
        ---\n\n# Doc\n",
    );

    let output = rendering_builder(&fixture)
        .application_input("MD_DRY_RUN", "1")
        .build()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .args(["--deny", "style"])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--deny style must fail on deprecated snake-case alias"
    );
}

/// Valid `style.markdown` cleanup rules are neither unknown nor inactive
/// render styles, so strict rendering accepts the shipped control document.
#[test]
fn deny_style_accepts_markdown_cleanup_rules() {
    let fixture = CliProcessFixture::named("deny_style_accepts_markdown_cleanup_rules");
    let document = biscuit_test_harness::manifest_dir!()
        .join("../lib/tests/fixtures/clean/style-markdown-control.md");
    let output = rendering_builder(&fixture)
        .application_input("MD_DRY_RUN", "1")
        .build()
        .arg(&document)
        .arg("--output")
        .arg("html")
        .args(["--deny", "style"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "--deny style must accept valid style.markdown rules: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// An invalid rule is a typed style error naming the rule, as for any other
/// invalid style value.
#[test]
fn style_invalid_markdown_rule_fails_with_its_path() {
    let fixture = CliProcessFixture::named("style_invalid_markdown_rule_fails_with_its_path");
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   markdown:\n\
        \x20       indentation: 3\n\
        ---\n\n# Doc\n",
    );
    let output = rendering_builder(&fixture)
        .application_input("MD_DRY_RUN", "1")
        .build()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(!output.status.success(), "an invalid rule must fail the render");
    let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("style.markdown.indentation"), "{stderr}");
}

#[test]
fn style_non_strict_renders_with_unknown_key() {
    let fixture = CliProcessFixture::named("style_non_strict_renders_with_unknown_key");
    // Without `--deny style`, an unknown key must NOT fail the render; it
    // becomes an informational warning. Route through `--output html` to
    // exercise the frontmatter pipeline.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   page:\n\
        \x20       made-up-key: 2ch\n\
        ---\n\n# Doc\n",
    );
    let output = rendering_builder(&fixture)
        .application_input("MD_DRY_RUN", "1")
        .build()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "unknown key without --deny style must still render: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn removed_page_spacing_name_warns_with_its_replacement() {
    let fixture = CliProcessFixture::named("removed_page_spacing_name_warns_with_its_replacement");
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   page:\n\
        \x20       left-margin: 2ch\n\
        ---\n\n# Doc\n",
    );
    let output = rendering_builder(&fixture)
        .application_input("MD_DRY_RUN", "1")
        .build()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("unknown style key `style.page.left-margin`; use `page.margin-left`"),
        "{stderr}"
    );
}

#[test]
fn style_setter_margin_overrides_frontmatter() {
    // Spec test #2: a setter overrides frontmatter, field by field. The fixture
    // has `margin-left: 2ch` and `margin-right: 4ch`; `style.ml=7` wins the
    // left side, and the right side the setter does not name stays the
    // document's.
    let raw = std::fs::read_to_string(style_prop_fixture()).unwrap();
    let page = common::style::styled_page(&raw, &["doc.md", "style.ml=7"], 80);
    assert_eq!(
        tv_cells(&page.page_margin().left),
        7,
        "the setter must win over frontmatter margin-left"
    );
    assert_eq!(
        tv_cells(&page.page_margin().right),
        4,
        "frontmatter margin-right (4ch) must still apply when no setter names it"
    );
}

// =============================================================================
