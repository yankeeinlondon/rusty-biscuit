use std::collections::HashSet;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_json::json;

use super::*;

const FAILING_DRIVE_PATH: &str = r"C:\Users\x\AppData\Local\Temp\.tmpZZZ\repo";
const UNC_HIDDEN_PATH: &str = r"\\server\share\.hidden\repo";
const COMMONMARK_PUNCTUATION: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

fn compose_body(source: &str, state: serde_json::Value) -> Markdown {
    let markdown: Markdown = source.into();
    markdown
        .compose_with(ComposeOptions::new().with_external_state(state))
        .unwrap()
        .0
}

fn paragraph_text(markdown: &str) -> String {
    let mut text = String::new();
    for event in Parser::new_ext(markdown, Options::all() - Options::ENABLE_SMART_PUNCTUATION) {
        match event {
            Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) => {}
            Event::Text(chunk) => text.push_str(&chunk),
            other => panic!("expected literal paragraph text, got {other:?} from {markdown:?}"),
        }
    }
    text
}

#[test]
fn body_scalar_paths_and_backslash_punctuation_parse_as_exact_literal_text() {
    let mut cases = vec![FAILING_DRIVE_PATH.to_string(), UNC_HIDDEN_PATH.to_string()];
    cases.extend(
        COMMONMARK_PUNCTUATION
            .chars()
            .map(|punctuation| format!(r"C:\{punctuation}segment")),
    );

    for value in cases {
        let composed = compose_body("before {{ value }} after\n", json!({"value": value}));
        assert_eq!(
            paragraph_text(composed.content()),
            format!("before {value} after"),
            "body interpolation changed the parsed scalar for {value:?}; source was {:?}",
            composed.content()
        );
    }
}

#[test]
fn inline_code_path_parses_back_to_the_exact_scalar() {
    let composed = compose_body("Path: `{{ value }}`\n", json!({"value": FAILING_DRIVE_PATH}));
    let code = Parser::new_ext(composed.content(), Options::all())
        .find_map(|event| match event {
            Event::Code(code) => Some(code.into_string()),
            _ => None,
        })
        .expect("composed output must retain an inline code span");
    assert_eq!(code, FAILING_DRIVE_PATH);
}

#[test]
fn opted_in_fenced_and_indented_code_keep_raw_interpolation_bytes() {
    for source in ["```text\n{{ value }}\n```\n", "    {{ value }}\n"] {
        let markdown: Markdown = source.into();
        let composed = markdown
            .compose_with(
                ComposeOptions::new()
                    .with_external_state(json!({"value": FAILING_DRIVE_PATH}))
                    .with_interpolate_code_blocks(true),
            )
            .unwrap()
            .0;
        let code = Parser::new_ext(composed.content(), Options::all())
            .find_map(|event| match event {
                Event::Text(text) => Some(text.into_string()),
                _ => None,
            })
            .expect("composed output must retain code-block text");
        assert_eq!(code.trim_end(), FAILING_DRIVE_PATH);
    }
}

#[test]
fn frontmatter_interpolation_keeps_raw_strings_and_native_scalar_types() {
    let markdown: Markdown = "---\nquoted: '{{ path }}'\nnative_bool: '{{ flag }}'\nnative_number: '{{ count }}'\n---\nbody\n".into();
    let composed = markdown
        .compose_with(ComposeOptions::new().with_external_state(json!({
            "path": FAILING_DRIVE_PATH,
            "flag": true,
            "count": 0,
        })))
        .unwrap()
        .0;

    assert_eq!(composed.fm_get::<String>("quoted").unwrap().as_deref(), Some(FAILING_DRIVE_PATH));
    assert_eq!(composed.fm_get::<bool>("native_bool").unwrap(), Some(true));
    assert_eq!(composed.fm_get::<i64>("native_number").unwrap(), Some(0));
}

