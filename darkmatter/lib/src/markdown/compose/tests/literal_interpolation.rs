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

/// Content of the one code span in `markdown`, as CommonMark parses it.
fn code_span_text(markdown: &str) -> String {
    let mut spans = Parser::new_ext(markdown, Options::all()).filter_map(|event| match event {
        Event::Code(code) => Some(code.into_string()),
        _ => None,
    });
    let span = spans
        .next()
        .unwrap_or_else(|| panic!("composed output must retain an inline code span: {markdown:?}"));
    assert!(
        spans.next().is_none(),
        "an interpolated value split its span into several: {markdown:?}"
    );
    span
}

/// Compose `value` into an inline code span and return both what the composer
/// wrote and what CommonMark reads back from it.
fn code_span_round_trip(value: &str) -> (String, String) {
    let composed = compose_body("Path: `{{ value }}` end\n", json!({"value": value}));
    let source = composed.content().to_string();
    let parsed = code_span_text(&source);
    (source, parsed)
}

#[test]
fn inline_code_widens_its_fence_past_the_longest_backtick_run() {
    // A one-backtick delimiter closes on the value's own backtick, so each of
    // these values needs a strictly longer fence than the run it contains.
    for (value, expected_fence) in [("a`b", "``"), ("a``b", "```"), ("a```b", "````")] {
        let (source, parsed) = code_span_round_trip(value);
        assert_eq!(parsed, value, "source was {source:?}");
        assert_eq!(
            source,
            format!("Path: {expected_fence}{value}{expected_fence} end\n"),
            "the fence must be exactly one backtick longer than the value's longest run"
        );
    }
}

#[test]
fn inline_code_pads_values_that_touch_a_backtick_at_either_edge() {
    // Widening alone is not enough here: without the padding spaces the value's
    // leading or trailing backtick would fuse with the delimiter and change the
    // fence length CommonMark sees.
    for (value, expected_source) in [
        ("`", "Path: `` ` `` end\n"),
        ("```", "Path: ```` ``` ```` end\n"),
        ("` `", "Path: `` ` ` `` end\n"),
    ] {
        let (source, parsed) = code_span_round_trip(value);
        assert_eq!(parsed, value, "source was {source:?}");
        assert_eq!(source, expected_source);
    }
}

#[test]
fn inline_code_keeps_the_values_own_edge_spaces() {
    // CommonMark strips one space from each end of a code span whose content is
    // padded at both ends, so a value that is itself padded has to be padded
    // again to survive the round trip. Its all-space sibling is exempt from the
    // stripping rule and must therefore NOT be padded.
    for (value, expected_source) in [(" x ", "Path: `  x  ` end\n"), ("  ", "Path: `  ` end\n")] {
        let (source, parsed) = code_span_round_trip(value);
        assert_eq!(parsed, value, "source was {source:?}");
        assert_eq!(source, expected_source);
    }
}

#[test]
fn authored_code_span_padding_is_syntax_rather_than_content() {
    // The mirror of the test above: spaces the *author* wrote around the
    // expression are CommonMark's optional padding and denote nothing, so they
    // must not reappear in the value.
    let composed = compose_body("Path: ` {{ value }} ` end\n", json!({"value": "x"}));
    assert_eq!(code_span_text(composed.content()), "x");
}
