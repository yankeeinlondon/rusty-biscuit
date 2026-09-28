use super::*;
use darkmatter::markdown::hash::restore_properties_text;
use darkmatter::markdown::literal_token::decode_literal_tokens;
use serde_json::json;

/// The pre-run document every repair case edits.
const ORIGINAL: &str = "---\nprompt: write it\ntitle: t\nauthored: kept # a comment\n---\nOld body\n";

/// `ORIGINAL` with `line` appended to the frontmatter, as an agent would add it.
fn with_added(line: &str) -> String {
    ORIGINAL.replace("---\nOld body", &format!("{line}\n---\nOld body"))
}

fn parsed(document: &str) -> Value {
    parse_frontmatter(document).expect("the frontmatter parses")
}

// -- repair -----------------------------------------------------------------

/// The input-shape matrix for one agent-added top-level value, walked from the
/// same fixture with one edit per row. `Some(text)` is the string the value
/// must read as after repair (quoted when needed); `None` means the line is
/// left byte-for-byte.
#[test]
fn repair_walks_every_value_shape_from_one_fixture() {
    // Control: the unedited fixture is returned unchanged.
    assert_eq!(repair_agent_frontmatter(ORIGINAL, ORIGINAL).unwrap(), ORIGINAL);

    let rows: &[(&str, Option<&str>, Value)] = &[
        // The spec's repair cases.
        ("title2: Fix: colons", Some("\"Fix: colons\""), json!("Fix: colons")),
        ("note: see issue #42", Some("\"see issue #42\""), json!("see issue #42")),
        ("tabbed: see\t#42", Some("\"see\t#42\""), json!("see\t#42")),
        ("ends: with colon:", Some("\"with colon:\""), json!("with colon:")),
        ("alias: *not an alias*", Some("\"*not an alias*\""), json!("*not an alias*")),
        ("tick: `code`", Some("\"`code`\""), json!("`code`")),
        ("bang: !important", Some("\"!important\""), json!("!important")),
        ("dash: - item", Some("\"- item\""), json!("- item")),
        ("broken: \"half\" quoted", Some("\"\\\"half\\\" quoted\""), json!("\"half\" quoted")),
        ("path: C:\\dir: x", Some("\"C:\\\\dir: x\""), json!("C:\\dir: x")),
        // Already valid YAML: never touched.
        ("summary: fixed {{…}} parsing", None, json!("fixed {{…}} parsing")),
        ("cmd: \"$(echo X)\"", None, json!("$(echo X)")),
        ("single: 'a: b'", None, json!("a: b")),
        ("count: 42", None, json!(42)),
        ("count: 42 # the answer", None, json!(42)),
        ("ratio: -1.5e3", None, json!(-1500.0)),
        ("flag: true", None, json!(true)),
        ("nothing: null", None, Value::Null),
        ("tilde: ~", None, Value::Null),
        ("empty:", None, Value::Null),
        ("list: [a, b]", None, json!(["a", "b"])),
        ("map: {k: v}", None, json!({"k": "v"})),
        ("block: |\n  line one: x\n  line two #3", None, json!("line one: x\nline two #3\n")),
        ("nested:\n  deep: a: b", None, Value::Null),
    ];
    for (line, repaired_value, reads_as) in rows {
        let candidate = with_added(line);
        let key = line.split(':').next().unwrap();
        let result = repair_agent_frontmatter(&candidate, ORIGINAL);
        if *line == "nested:\n  deep: a: b" {
            // A multi-line node is outside the repair case; its bad nesting
            // is reported, not guessed at.
            let rejection = result.expect_err("bad nesting is refused");
            assert_eq!(rejection.line, Some(6), "{rejection}");
            assert!(rejection.agent_edit, "{rejection}");
            continue;
        }
        let repaired = result.unwrap_or_else(|error| panic!("{line:?}: {error}"));
        match repaired_value {
            Some(quoted) => {
                assert!(
                    repaired.contains(&format!("{key}: {quoted}\n")),
                    "{line:?} repaired to:\n{repaired}"
                );
            }
            None => assert_eq!(repaired, candidate, "{line:?} must be left alone"),
        }
        assert_eq!(parsed(&repaired)[key], *reads_as, "{line:?}");
        // Unchanged author lines, comments included, keep their bytes.
        assert!(repaired.contains("authored: kept # a comment\n"), "{repaired}");
    }
}

#[test]
fn an_unchanged_key_is_never_rewritten_even_when_it_would_qualify() {
    let original = "---\nnote: see issue #42\n---\nOld\n";
    let candidate = "---\nnote: see issue #42\nextra: x\n---\nNew\n";
    let repaired = repair_agent_frontmatter(candidate, original).unwrap();
    assert_eq!(repaired, candidate, "the authored comment stays a comment");
    assert_eq!(parsed(&repaired)["note"], json!("see issue"));
}