#[test]
fn raw_markdown_explicitly_restores_authored_emphasis_and_links() {
    let authored = "**important** and [docs](https://example.com)";
    let composed = compose_body("{{ raw_markdown(value) }}\n", json!({"value": authored}));
    let events: Vec<_> = Parser::new_ext(composed.content(), Options::all()).collect();

    assert!(events.iter().any(|event| matches!(event, Event::Start(Tag::Strong))));
    assert!(events.iter().any(|event| matches!(event, Event::Start(Tag::Link { .. }))));
    assert!(events.iter().any(|event| matches!(event, Event::Text(text) if text.as_ref() == "important")));
}

#[test]
fn raw_markdown_rejects_missing_arguments() {
    let markdown: Markdown = "{{ raw_markdown() }}\n".into();
    let error = markdown
        .compose_with(ComposeOptions::new().with_fail_fast(true))
        .unwrap_err();
    assert!(error.to_string().contains("raw_markdown"));
    assert!(error.to_string().contains("1 argument"));
}

#[test]
fn shell_preflight_and_execution_share_raw_windows_shaped_command_bytes() {
    #[cfg(unix)]
    {
        let fixture = tempfile::tempdir().unwrap();
        let capture = fixture.path().join("command-argument.txt");
        let source = "::shell python3 -c \"import pathlib,sys; pathlib.Path(sys.argv[1]).write_text(sys.argv[2])\" '{{ capture }}' '{{ value }}'\n";
        let markdown: Markdown = source.into();
        let base = ComposeOptions::new().with_external_state(json!({
            "value": FAILING_DRIVE_PATH,
            "capture": capture,
        }));
        let preflight = markdown.compose_preflight(&base).unwrap();
        assert_eq!(preflight.entries.len(), 1);
        assert!(preflight.entries[0].raw_command.contains(FAILING_DRIVE_PATH));
        let approved: HashSet<String> = preflight.approval_set().into_iter().collect();
        markdown
            .compose_with(
                base.with_pre_approved_commands(approved)
                    .with_shell_policy_root(fixture.path()),
            )
            .unwrap();
        assert_eq!(std::fs::read_to_string(capture).unwrap(), FAILING_DRIVE_PATH);
    }

    #[cfg(windows)]
    {
        let source = "::shell echo '{{ value }}'\n";
        let markdown: Markdown = source.into();
        let base = ComposeOptions::new().with_external_state(json!({"value": FAILING_DRIVE_PATH}));
        let preflight = markdown.compose_preflight(&base).unwrap();
        assert_eq!(preflight.entries.len(), 1);
        assert!(preflight.entries[0].raw_command.contains(FAILING_DRIVE_PATH));
    }
}

#[test]
fn transcluded_child_path_parses_as_exact_literal_text() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("root.md");
    let child = fixture.path().join("child.md");
    std::fs::write(&root, "::file ./child.md\n").unwrap();
    std::fs::write(&child, "child={{ value }}\n").unwrap();

    let markdown = Markdown::try_from(root.as_path()).unwrap();
    let composed = markdown
        .compose_with(
            ComposeOptions::new()
                .with_source_file(&root)
                .with_external_state(json!({"value": UNC_HIDDEN_PATH})),
        )
        .unwrap()
        .0;
    assert_eq!(paragraph_text(composed.content()), format!("child={UNC_HIDDEN_PATH}"));
}

#[test]
fn zzz_probe_roundtrip() {
    for value in ["a`b","a``b","a```b","```","`","` `"," x ","  ","x"] {
        let composed = compose_body("Path: `{{ value }}` end\n", json!({"value": value}));
        let code = Parser::new_ext(composed.content(), Options::all())
            .find_map(|e| match e { Event::Code(c) => Some(c.into_string()), _ => None });
        eprintln!("CODE {value:?} => src={:?} parsed={code:?}", composed.content());
    }
    for value in ["a\nb","a\n\nb","a\n    b","a\n\tb","    x","x  ","C:\\Users\\x\\.tmpZZZ\\repo"] {
        let composed = compose_body("before {{ value }} after\n", json!({"value": value}));
        let events: Vec<_> = Parser::new_ext(composed.content(), Options::all()).collect();
        eprintln!("PROSE {value:?} => src={:?}\n   ev={events:?}", composed.content());
    }
}
