//! Defect D — HTML code blocks invert their light/dark theme variant for page
//! contrast, matching the terminal renderer (code-blocks-only). A dark page
//! emits a *light* code panel and vice versa; prose is unaffected.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{CodeHighlighter, ColorMode, ThemePair};
use darkmatter::markdown::output::{HtmlOptions, as_html};

/// `#rrggbb` background of a resolved (theme-pair, mode) pair.
fn theme_bg_hex(pair: ThemePair, mode: ColorMode) -> String {
    let bg = CodeHighlighter::new(pair, mode)
        .theme()
        .settings
        .background
        .expect("theme has a background");
    format!("#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b)
}

fn github_html(mode: ColorMode) -> String {
    let md: Markdown = "```rust\nfn main() {}\n```\n".into();
    let mut opts = HtmlOptions::default();
    opts.code_theme = ThemePair::Github;
    opts.prose_theme = ThemePair::Github;
    opts.color_mode = mode;
    as_html(&md, opts).expect("as_html")
}

#[test]
fn html_code_block_inverts_for_dark_page() {
    let html = github_html(ColorMode::Dark);
    let light = theme_bg_hex(ThemePair::Github, ColorMode::Light);
    let dark = theme_bg_hex(ThemePair::Github, ColorMode::Dark);

    assert!(
        html.contains(&format!("background-color: {light};")),
        "dark page should emit the LIGHT github code background {light}"
    );
    assert!(
        !html.contains(&format!("background-color: {dark};")),
        "dark page must not emit the dark github code background {dark}"
    );
}

#[test]
fn html_code_block_inverts_for_light_page() {
    let html = github_html(ColorMode::Light);
    let dark = theme_bg_hex(ThemePair::Github, ColorMode::Dark);

    assert!(
        html.contains(&format!("background-color: {dark};")),
        "light page should emit the DARK github code background {dark}"
    );
}

#[test]
fn single_variant_theme_html_is_mode_invariant() {
    // dracula is single-variant: inversion is a no-op, so dark- and light-mode
    // HTML output are identical.
    let md: Markdown = "```rust\nfn main() {}\n```\n".into();
    let render = |mode| {
        let mut opts = HtmlOptions::default();
        opts.code_theme = ThemePair::Dracula;
        opts.prose_theme = ThemePair::Dracula;
        opts.color_mode = mode;
        as_html(&md, opts).expect("as_html")
    };
    assert_eq!(render(ColorMode::Dark), render(ColorMode::Light));
}
