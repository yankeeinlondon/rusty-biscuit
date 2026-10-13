//! Control characters in source-derived values (paths, docs, signatures)
//! stay inert on stdout and stderr in every export format: no raw C0 byte
//! but the line feeds (and the tabs) a format writes itself, no C1 control,
//! no DEL. Hostile file names are Unix-only (Windows rejects LF and ESC in a
//! name); hostile file contents run everywhere.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use super::*;

/// Java whose docs and signature comment carry ESC/OSC/C1/BEL controls.
const HOSTILE_CONTENTS: &str = "/** Doc \u{1b}[31mred\u{9b}31m bell\u{7} end */\nclass Ok {\n    \
    /** Line one\u{85} next \u{1b}]8;;http://x\u{7}link\u{1b}\\ */\n    \
    void call(/* \u{1b}[2J */ int value) {}\n}\n";

/// The review's newline + blank line + indented-code filename.
#[cfg(unix)]
const INJECTED: &str = "prefix\n\n        injected.java";
/// A C1 CSI and an ESC CSI in one filename.
#[cfg(unix)]
const CSI: &str = "bad\u{9b}31m\u{1b}[31mred.java";

/// Every invocation under test: the four export formats (Prose in both
/// presentations) plus the existing listings for comparison.
const FORMATS: &[&[&str]] = &[
    &["symbols", "--format", "tsv", "--signatures", "full"],
    &["symbols", "--format", "markdown", "--signatures", "full", "--comments"],
    &["symbols", "--format", "prose", "--signatures", "full", "--comments"],
    &["--plain", "symbols", "--format", "prose", "--signatures", "full", "--comments"],
    &["symbols", "--format", "ctags"],
    &["symbols", "--format", "plain", "--signatures", "full"],
    &["symbols", "--format", "table"],
];

/// Fails on a control character the format did not write itself.
fn assert_inert(bytes: &[u8], tabs: bool, context: &str) {
    let text = std::str::from_utf8(bytes).unwrap_or_else(|error| panic!("{context}: not UTF-8: {error}"));
    for (offset, character) in text.char_indices() {
        let allowed = character == '\n' || (tabs && character == '\t');
        assert!(
            !character.is_control() || allowed,
            "{context}: raw {character:?} at byte {offset} in:\n{}",
            text.escape_debug()
        );
    }
}

fn is_tabular(args: &[&str]) -> bool {
    args.iter().any(|arg| matches!(*arg, "tsv" | "ctags"))
}

/// The Markdown events, failing on any block or inline a source value could
/// have opened (code block, HTML, link, image, inline code).
fn assert_inert_markdown(markdown: &str) -> Vec<String> {
    let mut texts = Vec::new();
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock | Tag::Link { .. } | Tag::Image { .. })
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::Code(_) => panic!("unexpected Markdown event {event:?} in:\n{markdown}"),
            Event::Text(text) => texts.push(text.into_string()),
            _ => {}
        }
    }
    texts
}

#[test]
fn hostile_docs_and_signatures_stay_inert_in_every_format() {
    let scratch = Scratch::new(&[("Ok.java", HOSTILE_CONTENTS)]);
    for args in FORMATS {
        let context = args.join(" ");
        let output = scratch.hug(args);
        assert!(output.status.success(), "{context}: {}", String::from_utf8_lossy(&output.stderr));
        assert_inert(&output.stdout, is_tabular(args), &context);
        assert_inert(&output.stderr, false, &context);
        assert!(String::from_utf8_lossy(&output.stdout).contains("call"), "{context}");
    }
    let markdown = scratch.stdout(FORMATS[1]);
    let texts = assert_inert_markdown(&markdown).concat();
    assert!(texts.contains(r"Doc \u{1b}[31mred\u{9b}31m bell\u{7} end"), "{texts}");
    assert!(texts.contains(r"/* \u{1b}[2J */"), "{texts}");
}

