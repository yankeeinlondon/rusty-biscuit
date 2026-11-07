//! `data_locations`: the byte offset of the value at a JSON Pointer, read
//! back through `source_position` as a 1-based line and character column.

use biscuit_file::{locate_json, locate_jsonl, source_position};

/// `(line, column)` of the located value, asserting the text there starts
/// with `token`.
fn position(text: &str, offset: Option<usize>, token: &str) -> (usize, usize) {
    let offset = offset.unwrap_or_else(|| panic!("no location for {token:?} in {text:?}"));
    assert!(text[offset..].starts_with(token), "{:?} does not start with {token:?}", &text[offset..]);
    source_position(text, offset).unwrap()
}

mod json {
    use super::*;

    const PRETTY: &str = "{\n  \"name\": \"svc\",\n  \"servers\": [\n    {\"port\": 80},\n    {\"port\": \"x\"}\n  ]\n}\n";

    #[test]
    fn root_and_nested_pointers_locate_their_values() {
        assert_eq!(position(PRETTY, locate_json(PRETTY, ""), "{"), (1, 1));
        assert_eq!(position(PRETTY, locate_json(PRETTY, "/name"), "\"svc\""), (2, 11));
        assert_eq!(position(PRETTY, locate_json(PRETTY, "/servers"), "["), (3, 14));
        assert_eq!(position(PRETTY, locate_json(PRETTY, "/servers/1/port"), "\"x\""), (5, 14));
    }

    #[test]
    fn a_whitespace_padded_root_starts_at_its_first_token() {
        let text = " \n\t[1, 2]\n";
        assert_eq!(position(text, locate_json(text, ""), "["), (2, 2));
        assert_eq!(position(text, locate_json(text, "/1"), "2"), (2, 6));
    }

    #[test]
    fn tab_indented_and_minified_json_locate_the_same_value() {
        let tabs = "{\n\t\"a\": {\n\t\t\"b\": [true, null]\n\t}\n}";
        assert_eq!(position(tabs, locate_json(tabs, "/a/b/1"), "null"), (3, 15));
        let minified = r#"{"a":{"b":[true,null]}}"#;
        assert_eq!(position(minified, locate_json(minified, "/a/b/1"), "null"), (1, 17));
    }

    #[test]
    fn escaped_pointer_segments_and_the_empty_key_decode() {
        let text = r#"{"a/b": 1, "m~n": 2, "": 3, "~1": 4}"#;
        assert_eq!(position(text, locate_json(text, "/a~1b"), "1"), (1, 9));
        assert_eq!(position(text, locate_json(text, "/m~0n"), "2"), (1, 19));
        assert_eq!(position(text, locate_json(text, "/"), "3"), (1, 26));
        assert_eq!(position(text, locate_json(text, "/~01"), "4"), (1, 35));
    }

    #[test]
    fn a_missing_child_is_none_and_its_parent_is_located() {
        assert_eq!(locate_json(PRETTY, "/servers/2"), None);
        assert_eq!(locate_json(PRETTY, "/servers/0/host"), None);
        assert_eq!(locate_json(PRETTY, "/name/deeper"), None);
        assert_eq!(position(PRETTY, locate_json(PRETTY, "/servers/0"), "{"), (4, 5));
    }

    #[test]
    fn a_duplicate_key_resolves_to_its_last_occurrence() {
        let text = "{\"a\": 1,\n \"b\": 2,\n \"a\": 3}";
        assert_eq!(position(text, locate_json(text, "/a"), "3"), (3, 7));
    }

    #[test]
    fn surrogate_escapes_in_keys_and_strings_before_the_target_are_decoded() {
        let text = r#"{"\ud83d\ude00": "\ud83d\ude00 x", "k": 1}"#;
        assert_eq!(position(text, locate_json(text, "/😀"), "\""), (1, 18));
        assert_eq!(position(text, locate_json(text, "/k"), "1"), (1, 41));
    }

    #[test]
    fn multibyte_characters_count_as_one_column() {
        let text = "{\"名前\": \"é\", \"n\": \"bad\"}";
        assert_eq!(position(text, locate_json(text, "/n"), "\"bad\""), (1, 18));
    }

    #[test]
    fn crlf_twin_locates_at_the_same_line_and_column() {
        let crlf = PRETTY.replace('\n', "\r\n");
        for pointer in ["", "/name", "/servers/1/port"] {
            let lf_at = source_position(PRETTY, locate_json(PRETTY, pointer).unwrap());
            let crlf_at = source_position(&crlf, locate_json(&crlf, pointer).unwrap());
            assert_eq!(lf_at, crlf_at, "{pointer}");
        }
    }

    #[test]
    fn malformed_pointers_are_none() {
        let text = r#"{"a": [1, 2], "0": 1}"#;
        for pointer in ["a", "/a~", "/a~2", "/a/01", "/a/-1", "/a/+1", "/a/", "/a/1.0"] {
            assert_eq!(locate_json(text, pointer), None, "{pointer}");
        }
        assert!(locate_json(text, "/0").is_some(), "an index-like key is still a key");
    }

