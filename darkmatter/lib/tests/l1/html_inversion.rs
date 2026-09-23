//! Defect D — HTML code blocks invert their light/dark theme variant for page
//! contrast, matching the terminal renderer (code-blocks-only). A dark page
//! emits a *light* code panel and vice versa; prose is unaffected.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{CodeHighlighter, ColorMode, ThemePair};
use darkmatter::markdown::output::HtmlOptions;

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
    md.as_html(opts).expect("as_html")
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
fn theme_pair_code_panel_inverts_with_page_mode() {
    // Every `ThemePair` resolves to both a light and a dark variant; Dracula's
    // pairing is dark=Dracula, light=OneHalfLight. With the default `Inverse`
    // code-block mode the panel takes the opposite variant of the page, so a
    // dark page yields the light variant and a light page the dark one — the
    // two renders differ. No `ThemePair` is mode-invariant.
    let md: Markdown = "```rust\nfn main() {}\n```\n".into();
    let render = |mode| {
        let mut opts = HtmlOptions::default();
        opts.code_theme = ThemePair::Dracula;
        opts.prose_theme = ThemePair::Dracula;
        opts.color_mode = mode;
        md.as_html(opts).expect("as_html")
    };

    let dark_variant = theme_bg_hex(ThemePair::Dracula, ColorMode::Dark);
    let light_variant = theme_bg_hex(ThemePair::Dracula, ColorMode::Light);

    let dark_page = render(ColorMode::Dark);
    assert!(
        dark_page.contains(&format!("background-color: {light_variant};")),
        "dark page should invert to the light variant panel {light_variant}"
    );

    let light_page = render(ColorMode::Light);
    assert!(
        light_page.contains(&format!("background-color: {dark_variant};")),
        "light page should invert to the dark variant panel {dark_variant}"
    );

    assert_ne!(dark_page, light_page);
}
