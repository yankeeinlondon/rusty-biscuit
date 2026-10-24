//! The vendored Mermaid syntax: a flattened, self-contained file that syntect
//! loads, and the highlighting a `mermaid` fence gets from it in terminal and
//! HTML output.
//!
//! Each highlighting assertion compares a token's color in a `mermaid` fence
//! with the same source in a `text` fence, so it proves the token got a scope
//! the theme colors, without pinning any theme's palette.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use darkmatter::markdown::output::{ColorDepth, HtmlOptions, TerminalOptions};
use syntect::parsing::{SyntaxDefinition, SyntaxSetBuilder};

const GANTT: &str = "%%{init: {'theme': 'forest'}}%%\ngantt\n    %% a planning comment\n    title Release plan\n    dateFormat YYYY-MM-DD\n    section Build\n    Compile :a1, 2026-01-01, 3d\n";
const FLOWCHART: &str = "flowchart LR\n    Alpha[Start] --> Beta[End]\n";
const SEQUENCE: &str = "sequenceDiagram\n    Alice->>Bob: Hello\n";

fn fenced(language: &str, source: &str) -> Markdown {
    Markdown::try_from_content(format!("```{language}\n{source}```\n")).expect("markdown")
}

fn terminal(language: &str, source: &str) -> String {
    let mut options = TerminalOptions::default();
    options.code_theme = ThemePair::OneHalf;
    options.prose_theme = ThemePair::OneHalf;
    options.color_mode = ColorMode::Dark;
    options.color_depth = Some(ColorDepth::TrueColor);
    options.max_width = Some(80);
    fenced(language, source).as_terminal(options).expect("terminal render")
}

fn html(language: &str, source: &str) -> String {
    let mut options = HtmlOptions::default();
    options.code_theme = ThemePair::OneHalf;
    options.prose_theme = ThemePair::OneHalf;
    options.color_mode = ColorMode::Dark;
    fenced(language, source).as_html(options).expect("html render")
}

/// The truecolor foreground in effect where `token` first appears.
fn terminal_color(output: &str, token: &str) -> String {
    let at = output.find(token).unwrap_or_else(|| panic!("{token:?} missing from:\n{output:?}"));
    let start = output[..at].rfind("\x1b[38;2;").unwrap_or_else(|| panic!("no foreground before {token:?}"));
    let rest = &output[start + "\x1b[38;2;".len()..];
    rest[..rest.find('m').expect("terminated SGR")].to_string()
}

/// The inline `color:` of the span holding the first `token` inside the code
/// element (the page head holds unrelated text such as `initial-scale`).
fn html_color(output: &str, token: &str) -> String {
    let output = &output[output.find("<code").expect("a code element")..];
    let at = output.find(token).unwrap_or_else(|| panic!("{token:?} missing from:\n{output}"));
    let start = output[..at].rfind("color: #").unwrap_or_else(|| panic!("no color before {token:?}"));
    output[start + "color: #".len()..start + "color: #".len() + 6].to_string()
}

/// Asserts each token renders in a different color in a `mermaid` fence than
/// in a `text` fence holding the same source, in both outputs.
fn assert_highlighted(source: &str, terminal_tokens: &[&str], html_tokens: &[&str]) {
    let (mermaid, plain) = (terminal("mermaid", source), terminal("text", source));
    for token in terminal_tokens {
        assert_ne!(
            terminal_color(&mermaid, token),
            terminal_color(&plain, token),
            "terminal: {token:?} must be highlighted in:\n{mermaid:?}"
        );
    }
    let (mermaid, plain) = (html("mermaid", source), html("text", source));
    for token in html_tokens {
        assert_ne!(
            html_color(&mermaid, token),
            html_color(&plain, token),
            "html: {token:?} must be highlighted in:\n{mermaid}"
        );
    }
}

#[test]
fn vendored_mermaid_syntax_is_flat_and_self_contained() {
    let syntax = include_str!("../../assets/syntaxes/mermaid/Mermaid.sublime-syntax");

    for line in syntax.lines() {
        let key = line.split(':').next().unwrap_or_default();
        assert_ne!(key, "extends", "the flattened syntax must not extend another: {line}");
        assert_ne!(key, "version", "the flattened syntax must be syntax version 1: {line}");
        assert!(!line.contains("meta_prepend") && !line.contains("meta_append"), "{line}");
        // A reference to another syntax would not resolve in the set Darkmatter
        // loads this file into.
        assert!(!line.contains("scope:source.") && !line.contains("scope:text."), "{line}");
    }

    let definition = SyntaxDefinition::load_from_str(syntax, true, None).expect("syntect loads the syntax");
    assert_eq!(definition.name, "Mermaid");
    assert_eq!(definition.scope.build_string(), "source.mermaid");
    assert!(definition.file_extensions.iter().any(|ext| ext == "mermaid"));
    assert!(definition.file_extensions.iter().any(|ext| ext == "mmd"));

    let mut builder = SyntaxSetBuilder::new();
    builder.add_plain_text_syntax();
    builder.add(definition);
    let set = builder.build();
    assert!(set.find_syntax_by_extension("mmd").is_some());
}

#[test]
fn mermaid_and_mmd_fences_highlight_identically() {
    let (mmd, mermaid) = (terminal("mmd", FLOWCHART), terminal("mermaid", FLOWCHART));
    for token in ["flowchart", "Alpha", "-->"] {
        assert_eq!(terminal_color(&mmd, token), terminal_color(&mermaid, token), "{token:?}");
    }
    assert_ne!(terminal_color(&mmd, "Alpha"), terminal_color(&terminal("text", FLOWCHART), "Alpha"));
}

#[test]
fn mermaid_gantt_highlights_its_header_comment_and_init_directive() {
    assert_highlighted(GANTT, &["gantt", "a planning comment", "init"], &["gantt", "a planning comment", "init"]);
}

#[test]
fn mermaid_flowchart_highlights_nodes_and_arrows() {
    assert_highlighted(FLOWCHART, &["flowchart", "Alpha", "-->"], &["flowchart", "Alpha", "--&gt;"]);
}

#[test]
fn mermaid_sequence_highlights_participants_and_arrows() {
    assert_highlighted(SEQUENCE, &["sequenceDiagram", "Alice", "->>"], &["sequenceDiagram", "Alice", "-&gt;&gt;"]);
}