#[test]
fn a_changed_key_is_repaired_and_an_owned_property_is_not() {
    let original = "---\nprompt: p\nnote: old\n---\nOld\n";
    let candidate = "---\nprompt: now: broken\nnote: new: text\n---\nNew\n";
    let error = repair_agent_frontmatter(candidate, original)
        .expect_err("the owned `prompt` is never repaired, so it cannot parse");
    assert_eq!(error.line, Some(2), "{error}");
    assert!(error.agent_edit);

    let candidate = "---\nprompt: p\nnote: new: text\n---\nNew\n";
    let repaired = repair_agent_frontmatter(candidate, original).unwrap();
    assert_eq!(parsed(&repaired)["note"], json!("new: text"));
}

#[test]
fn duplicate_keys_name_the_line_and_the_agent() {
    let candidate = with_added("title: again");
    let error = repair_agent_frontmatter(&candidate, ORIGINAL).expect_err("duplicate key");
    assert_eq!(error.line, Some(5), "{error}");
    assert!(error.agent_edit, "{error}");
    assert!(error.reason.contains("more than once"), "{error}");
}

#[test]
fn a_missing_closing_delimiter_is_refused() {
    let candidate = "---\nprompt: write it\ntitle: t\nNew body\n";
    let error = repair_agent_frontmatter(candidate, ORIGINAL).expect_err("no closing ---");
    assert_eq!(error.line, Some(1));
    assert!(error.agent_edit);
    // A document that never had frontmatter is not a repair case.
    assert_eq!(repair_agent_frontmatter("Just body\n", "Old\n").unwrap(), "Just body\n");
}

#[test]
fn crlf_line_endings_survive_a_repair() {
    let original = "---\r\nprompt: p\r\ntitle: t\r\n---\r\nOld\r\n";
    let candidate = "---\r\nprompt: p\r\ntitle: t\r\nnote: see issue #42\r\nhead: Fix: colons\r\n---\r\nNew\r\n";
    let repaired = repair_agent_frontmatter(candidate, original).unwrap();
    assert_eq!(
        repaired,
        "---\r\nprompt: p\r\ntitle: t\r\nnote: \"see issue #42\"\r\nhead: \"Fix: colons\"\r\n---\r\nNew\r\n"
    );
}

#[test]
fn a_block_scalar_keeps_its_formatting() {
    let original = "---\nprompt: p\n---\nOld\n";
    let candidate = "---\nprompt: p\nsummary: |\n    four spaces: kept\n\n    # not a comment\n---\nNew\n";
    let repaired = repair_agent_frontmatter(candidate, original).unwrap();
    assert_eq!(repaired, candidate);
}

// -- encode -----------------------------------------------------------------

/// Runs restore + encode as the closure does, returning the encoded text.
fn encode_after_restore(candidate: &str, original: &str) -> String {
    let restored = restore_properties_text(candidate, original, CLOSURE_OWNED_PROPERTIES).unwrap();
    encode_agent_values(&restored.text, &restored.frontmatter_delta).unwrap_or_else(|error| panic!("{error:?}"))
}

#[test]
fn only_owned_values_that_could_instruct_become_tokens() {
    let original = "---\nprompt: p\narea_note: \"in {{ area }}\"\nstatus: draft\n---\nOld\n";
    let candidate = "---\nprompt: p\narea_note: \"in {{ area }}\"\nstatus: completed\nsummary: fixed {{…}} parsing\ncmd: \"$(echo X)\"\nnote: \"see issue #42\"\n---\nNew\n";
    let encoded = encode_after_restore(candidate, original);

    assert!(encoded.contains("status: completed\n"), "{encoded}");
    assert!(encoded.contains("note: \"see issue #42\"\n"), "{encoded}");
    assert!(encoded.contains("area_note: \"in {{ area }}\"\n"), "authored bytes kept:\n{encoded}");
    assert!(encoded.contains(&format!("summary: {}\n", encode_yaml_scalar("fixed {{…}} parsing"))));
    assert!(encoded.contains(&format!("cmd: {}\n", encode_yaml_scalar("$(echo X)"))));

    let decoded = decode_literal_tokens(&parsed(&encoded)).unwrap();
    assert_eq!(decoded["summary"], json!("fixed {{…}} parsing"));
    assert_eq!(decoded["cmd"], json!("$(echo X)"));
    assert_eq!(decoded["area_note"], json!("in {{ area }}"));
}

#[test]
fn a_changed_container_owns_only_its_changed_string_leaves() {
    let original = "---\nprompt: p\nmeta:\n  keep: \"{{ area }}\"\n  list:\n    - \"{{ a }}\"\n---\nOld\n";
    let candidate = "---\nprompt: p\nmeta:\n  keep: \"{{ area }}\"\n  added: new {{ x }}\n  list:\n    - \"{{ a }}\"\n    - \"$(b)\"\n  count: 3\n---\nNew\n";
    let encoded = encode_after_restore(candidate, original);
    let tree = parsed(&encoded);
    assert_eq!(tree["meta"]["keep"], json!("{{ area }}"));
    assert_eq!(tree["meta"]["list"][0], json!("{{ a }}"));
    assert_eq!(tree["meta"]["added"], json!(encode("new {{ x }}")));
    assert_eq!(tree["meta"]["list"][1], json!(encode("$(b)")));
    assert_eq!(tree["meta"]["count"], json!(3));
}