#[cfg(unix)]
#[test]
fn hostile_file_names_stay_inert_on_stdout_and_stderr_in_every_format() {
    let scratch = Scratch::new(&[
        (INJECTED, "class Injected { void run() {} }\n"),
        (CSI, "class Bad { void go() {} }\n"),
        ("Ok.java", "class Ok { void call() {} }\n"),
    ]);
    for args in FORMATS {
        let context = args.join(" ");
        let output = scratch.hug(args);
        let ctags = args.contains(&"ctags");
        assert_eq!(output.status.code(), Some(i32::from(ctags)), "{context}: {}", String::from_utf8_lossy(&output.stderr));
        assert_inert(&output.stdout, is_tabular(args), &context);
        assert_inert(&output.stderr, false, &context);
        let stdout = String::from_utf8(output.stdout).expect("utf-8");
        if !ctags {
            assert!(output.stderr.is_empty(), "{context}: {}", String::from_utf8_lossy(&output.stderr));
            assert!(stdout.contains("Injected") && stdout.contains("Bad"), "{context}:\n{stdout}");
        }
    }
}

#[cfg(unix)]
#[test]
fn ctags_warnings_for_hostile_paths_are_one_visible_line_per_skipped_tag() {
    let scratch = Scratch::new(&[
        (INJECTED, "class Injected { void run() {} }\n"),
        (CSI, "class Bad { void go() {} }\n"),
        ("Ok.java", "class Ok { void call() {} }\n"),
    ]);
    let output = scratch.hug(&["symbols", "--format", "ctags"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let stderr = String::from_utf8(output.stderr).expect("utf-8");
    for omitted in ["Injected", "run", "Bad", "go"] {
        assert!(!stdout.lines().any(|tag| tag.starts_with(&format!("{omitted}\t"))), "{omitted}:\n{stdout}");
    }
    assert!(stdout.contains("call\tOk.java\t1;\"\tkind:method\tclass:Ok"), "{stdout}");
    let warnings: Vec<&str> = stderr.lines().collect();
    let injected = r"prefix\u{a}\u{a}        injected.java";
    let csi = r"bad\u{9b}31m\u{1b}[31mred.java";
    let expected: Vec<String> = [(csi, 1), (csi, 1), (injected, 1), (injected, 1)]
        .iter()
        .map(|(path, line)| format!("warning: ctags skipped unsupported control character in {path} at line {line}"))
        .collect();
    let mut sorted = warnings.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, expected, "{stderr}");
}

#[cfg(unix)]
#[test]
fn markdown_newline_file_name_is_text_inside_its_item_not_a_code_block() {
    let scratch = Scratch::new(&[(INJECTED, "class Injected { void run() {} }\n")]);
    let markdown = scratch.stdout(&["symbols", "--format", "markdown"]);
    // pulldown-cmark splits text at backslash escapes, so an item's text is
    // the concatenation of its text events.
    let mut items: Vec<String> = Vec::new();
    let mut depth = 0usize;
    for event in Parser::new(&markdown) {
        match event {
            Event::Start(Tag::Item) => {
                depth += 1;
                items.push(String::new());
            }
            Event::End(TagEnd::Item) => depth -= 1,
            Event::Start(Tag::CodeBlock(_)) => panic!("code block in:\n{markdown}"),
            Event::Text(text) => {
                assert!(depth > 0 || !text.contains("injected"), "file name outside an item:\n{markdown}");
                items.last_mut().expect("text after an item").push_str(&text);
            }
            _ => {}
        }
    }
    let found = items.iter().any(|item| item.contains(r"prefix\u{a}\u{a}        injected.java:1:1"));
    assert!(found, "{items:?}\n{markdown}");
}

#[cfg(unix)]
#[test]
fn failed_file_errors_with_hostile_paths_are_one_visible_line() {
    // Invalid UTF-8 contents make the read fail with the path in the message.
    let scratch = Scratch::new(&[("Ok.java", "class Ok {}\n")]);
    let hostile = scratch.repo.path().join(CSI);
    std::fs::write(&hostile, b"class Bad {}\n\xff\xfe\n").expect("write invalid UTF-8");
    for args in FORMATS.iter().copied().chain([&["--json", "symbols"][..], &["symbols", "--jsonl"]]) {
        let context = args.join(" ");
        let output = scratch.hug(args);
        assert_eq!(output.status.code(), Some(1), "{context}");
        let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
        assert_inert(&output.stderr, false, &context);
        assert_eq!(stderr.lines().count(), 1, "{context}: {stderr}");
        assert!(stderr.contains(r"bad\u{9b}31m\u{1b}[31mred.java"), "{context}: {stderr}");
    }
}