    #[test]
    fn malformed_json_is_none() {
        for text in ["", "{", "{\"a\": 1,}", "[1] 2", "{'a': 1}"] {
            assert_eq!(locate_json(text, ""), None, "{text:?}");
        }
    }
}

mod jsonl {
    use super::*;

    #[test]
    fn records_are_indexed_skipping_blank_lines_but_keep_physical_lines() {
        let text = "{\"a\": 1}\n\n   \n  {\"a\": [true, \"x\"]}\n";
        assert_eq!(position(text, locate_jsonl(text, "/0/a"), "1"), (1, 7));
        assert_eq!(position(text, locate_jsonl(text, "/1"), "{"), (4, 3));
        assert_eq!(position(text, locate_jsonl(text, "/1/a/1"), "\"x\""), (4, 16));
    }

    #[test]
    fn root_is_the_first_record() {
        let text = "\n  [1]\n";
        assert_eq!(position(text, locate_jsonl(text, ""), "[1]"), (2, 3));
        assert_eq!(locate_jsonl("", ""), Some(0));
    }

    #[test]
    fn crlf_records_locate_like_their_lf_twin() {
        let lf = "{\"a\": 1}\n\n{\"b\": \"é\", \"c\": 2}\n";
        let crlf = lf.replace('\n', "\r\n");
        for pointer in ["/0/a", "/1/c"] {
            let lf_at = source_position(lf, locate_jsonl(lf, pointer).unwrap());
            let crlf_at = source_position(&crlf, locate_jsonl(&crlf, pointer).unwrap());
            assert_eq!(lf_at, crlf_at, "{pointer}");
        }
        assert_eq!(source_position(&crlf, locate_jsonl(&crlf, "/1/c").unwrap()), Some((3, 17)));
    }

    #[test]
    fn a_missing_record_or_child_is_none_and_its_parent_is_located() {
        let text = "{\"a\": 1}\n{\"a\": 2}\n";
        assert_eq!(locate_jsonl(text, "/2"), None);
        assert_eq!(locate_jsonl(text, "/1/b"), None);
        assert_eq!(position(text, locate_jsonl(text, "/1"), "{"), (2, 1));
    }

    #[test]
    fn malformed_text_or_pointer_is_none() {
        assert_eq!(locate_jsonl("{\"a\": 1}\n{oops}\n", "/0/a"), None);
        assert_eq!(locate_jsonl("1 2\n", "/0"), None);
        for pointer in ["0", "/01", "/~2", "/a"] {
            assert_eq!(locate_jsonl("1\n", pointer), None, "{pointer}");
        }
    }
}

#[cfg(feature = "toml")]
mod toml {
    use super::*;
    use biscuit_file::locate_toml;

    #[test]
    fn standard_tables_and_array_of_tables_locate_their_values() {
        let text = "title = \"x\"\n\n[owner]\nage = 3\n\n[[servers]]\nport = 80\n\n[[servers]]\nport = \"bad\"\n";
        assert_eq!(position(text, locate_toml(text, "/owner/age"), "3"), (4, 7));
        assert_eq!(position(text, locate_toml(text, "/servers/1/port"), "\"bad\""), (10, 8));
        assert_eq!(locate_toml(text, ""), Some(0));
    }

    #[test]
    fn dotted_keys_inline_tables_and_inline_arrays_locate() {
        let text = "[db]\nnet.ip.v4 = \"1.2.3.4\"\nconn = { host = \"h\", port = 5 }\nlist = [1, \"two\"]\n";
        assert_eq!(position(text, locate_toml(text, "/db/net/ip/v4"), "\"1.2.3.4\""), (2, 13));
        assert_eq!(position(text, locate_toml(text, "/db/conn/port"), "5"), (3, 29));
        assert_eq!(position(text, locate_toml(text, "/db/list/1"), "\"two\""), (4, 12));
    }

    #[test]
    fn a_super_table_defined_after_its_sub_table_locates() {
        let text = "[a.b]\nc = 1\n\n[a]\nd = 2\n";
        assert_eq!(position(text, locate_toml(text, "/a/d"), "2"), (5, 5));
        assert_eq!(position(text, locate_toml(text, "/a/b/c"), "1"), (2, 5));
    }

    #[test]
    fn escaped_segments_multibyte_and_crlf() {
        let lf = "\"a/b\" = 1\n\"名前\" = \"é\"\n\"m~n\" = { x = 2 }\n";
        assert_eq!(position(lf, locate_toml(lf, "/a~1b"), "1"), (1, 9));
        assert_eq!(position(lf, locate_toml(lf, "/m~0n/x"), "2"), (3, 15));
        assert_eq!(position(lf, locate_toml(lf, "/名前"), "\"é\""), (2, 8));
        let crlf = lf.replace('\n', "\r\n");
        assert_eq!(position(&crlf, locate_toml(&crlf, "/m~0n/x"), "2"), (3, 15));
    }

