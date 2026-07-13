# Common recipes

## 1) Highlight to HTML (single call)

````rust
use two_face::re_exports::syntect;

pub fn highlight_html(code: &str, ext: &str) -> String {
    let syn_set = two_face::syntax::extra_newlines();
    let theme_set = two_face::theme::extra();

    let syntax = syn_set
        .find_syntax_by_extension(ext)
        .unwrap_or_else(|| syn_set.find_syntax_plain_text());

    let theme = &theme_set[two_face::theme::EmbeddedThemeName::Nord];

    syntect::html::highlighted_html_for_string(code, &syn_set, syntax, theme)
        .expect("HTML highlighting failed")
}
````

## 2) Highlight to ANSI (24-bit terminal)

Best for CLIs that print to terminals with truecolor support.

````rust
use two_face::re_exports::syntect;
use syntect::easy::HighlightLines;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

pub fn highlight_ansi(code: &str, ext: &str) -> String {
    let syn_set = two_face::syntax::extra_newlines();
    let theme_set = two_face::theme::extra();

    let syntax = syn_set
        .find_syntax_by_extension(ext)
        .unwrap_or_else(|| syn_set.find_syntax_plain_text());

    let theme = &theme_set[two_face::theme::EmbeddedThemeName::Dracula];
    let mut h = HighlightLines::new(syntax, theme);

    let mut out = String::new();
    for line in LinesWithEndings::from(code) {
        let ranges = h.highlight_line(line, &syn_set).expect("highlight_line failed");
        out.push_str(&as_24_bit_terminal_escaped(&ranges, false));
    }
    out
}
````

## 3) Detect syntax by first line (shebang) with fallback

````rust
pub fn pick_syntax<'a>(
    syn_set: &'a syntect::parsing::SyntaxSet,
    path_ext: Option<&str>,
    first_line: Option<&str>,
) -> &'a syntect::parsing::SyntaxReference {
    if let Some(fl) = first_line {
        if let Some(s) = syn_set.find_syntax_by_first_line(fl) {
            return s;
        }
    }
    if let Some(ext) = path_ext {
        if let Some(s) = syn_set.find_syntax_by_extension(ext) {
            return s;
        }
    }
    syn_set.find_syntax_plain_text()
}
````

## 4) Dockerfile / TOML / TypeScript quick picks

````rust
let docker = syn_set.find_syntax_by_extension("Dockerfile").unwrap();
let toml = syn_set.find_syntax_by_extension("toml").unwrap();
let ts = syn_set.find_syntax_by_extension("ts").unwrap();
````

## 5) Simple curated theme picker

Expose a small set of themes to users.

````rust
const SUPPORTED: &[two_face::theme::EmbeddedThemeName] = &[
    two_face::theme::EmbeddedThemeName::Nord,
    two_face::theme::EmbeddedThemeName::Dracula,
    two_face::theme::EmbeddedThemeName::SolarizedDark,
];

pub fn theme_by_index(
    themes: &syntect::highlighting::ThemeSet,
    idx: usize,
) -> &syntect::highlighting::Theme {
    let name = SUPPORTED[idx.min(SUPPORTED.len() - 1)];
    &themes[name]
}
````