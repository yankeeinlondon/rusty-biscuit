mod common;

use common::{md_cmd, md_file};
use common::layout::*;
use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;

#[test]
fn style_fixture_cli_pipe_smoke_passes() {
    // Smoke check that `md style-prop.md` exits successfully and emits a
    // non-empty stdout when stdout is a pipe (the CLI test runner captures
    // stdout, so `OutputFormat::Auto` takes the markdown pass-through path
    // here — this test does NOT exercise the terminal renderer). Terminal
    // layout coverage lives in the Level 2 WezTerm pane tests
    // (`darkmatter/cli/tests/level2_layout.rs::level2_style_fixture_*`).
    let output = md_cmd().arg(style_prop_fixture()).output().unwrap();
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
    // Acceptance: `md --output html style-prop.md` uses the same page-level
    // frontmatter values through `render_to_browser`. MD_DRY_RUN avoids
    // launching a browser.
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_fixture_strict_style_passes_on_schema_clean_doc() {
    // The fixture only generates `KnownButInactive` warnings (the ul / ol
    // keys are wired in later sub-specs). `--strict-style` must NOT fail on
    // `KnownButInactive`.
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--strict-style")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "--strict-style must succeed on schema-clean fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_strict_style_fails_on_unknown_key() {
    // Spec test #5: `--strict-style` fails on `UnknownKey`. We route through
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

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on unknown key"
    );
}

#[test]
fn style_strict_style_fails_on_deprecated_key() {
    // `--strict-style` promotes `Deprecated` warnings to errors. The
    // canonical key is `style.page.left-margin`; the alias
    // `style.page.left_margin` should trigger a Deprecated warning, which
    // strict mode turns into an error. Route through `--output html` to
    // exercise the frontmatter pipeline.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   page:\n\
        \x20       left_margin: 2ch\n\
        ---\n\n# Doc\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on deprecated snake-case alias"
    );
}

#[test]
fn style_strict_style_fails_on_top_level_hr_alias() {
    // Sub-spec #6: `--strict-style` must reject the deprecated top-level
    // `hr:` block, even when its contents would type-check as `HrStyle`.
    let tmp = md_file(
        "---\n\
        hr:\n\
        \x20   style: waves\n\
        ---\n\n# Doc\n\n---\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on top-level `hr:` alias"
    );
}

#[test]
fn style_strict_style_fails_on_top_level_hr_alias_with_invalid_field() {
    // `alignment: true` cannot deserialize as `HrAlignment`. The alias
    // presence — not typed migration success — is what must be rejected.
    let tmp = md_file(
        "---\n\
        hr:\n\
        \x20   style: waves\n\
        \x20   alignment: true\n\
        ---\n\n# Doc\n\n---\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on top-level `hr:` alias even when the \
         typed migration would drop fields"
    );
}

#[test]
fn style_strict_style_fails_on_non_mapping_top_level_hr() {
    // A scalar `hr: dashes` is still alias usage and must trip strict mode.
    let tmp = md_file(
        "---\n\
        hr: dashes\n\
        ---\n\n# Doc\n\n---\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on non-mapping top-level `hr:`"
    );
}

#[test]
fn style_non_strict_renders_with_unknown_key() {
    // Without `--strict-style`, an unknown key must NOT fail the render; it
    // becomes an informational warning. Route through `--output html` to
    // exercise the frontmatter pipeline.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   page:\n\
        \x20       made-up-key: 2ch\n\
        ---\n\n# Doc\n",
    );
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "unknown key without --strict-style must still render: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_cli_margin_overrides_frontmatter() {
    // Spec test #2: CLI flag overrides frontmatter. The fixture has
    // `left-margin: 2ch`; `--ml 7` claims that field via
    // `PageStyleOverrides::margin_left = true`, so the CLI value (7) wins.
    let page = {
        // Mirror the cli's parse → apply_cli → apply_style_frontmatter
        // pipeline as in the public API.
        use darkmatter::markdown::Markdown;
        use darkmatter::style::{PageStyleOverrides, apply_page_style, from_frontmatter};
        let raw = std::fs::read_to_string(style_prop_fixture()).unwrap();
        let md = Markdown::try_from_content(&raw).unwrap();
        let (style, _) = from_frontmatter(md.frontmatter()).unwrap();

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_margin_left(7);
        let overrides = PageStyleOverrides {
            margin_left: true,
            ..PageStyleOverrides::default()
        };
        apply_page_style(page, &style, overrides).expect("apply")
    };
    assert_eq!(
        tv_cells(&page.page_margin().left),
        7,
        "CLI override must win over frontmatter left-margin"
    );
    assert_eq!(
        tv_cells(&page.page_margin().right),
        4,
        "frontmatter right-margin (4ch) must still apply when not claimed"
    );
}

// =============================================================================