#[test]
fn a_stored_token_is_left_alone_and_a_raw_look_alike_is_encoded_once() {
    let token = encode("fixed {{…}}");
    let original = format!("---\nprompt: p\nsummary: \"{token}\"\n---\nOld\n");
    // Unchanged: the stored token keeps its bytes.
    let unchanged = format!("---\nprompt: p\nsummary: \"{token}\"\n---\nNew\n");
    assert_eq!(encode_after_restore(&unchanged, &original), unchanged);

    // The agent rewrote the token as its decoded text: encoded again, to the
    // same token.
    let rewritten = "---\nprompt: p\nsummary: fixed {{…}}\n---\nNew\n";
    let encoded = encode_after_restore(rewritten, &original);
    assert_eq!(parsed(&encoded)["summary"], json!(token));

    // A raw agent string that merely spells a token is data: it is encoded
    // from its raw value, never trusted as already encoded.
    let fresh = "---\nprompt: p\nother: \"{{!data:v1:YQ}}\"\n---\nNew\n";
    let encoded = encode_after_restore(fresh, "---\nprompt: p\n---\nOld\n");
    let decoded = decode_literal_tokens(&parsed(&encoded)).unwrap();
    assert_eq!(decoded["other"], json!("{{!data:v1:YQ}}"));
}

#[test]
fn encoding_keeps_crlf_and_replaces_a_changed_block_scalar() {
    let original = "---\r\nprompt: p\r\n---\r\nOld\r\n";
    let candidate = "---\r\nprompt: p\r\nlog: |\r\n  one {{ x }}\r\n  two\r\nafter: \"{{ y }}\"\r\n---\r\nNew\r\n";
    let encoded = encode_after_restore(candidate, original);
    assert!(!encoded.replace("\r\n", "").contains('\n'), "{encoded:?}");
    let decoded = decode_literal_tokens(&parsed(&encoded)).unwrap();
    assert_eq!(decoded["log"], json!("one {{ x }}\ntwo\n"));
    assert_eq!(decoded["after"], json!("{{ y }}"));
}

#[test]
fn an_unlocatable_owned_value_is_refused_with_its_line() {
    let original = "---\nprompt: p\n---\nOld\n";
    for (candidate, property, line) in [
        ("---\nprompt: p\nanchored: &a \"{{ x }}\"\n---\nNew\n", "anchored", 3),
        ("---\nprompt: p\nflow: [a, b $(x)]\n---\nNew\n", "flow[1]", 3),
        ("---\nprompt: p\nok: fine\nnested:\n  - - \"{{ x }}\"\n---\nNew\n", "nested[0][0]", 4),
    ] {
        let restored = restore_properties_text(candidate, original, CLOSURE_OWNED_PROPERTIES).unwrap();
        let Err(EncodeError::Rejected(error)) =
            encode_agent_values(&restored.text, &restored.frontmatter_delta)
        else {
            panic!("expected a rejection for {candidate}");
        };
        assert_eq!(error.property.as_deref(), Some(property), "{error}");
        assert_eq!(error.line, Some(line), "{error}");
        assert!(error.agent_edit);
    }
}

#[test]
fn the_core_scalar_spellings_are_recognized() {
    for text in ["0", "-12", "+3", "1.5", ".5", "1.", "1e3", "-2.5E-4", "0o17", "0xFF", ".inf", "-.Inf", ".nan", "null", "~", "True"] {
        assert!(is_core_scalar(text), "{text}");
    }
    for text in ["", ".", "1.2.3", "e3", "0x", "0o8", "yes", "1_000", "12abc", "-"] {
        assert!(!is_core_scalar(text), "{text}");
    }
}

#[test]
fn persisted_data_encodes_only_gated_string_leaves_and_never_keys() {
    let value = json!({
        "{{ key }}": "plain",
        "note": "see {{…}}",
        "list": ["$(x)", "ok", 3, null],
        "nested": {"deep": "a {{ b }}"},
    });
    let stored = persisted_data(&value);
    assert_eq!(stored["{{ key }}"], json!("plain"));
    assert_eq!(stored["note"], json!(encode("see {{…}}")));
    assert_eq!(stored["list"], json!([encode("$(x)"), "ok", 3, null]));
    assert_eq!(stored["nested"]["deep"], json!(encode("a {{ b }}")));
    assert_eq!(decode_literal_tokens(&stored).unwrap(), value);
    assert_eq!(persisted_data(&json!("plain")), json!("plain"));
}