    #[test]
    fn a_missing_child_is_none_and_its_parent_is_located() {
        let text = "[[servers]]\nport = 1\n";
        assert_eq!(locate_toml(text, "/servers/1"), None);
        assert_eq!(locate_toml(text, "/servers/0/host"), None);
        assert_eq!(locate_toml(text, "/servers/0/port/x"), None);
        assert!(locate_toml(text, "/servers/0").is_some());
    }

    #[test]
    fn malformed_toml_or_pointer_is_none() {
        assert_eq!(locate_toml("a = \n", "/a"), None);
        assert_eq!(locate_toml("a = 1\na = 2\n", "/a"), None);
        assert_eq!(locate_toml("a = [1]\n", "/a/01"), None);
        assert_eq!(locate_toml("a = 1\n", "a"), None);
    }
}

#[cfg(feature = "json5")]
mod json5 {
    use super::*;
    use biscuit_file::{json5_parse_error_offset, locate_json5};

    #[test]
    fn comments_unquoted_and_single_quoted_keys_and_trailing_commas() {
        let text = "// leading\n{\n  name: 'svc', /* inline */\n  'servers': [\n    {port: 80,},\n    {\"port\": -5},\n  ],\n}\n";
        assert_eq!(position(text, locate_json5(text, ""), "{"), (2, 1));
        assert_eq!(position(text, locate_json5(text, "/name"), "'svc'"), (3, 9));
        assert_eq!(position(text, locate_json5(text, "/servers/0/port"), "80"), (5, 12));
        assert_eq!(position(text, locate_json5(text, "/servers/1/port"), "-5"), (6, 14));
        assert_eq!(locate_json5(text, "/servers/2"), None);
    }

    #[test]
    fn escaped_keys_and_strings_before_the_target_decode() {
        let text = r#"{"\u0061": 'it\'s', \u0062: "\ud83d\ude00", 'c/d': 1, "e~f": 2, "": 3}"#;
        assert_eq!(position(text, locate_json5(text, "/a"), "'it"), (1, 12));
        assert_eq!(position(text, locate_json5(text, "/b"), "\"\\ud83d"), (1, 29));
        assert_eq!(position(text, locate_json5(text, "/c~1d"), "1"), (1, 52));
        assert_eq!(position(text, locate_json5(text, "/e~0f"), "2"), (1, 62));
        assert_eq!(position(text, locate_json5(text, "/"), "3"), (1, 69));
    }

    #[test]
    fn a_duplicate_key_resolves_to_its_last_occurrence() {
        let text = "{a: 1, 'a': 2,\n \"a\": 3}";
        assert_eq!(position(text, locate_json5(text, "/a"), "3"), (2, 7));
    }

    #[test]
    fn a_string_line_continuation_keeps_the_next_line_right() {
        let text = "{s: 'one \\\ntwo', n: 1}";
        assert_eq!(position(text, locate_json5(text, "/n"), "1"), (2, 10));
    }

    #[test]
    fn multibyte_characters_and_crlf() {
        let lf = "{名前: 'é', n: [+1,\n  .5, 0x1F]}";
        assert_eq!(position(lf, locate_json5(lf, "/n/0"), "+1"), (1, 15));
        assert_eq!(position(lf, locate_json5(lf, "/n/2"), "0x1F"), (2, 7));
        let crlf = lf.replace('\n', "\r\n");
        assert_eq!(position(&crlf, locate_json5(&crlf, "/n/2"), "0x1F"), (2, 7));
    }

    #[test]
    fn a_missing_child_is_none_and_its_parent_is_located() {
        let text = "{a: {b: [1]}}";
        assert_eq!(locate_json5(text, "/a/c"), None);
        assert_eq!(locate_json5(text, "/a/b/1"), None);
        assert_eq!(locate_json5(text, "/a/b/0/x"), None);
        assert_eq!(position(text, locate_json5(text, "/a/b"), "[1]"), (1, 9));
    }

    #[test]
    fn malformed_json5_or_pointer_is_none() {
        for text in ["", "{a: ", "{a: 1} 2", "{a: 'open}", "[1,,]"] {
            assert_eq!(locate_json5(text, ""), None, "{text:?}");
        }
        for pointer in ["a", "/a~", "/l/01"] {
            assert_eq!(locate_json5("{a: 1, l: [1]}", pointer), None, "{pointer}");
        }
    }

    #[test]
    fn parse_error_offset_is_a_byte_offset_after_multibyte_characters() {
        let text = "{\n  名前: 'é', bad: ]\n}";
        let offset = json5_parse_error_offset(text).expect("a syntax error");
        assert_eq!(&text[offset..offset + 1], "]");
        // Character column 17, byte column 22.
        assert_eq!(source_position(text, offset), Some((2, 17)));
    }

    #[test]
    fn parse_error_offset_is_none_for_text_that_parses() {
        assert_eq!(json5_parse_error_offset("// c\n{a: 1,}"), None);
        assert!(json5_parse_error_offset("{a: 1").is_some());
        assert!(json5_parse_error_offset("'open").is_some());
    }
}
