//! `schemas::completion::path_candidates`: what Tab offers for a property path
//! and its value, per command, from a document's effective schema.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::completion::{
    PathCandidate, PathCandidateKind, PathCommand, PathPosition, PathQuery, PathSchema, declared_shape, path_candidates, path_value_takes_files,
};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, frontmatter_as_json};
use serde_json::{Value, json};
use simplified_schema::completion::PathStep;
use simplified_schema::{PropertyPath, SetterValue};

use PathCommand::{Compose, Get, Rm, Set};

/// A document's effective schema under the default compiled baseline.
fn loaded(document: &str) -> (Option<EffectiveSchema>, Value) {
    let markdown: Markdown = document.into();
    let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
        .with_darkmatter_baseline_json_schema()
        .expect("the compiled baseline loads");
    let effective = api.effective_for(&markdown).expect("the schema resolves");
    (effective, frontmatter_as_json(&markdown))
}

fn path_schema(effective: Option<&EffectiveSchema>) -> PathSchema<'_> {
    effective.map_or(PathSchema::CompiledBaseline, PathSchema::Effective)
}

fn candidates(document: &str, command: PathCommand, word: &str) -> Vec<PathCandidate> {
    let (effective, frontmatter) = loaded(document);
    path_candidates(&PathQuery {
        schema: path_schema(effective.as_ref()),
        frontmatter: &frontmatter,
        command,
        position: PathPosition::Path(word),
        earlier_paths: &[],
    })
}

fn inserts(document: &str, command: PathCommand, word: &str) -> Vec<String> {
    candidates(document, command, word).into_iter().map(|candidate| candidate.insert).collect()
}

fn set_values(document: &str, path: &str, typed: &str) -> Vec<String> {
    let (effective, frontmatter) = loaded(document);
    let path = PropertyPath::parse(path).unwrap();
    path_candidates(&PathQuery {
        schema: path_schema(effective.as_ref()),
        frontmatter: &frontmatter,
        command: Set,
        position: PathPosition::Value { path: &path, typed },
        earlier_paths: &[],
    })
    .into_iter()
    .map(|candidate| candidate.insert)
    .collect()
}

fn baseline(command: PathCommand, word: &str) -> Vec<PathCandidate> {
    path_candidates(&PathQuery {
        schema: PathSchema::CompiledBaseline,
        frontmatter: &Value::Null,
        command,
        position: PathPosition::Path(word),
        earlier_paths: &[],
    })
}

const SHAPES: &str = r#"---
$schema:
  title: string -> the page title
  status: enum(draft, published)
  author:
    name: string -> who wrote it
    email: string
  items: "string[]"
  open: object
  tags:
    "<string>": unknown
    known: string
author:
  avatar: me.png
items: [a, b]
extra: 1
---
"#;

mod keys_per_command {
    use super::*;

    const DOCUMENT: &str = r#"---
$schema:
  needed: string(required)
  declared: string
  present_declared: string
zeta: 1
present_declared: x
alpha: 2
needed: y
---
"#;

    /// The test document's own keys, leaving out the baseline's many globals.
    fn own(mut words: Vec<String>) -> Vec<String> {
        const OWN: [&str; 5] = ["needed", "declared", "present_declared", "zeta", "alpha"];
        words.retain(|word| OWN.contains(&word.trim_end_matches(['=', '.'])));
        words
    }

    #[test]
    fn compose_offers_schema_keys_with_required_first_then_keys_present() {
        assert_eq!(
            own(inserts(DOCUMENT, Compose, "")),
            ["needed=", "declared=", "present_declared=", "zeta=", "alpha="]
        );
    }

    #[test]
    fn set_offers_schema_keys_with_required_first_then_keys_present() {
        assert_eq!(own(inserts(DOCUMENT, Set, "")), ["needed", "declared", "present_declared", "zeta", "alpha"]);
    }

    #[test]
    fn get_offers_keys_present_in_document_order_then_absent_schema_keys() {
        assert_eq!(own(inserts(DOCUMENT, Get, "")), ["zeta", "present_declared", "alpha", "needed", "declared"]);
    }

    #[test]
    fn rm_offers_only_keys_present_in_document_order() {
        assert_eq!(inserts(DOCUMENT, Rm, ""), ["zeta", "present_declared", "alpha", "needed"]);
    }

    #[test]
    fn a_document_schema_still_sees_the_baseline() {
        let words = inserts(DOCUMENT, Compose, "st");
        assert!(words.contains(&"style.".to_string()), "{words:?}");
        assert!(words.contains(&"style=".to_string()), "{words:?}");
    }

    #[test]
    fn rm_ignores_declared_children_that_are_absent() {
        assert_eq!(inserts(SHAPES, Rm, "author."), ["author.avatar"]);
    }
}

mod spelling {
    use super::*;

    fn kinds(document: &str, command: PathCommand, word: &str) -> Vec<(String, PathCandidateKind)> {
        candidates(document, command, word)
            .into_iter()
            .map(|candidate| (candidate.insert, candidate.kind))
            .collect()
    }

    #[test]
    fn an_object_in_a_setter_command_descends_and_assigns() {
        assert_eq!(
            kinds(SHAPES, Compose, "auth"),
            [
                ("author.".to_string(), PathCandidateKind::Descend),
                ("author=".to_string(), PathCandidateKind::Assign)
            ]
        );
    }

    #[test]
    fn a_scalar_in_a_setter_command_assigns() {
        assert_eq!(kinds(SHAPES, Compose, "titl"), [("title=".to_string(), PathCandidateKind::Assign)]);
    }

    #[test]
    fn an_existing_object_selects_and_descends_in_a_path_command() {
        for command in [Set, Get, Rm] {
            assert_eq!(
                kinds(SHAPES, command, "auth"),
                [
                    ("author.".to_string(), PathCandidateKind::Descend),
                    ("author".to_string(), PathCandidateKind::Select)
                ],
                "{command:?}"
            );
        }
    }

    #[test]
    fn an_absent_object_only_descends_in_a_path_command() {
        const DOCUMENT: &str = "---\n$schema:\n  page:\n    width: number\n---\n";
        for command in [Set, Get] {
            assert_eq!(kinds(DOCUMENT, command, "pag"), [("page.".to_string(), PathCandidateKind::Descend)], "{command:?}");
        }
        assert_eq!(kinds(DOCUMENT, Compose, "page."), [("page.width=".to_string(), PathCandidateKind::Assign)]);
        assert_eq!(kinds(DOCUMENT, Set, "page."), [("page.width".to_string(), PathCandidateKind::Select)]);
    }

    #[test]
    fn a_union_with_an_object_arm_descends_and_keeps_the_whole_value() {
        const DOCUMENT: &str = "---\n$schema:\n  page:\n    - string\n    - width: number\n---\n";
        assert_eq!(inserts(DOCUMENT, Compose, "pag"), ["page.", "page="]);
        assert_eq!(inserts(DOCUMENT, Set, "pag"), ["page.", "page"]);
    }

    #[test]
    fn an_array_descends_through_its_indexes_and_assigns_whole() {
        assert_eq!(inserts(SHAPES, Compose, "item"), ["items.", "items="]);
        assert_eq!(inserts(SHAPES, Compose, "items."), ["items.0=", "items.1="]);
        assert_eq!(inserts(SHAPES, Get, "items."), ["items.0", "items.1"]);
        assert_eq!(kinds(SHAPES, Rm, "items.1"), [("items.1".to_string(), PathCandidateKind::Select)]);
    }

    #[test]
    fn a_container_item_descends_and_a_scalar_item_does_not() {
        const DOCUMENT: &str = "---\n$schema:\n  items: \"{ name: string }[]\"\nitems:\n  - name: a\n  - b\n---\n";
        assert_eq!(inserts(DOCUMENT, Compose, "items."), ["items.0.", "items.0=", "items.1="]);
        assert_eq!(inserts(DOCUMENT, Compose, "items.0."), ["items.0.name="]);
    }

    #[test]
    fn an_empty_or_absent_array_has_no_indexes_to_descend() {
        assert_eq!(inserts("---\n$schema:\n  items: \"string[]\"\n---\n", Compose, "item"), ["items="]);
        assert_eq!(inserts("---\nitems: []\n---\n", Compose, "item"), ["items="]);
        assert_eq!(inserts("---\nitems: []\n---\n", Rm, "item"), ["items"]);
    }

    #[test]
    fn an_open_object_invents_no_keys() {
        assert_eq!(inserts(SHAPES, Compose, "ope"), ["open="]);
        assert!(inserts(SHAPES, Compose, "open.").is_empty());
        const WITH_KEYS: &str = "---\n$schema:\n  open: object\nopen:\n  here: 1\n---\n";
        assert_eq!(inserts(WITH_KEYS, Compose, "ope"), ["open.", "open="]);
        assert_eq!(inserts(WITH_KEYS, Compose, "open."), ["open.here="]);
    }

    #[test]
    fn a_pattern_key_is_never_offered() {
        assert_eq!(inserts(SHAPES, Compose, "tags."), ["tags.known="]);
        const WITH_KEYS: &str = "---\n$schema:\n  tags:\n    \"<string>\": unknown\n    known: string\ntags:\n  mine: 1\n---\n";
        assert_eq!(inserts(WITH_KEYS, Compose, "tags."), ["tags.known=", "tags.mine="]);
        const ONLY_PATTERN: &str = "---\n$schema:\n  tags:\n    \"<string>\": unknown\n---\n";
        assert_eq!(inserts(ONLY_PATTERN, Compose, "tag"), ["tags="]);
        assert_eq!(inserts(ONLY_PATTERN, Set, "tag"), ["tags"]);
    }

    #[test]
    fn a_scalar_or_null_cannot_be_descended() {
        const DOCUMENT: &str = "---\n$schema:\n  author:\n    name: string\nauthor: text\nother:\n---\n";
        assert_eq!(inserts(DOCUMENT, Compose, "auth"), ["author="]);
        assert!(inserts(DOCUMENT, Compose, "author.").is_empty());
        assert_eq!(inserts(DOCUMENT, Compose, "othe"), ["other="]);
        assert!(inserts(DOCUMENT, Compose, "other.").is_empty());
    }

    #[test]
    fn a_path_that_could_only_fail_offers_nothing() {
        for word in ["items.9.", "items.01.", "missing.0.", "a..", ".", ".a", "items.0.x."] {
            assert!(inserts(SHAPES, Compose, word).is_empty(), "{word}");
        }
    }

    #[test]
    fn a_description_is_the_help_text_and_only_keys_have_it() {
        let found = candidates(SHAPES, Compose, "titl");
        assert_eq!(found[0].help.as_deref(), Some("the page title"));
        let found = candidates(SHAPES, Rm, "author.");
        assert_eq!(found[0].help, None);
        let found = candidates(SHAPES, Compose, "author.na");
        assert_eq!(found[0].help.as_deref(), Some("who wrote it"));
    }

    #[test]
    fn a_literal_dot_is_escaped_and_found_through_its_escape() {
        const DOCUMENT: &str = "---\na.b: 1\nplain:\n  c.d: 2\n---\n";
        assert_eq!(inserts(DOCUMENT, Get, "a"), [r"a\.b"]);
        assert_eq!(inserts(DOCUMENT, Get, r"a\"), [r"a\.b"]);
        assert_eq!(inserts(DOCUMENT, Compose, r"a\."), [r"a\.b="]);
        assert_eq!(inserts(DOCUMENT, Get, "plain.c"), [r"plain.c\.d"]);
        let written = inserts(DOCUMENT, Get, "plain.").remove(0);
        assert_eq!(PropertyPath::parse(&written).unwrap().segments(), ["plain", "c.d"]);
    }

    #[test]
    fn a_backslash_in_a_key_round_trips() {
        const DOCUMENT: &str = "---\n'a\\b': 1\n---\n";
        let written = inserts(DOCUMENT, Get, "a").remove(0);
        assert_eq!(written, r"a\\b");
        assert_eq!(PropertyPath::parse(&written).unwrap().segments(), [r"a\b"]);
    }

    #[test]
    fn the_typed_parent_is_kept_as_written() {
        assert_eq!(inserts("---\nb.c:\n  d: 1\n---\n", Get, r"b\.c.d"), [r"b\.c.d"]);
    }

    #[test]
    fn only_the_receiving_grammar_is_offered() {
        const DOCUMENT: &str = "---\n$schema:\n  title: string\n'9key': 1\nx=y: 2\nspaced key: 3\n'': 4\nnested:\n  '7': 5\n---\n";
        let compose = inserts(DOCUMENT, Compose, "");
        for refused in ["$schema", "9key", "x=y", "spaced key"] {
            assert!(!compose.iter().any(|word| word.starts_with(refused)), "{refused}: {compose:?}");
        }
        assert_eq!(inserts(DOCUMENT, Compose, "nested."), ["nested.7="]);

        let read = inserts(DOCUMENT, Get, "");
        for allowed in ["$schema", "9key", "x=y", "spaced key"] {
            assert!(read.iter().any(|word| word == allowed), "{allowed}: {read:?}");
        }
        assert!(!read.iter().any(String::is_empty), "{read:?}");
    }

    #[test]
    fn a_word_that_is_not_a_setter_offers_nothing_after_its_equals() {
        for word in ["./a=", "9key=", "=x", "a..b=x"] {
            assert!(inserts(SHAPES, Compose, word).is_empty(), "{word}");
        }
        // Outside compose `=` is an ordinary character of a key.
        assert_eq!(inserts("---\nx=y: 1\n---\n", Get, "x="), ["x=y"]);
    }
}

mod deduplication {
    use super::*;

    fn with_earlier(document: &str, word: &str, earlier: &[&str]) -> Vec<String> {
        let (effective, frontmatter) = loaded(document);
        let earlier: Vec<PropertyPath> = earlier.iter().map(|path| PropertyPath::parse(path).unwrap()).collect();
        path_candidates(&PathQuery {
            schema: path_schema(effective.as_ref()),
            frontmatter: &frontmatter,
            command: Compose,
            position: PathPosition::Path(word),
            earlier_paths: &earlier,
        })
        .into_iter()
        .map(|candidate| candidate.insert)
        .collect()
    }

    #[test]
    fn a_declared_and_present_key_appears_once_with_its_description() {
        let found = candidates("---\n$schema:\n  title: string -> the title\ntitle: x\n---\n", Rm, "titl");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].help.as_deref(), Some("the title"));

        let words = inserts("---\n$schema:\n  title: string -> the title\ntitle: x\n---\n", Compose, "titl");
        assert_eq!(words, ["title="]);
    }

    #[test]
    fn an_earlier_setter_is_not_offered_again_by_decoded_path() {
        const DOCUMENT: &str = "---\n$schema:\n  author:\n    name: string\n    email: string\n---\n";
        assert_eq!(with_earlier(DOCUMENT, "author.", &["author.name"]), ["author.email="]);
        // Spelled differently, the same decoded path.
        const DOTTED: &str = "---\na.b: 1\na:\n  b: 1\n---\n";
        assert_eq!(with_earlier(DOTTED, "a", &[r"a\.b"]), ["a.", "a="]);
        assert_eq!(with_earlier(DOTTED, "a.", &["a.b"]), Vec::<String>::new());
    }

    #[test]
    fn a_parent_whose_child_was_set_stays_available() {
        const DOCUMENT: &str = "---\n$schema:\n  author:\n    name: string\n    email: string\n---\n";
        assert_eq!(with_earlier(DOCUMENT, "auth", &["author.name"]), ["author.", "author="]);
    }

    #[test]
    fn a_whole_key_given_earlier_goes_but_its_children_stay() {
        const DOCUMENT: &str = "---\n$schema:\n  author:\n    name: string\n---\n";
        assert!(with_earlier(DOCUMENT, "auth", &["author"]).is_empty());
        assert_eq!(with_earlier(DOCUMENT, "author.", &["author"]), ["author.name="]);
    }
}

mod values {
    use super::*;

    const DOCUMENT: &str = r#"---
$schema:
  status: enum(draft, published)
  flag: boolean
  sized: string(suggest('small', 'large'))
  pinned: literal('only')
  count: literal(5)
  either:
    - enum(left, right)
    - boolean
    - enum(left, up)
    - number
  free: string
  amount: number
  anything: unknown
  shaped:
    a: string
  cover: file
  covers: "file[]"
  picture:
    - file
    - enum(none)
status: draft
covers: [a.png]
---
"#;

    #[test]
    fn enum_members_arrive_through_a_setter_and_through_set() {
        assert_eq!(inserts(DOCUMENT, Compose, "status="), ["status=draft", "status=published"]);
        assert_eq!(inserts(DOCUMENT, Compose, "status=pu"), ["status=published"]);
        assert_eq!(set_values(DOCUMENT, "status", ""), ["draft", "published"]);
        assert_eq!(set_values(DOCUMENT, "status", "d"), ["draft"]);
    }

    #[test]
    fn a_value_is_marked_as_finished() {
        let found = candidates(DOCUMENT, Compose, "status=dr");
        assert_eq!(found[0].kind, PathCandidateKind::Value);
        assert_eq!(found[0].help, None);
    }

    #[test]
    fn a_literal_offers_its_value() {
        assert_eq!(set_values(DOCUMENT, "pinned", ""), ["only"]);
        assert_eq!(set_values(DOCUMENT, "count", ""), ["5"]);
    }

    #[test]
    fn suggestions_are_offered() {
        assert_eq!(set_values(DOCUMENT, "sized", ""), ["small", "large"]);
    }

    #[test]
    fn a_boolean_offers_both() {
        assert_eq!(set_values(DOCUMENT, "flag", ""), ["true", "false"]);
    }

    #[test]
    fn a_union_offers_each_arm_once_in_order() {
        assert_eq!(set_values(DOCUMENT, "either", ""), ["left", "right", "true", "false", "up"]);
    }

    #[test]
    fn open_types_offer_nothing() {
        for path in ["free", "amount", "anything", "shaped", "cover", "covers", "undeclared"] {
            assert!(set_values(DOCUMENT, path, "").is_empty(), "{path}");
        }
    }

    #[test]
    fn a_value_below_an_unreachable_path_offers_nothing() {
        assert!(set_values("---\n$schema:\n  a:\n    b: boolean\na: 1\n---\n", "a.b", "").is_empty());
    }

    #[test]
    fn a_file_type_asks_the_caller_for_files() {
        let takes_files = |path: &str| {
            let (effective, frontmatter) = loaded(DOCUMENT);
            let path = PropertyPath::parse(path).unwrap();
            path_value_takes_files(&PathQuery {
                schema: path_schema(effective.as_ref()),
                frontmatter: &frontmatter,
                command: Set,
                position: PathPosition::Value { path: &path, typed: "" },
                earlier_paths: &[],
            })
        };
        assert!(takes_files("cover"));
        assert!(takes_files("picture"));
        assert!(takes_files("covers.0"));
        assert!(!takes_files("covers"), "a whole array is not one path");
        assert!(!takes_files("status"));
        assert!(!takes_files("undeclared"));
        assert_eq!(set_values(DOCUMENT, "picture", ""), ["none"], "the other arm stays");

        let (effective, frontmatter) = loaded(DOCUMENT);
        let word = |command, word| PathQuery {
            schema: path_schema(effective.as_ref()),
            frontmatter: &frontmatter,
            command,
            position: PathPosition::Path(word),
            earlier_paths: &[],
        };
        assert!(path_value_takes_files(&word(Compose, "cover=./a")));
        assert!(!path_value_takes_files(&word(Compose, "cover")));
        assert!(!path_value_takes_files(&word(Get, "cover=")));
    }

    #[test]
    fn a_nested_array_element_reads_its_own_type() {
        const NESTED: &str = "---\n$schema:\n  items: \"enum(a, b)[]\"\nitems: [a]\n---\n";
        assert_eq!(inserts(NESTED, Compose, "items.0="), ["items.0=a", "items.0=b"]);
        assert!(set_values(NESTED, "items", "").is_empty(), "a whole array has no member");
        assert!(set_values(NESTED, "items.3", "").is_empty(), "out of range");
    }
}

/// Every value below must reach `md` as one argument and read back as the scalar
/// the schema declares, through the same parser and binding a run uses.
mod value_spelling {
    use super::*;

    /// Every offered value, read back through the value parser and the declared
    /// type, is the member it stands for.
    #[test]
    fn every_candidate_decodes_to_its_member_under_each_declared_shape() {
        for (schema, members) in [
            ("\"enum('5', on, '1.10')\"", vec![json!("5"), json!("on"), json!("1.10")]),
            ("\"enum('null', 'true', '{a}')\"", vec![json!("null"), json!("true"), json!("{a}")]),
            ("literal(5)", vec![json!(5)]),
            ("\"literal('5')\"", vec![json!("5")]),
            ("literal(true)", vec![json!(true)]),
        ] {
            let document = format!("---\n$schema:\n  pick: {schema}\n---\n");
            let words = set_values(&document, "pick", "");
            assert_eq!(words.len(), members.len(), "{schema}: {words:?}");
            let (effective, frontmatter) = loaded(&document);
            let shape = declared_shape(effective.as_ref().unwrap(), &frontmatter, &[PathStep::Key("pick")], None);
            for (word, member) in words.iter().zip(&members) {
                let read = SetterValue::parse(word).unwrap().bind(shape.as_ref());
                assert_eq!(read, *member, "{schema}: {word}");
            }
        }
    }

    fn assert_reads_back(member_yaml: &str, expected: &str, spelled: &str) {
        let document = format!("---\n$schema:\n  pick:\n    - literal({member_yaml})\n---\n");
        let words = set_values(&document, "pick", "");
        assert_eq!(words, [spelled], "{member_yaml}");

        let (effective, frontmatter) = loaded(&document);
        let shape = declared_shape(effective.as_ref().unwrap(), &frontmatter, &[PathStep::Key("pick")], None);
        let read = SetterValue::parse(&words[0]).unwrap().bind(shape.as_ref());
        assert_eq!(read, json!(expected), "{member_yaml}");

        let setter = inserts(&document, Compose, "pick=").remove(0);
        assert_eq!(setter, format!("pick={spelled}"));
    }

    #[test]
    fn a_space_stays_plain() {
        assert_reads_back("'in review'", "in review", "in review");
    }

    #[test]
    fn a_dollar_stays_plain() {
        assert_reads_back("'$HOME'", "$HOME", "$HOME");
    }

    #[test]
    fn a_backslash_stays_plain() {
        assert_reads_back(r"'a\\b'", r"a\b", r"a\b");
    }

    #[test]
    fn an_equals_stays_plain() {
        assert_reads_back("'a=b'", "a=b", "a=b");
    }

    #[test]
    fn an_inner_quote_stays_plain() {
        assert_reads_back(r#"'say "hi"'"#, r#"say "hi""#, r#"say "hi""#);
    }

    #[test]
    fn a_fully_quoted_member_is_quoted_again() {
        assert_reads_back(r#"'"x"'"#, r#""x""#, r#""\"x\"""#);
    }

    #[test]
    fn words_the_parser_would_read_as_another_type_are_quoted() {
        assert_reads_back("'null'", "null", r#""null""#);
        assert_reads_back("'{a}'", "{a}", r#""{a}""#);
    }

    #[test]
    fn a_string_that_looks_numeric_binds_through_its_declared_type() {
        assert_reads_back("'1.10'", "1.10", "1.10");
    }

    #[test]
    fn with_no_effective_schema_the_baseline_answers_for_values() {
        let path = PropertyPath::parse("style.page.alignment").unwrap();
        let words: Vec<String> = path_candidates(&PathQuery {
            schema: PathSchema::CompiledBaseline,
            frontmatter: &Value::Null,
            command: Set,
            position: PathPosition::Value { path: &path, typed: "" },
            earlier_paths: &[],
        })
        .into_iter()
        .map(|candidate| candidate.insert)
        .collect();
        assert!(words.contains(&"center".to_string()), "{words:?}");
    }
}

mod narrowing {
    use super::*;

    const UNION: &str = r#"---
$schema:
  block:
    - kind: literal('rust')
      edition: enum(2021, 2024)
    - kind: literal('python')
      version: string
BLOCK
---
"#;

    fn with_block(block: &str) -> String {
        UNION.replace("BLOCK", block)
    }

    #[test]
    fn a_literal_tag_narrows_keys_to_its_arm() {
        let document = with_block("block:\n  kind: rust");
        assert_eq!(inserts(&document, Compose, "block."), ["block.kind=", "block.edition="]);
        let document = with_block("block:\n  kind: python");
        assert_eq!(inserts(&document, Compose, "block."), ["block.kind=", "block.version="]);
    }

    #[test]
    fn without_a_tag_every_arm_contributes() {
        let document = with_block("block: {}");
        assert_eq!(inserts(&document, Compose, "block."), ["block.kind=", "block.edition=", "block.version="]);
        let absent = with_block("");
        assert_eq!(inserts(&absent, Compose, "block."), ["block.kind=", "block.edition=", "block.version="]);
    }

    #[test]
    fn a_setter_applied_to_the_overlay_narrows_the_same_way() {
        // The caller passes frontmatter with the line's earlier setters applied.
        let (effective, _) = loaded(&with_block(""));
        let overlay = json!({ "block": { "kind": "rust" } });
        let earlier = [PropertyPath::parse("block.kind").unwrap()];
        let words: Vec<String> = path_candidates(&PathQuery {
            schema: path_schema(effective.as_ref()),
            frontmatter: &overlay,
            command: Compose,
            position: PathPosition::Path("block."),
            earlier_paths: &earlier,
        })
        .into_iter()
        .map(|candidate| candidate.insert)
        .collect();
        assert_eq!(words, ["block.edition="]);
    }

    #[test]
    fn a_tag_narrows_a_siblings_values() {
        let document = with_block("block:\n  kind: rust");
        assert_eq!(set_values(&document, "block.edition", ""), ["2021", "2024"]);
        assert!(set_values(&document, "block.version", "").is_empty());
    }

    #[test]
    fn the_tag_being_replaced_does_not_narrow_itself() {
        let document = with_block("block:\n  kind: rust");
        assert_eq!(set_values(&document, "block.kind", ""), ["rust", "python"]);
    }

    #[test]
    fn a_root_union_narrows_too() {
        const ROOT: &str = "---\n$schema:\n  - kind: literal('a')\n    only_a: string\n  - kind: literal('b')\n    only_b: string\nkind: b\n---\n";
        let words = inserts(ROOT, Compose, "only");
        assert_eq!(words, ["only_b="]);
        assert_eq!(set_values(ROOT, "kind", ""), ["a", "b"]);
    }
}

mod schema_sources {
    use super::*;

    #[test]
    fn a_replaced_baseline_property_loses_the_baseline_declaration() {
        const DOCUMENT: &str = "---\n$schema:\n  style: enum(plain, fancy)\n---\n";
        assert_eq!(inserts(DOCUMENT, Compose, "styl"), ["style="]);
        assert!(inserts(DOCUMENT, Compose, "style.").is_empty(), "the baseline's children are gone");
        assert_eq!(set_values(DOCUMENT, "style", ""), ["plain", "fancy"]);
    }

    #[test]
    fn a_separate_schema_file_supplies_keys_values_and_recursive_types() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("track.yaml"),
            concat!(
                "types:\n",
                "  item:\n",
                "    name: enum(one, two) -> the name\n",
                "    children: item[]\n",
                "$schema:\n",
                "  tree: item\n",
            ),
        )
        .unwrap();
        let path = dir.path().join("doc.md");
        std::fs::write(&path, "---\n$schema: ./track.yaml\ntree:\n  children:\n    - name: one\n---\n").unwrap();
        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path().to_path_buf()));
        let effective = api.effective_for(&markdown).unwrap();
        assert!(effective.is_some());
        let frontmatter = frontmatter_as_json(&markdown);
        let ask = |word: &str| -> Vec<String> {
            path_candidates(&PathQuery {
                schema: path_schema(effective.as_ref()),
                frontmatter: &frontmatter,
                command: Compose,
                position: PathPosition::Path(word),
                earlier_paths: &[],
            })
            .into_iter()
            .map(|candidate| candidate.insert)
            .collect()
        };
        assert_eq!(ask("tre"), ["tree.", "tree="]);
        assert_eq!(ask("tree."), ["tree.name=", "tree.children.", "tree.children="]);
        assert_eq!(ask("tree.children.0."), ["tree.children.0.name=", "tree.children.0.children="]);
        assert_eq!(ask("tree.children.0.name="), ["tree.children.0.name=one", "tree.children.0.name=two"]);
    }

    #[test]
    fn with_no_effective_schema_the_baseline_answers() {
        let words: Vec<String> = baseline(Compose, "sty").into_iter().map(|candidate| candidate.insert).collect();
        assert_eq!(words, ["style.", "style="]);
        let children = baseline(Compose, "style.");
        assert!(children.iter().any(|candidate| candidate.insert == "style.page."), "{children:?}");
        assert!(baseline(Rm, "").is_empty(), "no document, nothing to remove");
    }

    #[test]
    fn with_no_effective_schema_document_keys_still_supplement() {
        let frontmatter = json!({ "mine": 1 });
        let words: Vec<String> = path_candidates(&PathQuery {
            schema: PathSchema::CompiledBaseline,
            frontmatter: &frontmatter,
            command: Compose,
            position: PathPosition::Path("mi"),
            earlier_paths: &[],
        })
        .into_iter()
        .map(|candidate| candidate.insert)
        .collect();
        assert_eq!(words, ["mine="]);
    }

    /// The slot grammar is not built yet, so such a schema fails to resolve, and
    /// a caller then passes `None`. When it lands the schema will resolve, and the
    /// slot must still offer nothing, which this keeps asking.
    #[test]
    fn a_dynamic_slot_offers_nothing() {
        const DOCUMENT: &str = "---\n$schema:\n  status: \"enum({{ list() }})\"\nstatus: x\n---\n";
        let markdown: Markdown = DOCUMENT.into();
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        let effective = api.effective_for(&markdown).unwrap_or(None);
        let frontmatter = frontmatter_as_json(&markdown);
        let path = PropertyPath::parse("status").unwrap();
        let query = |position| PathQuery {
            schema: path_schema(effective.as_ref()),
            frontmatter: &frontmatter,
            command: Set,
            position,
            earlier_paths: &[],
        };
        let values = path_candidates(&query(PathPosition::Value { path: &path, typed: "" }));
        assert!(values.is_empty(), "{values:?}");

        let keys: Vec<String> = path_candidates(&query(PathPosition::Path("statu")))
            .into_iter()
            .map(|candidate| candidate.insert)
            .collect();
        assert_eq!(keys, ["status"], "the document's own key remains");
    }

    #[test]
    fn a_child_the_schema_does_not_declare_offers_no_values() {
        let (effective, _) = loaded("---\n$schema:\n  tree: string\n---\n");
        let overlay = json!({ "tree": { "x": 1 } });
        let path = PropertyPath::parse("tree.x").unwrap();
        let found = path_candidates(&PathQuery {
            schema: path_schema(effective.as_ref()),
            frontmatter: &overlay,
            command: Set,
            position: PathPosition::Value { path: &path, typed: "" },
            earlier_paths: &[],
        });
        assert!(found.is_empty(), "{found:?}");
    }
}

/// The baseline a request selected, in place of the compiled one.
mod selected_baseline {
    use super::*;

    const CUSTOM: &str = "$schema:\n  version: string\n  status: enum(custom)\n";

    struct Selected {
        effective: EffectiveSchema,
        frontmatter: Value,
        _dir: tempfile::TempDir,
    }

    /// `document`'s effective schema under the baseline `configure` selects, with
    /// `files` written beside it.
    fn select(
        document: &str,
        files: &[(&str, &str)],
        configure: impl FnOnce(DarkmatterSchemas, &std::path::Path) -> DarkmatterSchemas,
    ) -> Selected {
        let dir = tempfile::TempDir::new().unwrap();
        for (name, text) in files {
            std::fs::write(dir.path().join(name), text).unwrap();
        }
        let path = dir.path().join("doc.md");
        std::fs::write(&path, document).unwrap();
        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let api = configure(DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path().to_path_buf())), dir.path());
        let effective = api.effective_for(&markdown).unwrap().expect("a layer applies");
        Selected {
            effective,
            frontmatter: frontmatter_as_json(&markdown),
            _dir: dir,
        }
    }

    fn with_file(name: &'static str) -> impl FnOnce(DarkmatterSchemas, &std::path::Path) -> DarkmatterSchemas {
        move |api, dir| api.with_baseline_from_file(dir.join(name)).unwrap()
    }

    fn custom(document: &str) -> Selected {
        select(document, &[("base.yaml", CUSTOM)], with_file("base.yaml"))
    }

    fn read(selected: &Selected, key: &str, raw: &str) -> Value {
        let value = SetterValue::parse(raw).unwrap();
        let shape = declared_shape(&selected.effective, &selected.frontmatter, &[PathStep::Key(key)], Some(&value));
        value.bind(shape.as_ref())
    }

    fn compose_words(selected: &Selected, word: &str) -> Vec<String> {
        path_candidates(&PathQuery {
            schema: PathSchema::Effective(&selected.effective),
            frontmatter: &selected.frontmatter,
            command: Compose,
            position: PathPosition::Path(word),
            earlier_paths: &[],
        })
        .into_iter()
        .map(|candidate| candidate.insert)
        .collect()
    }

    #[test]
    fn a_custom_baseline_declares_the_type_a_value_is_read_through() {
        assert_eq!(read(&custom("---\ntitle: T\n---\n"), "version", "1.10"), json!("1.10"));
    }

    #[test]
    fn a_parsed_custom_baseline_declares_the_type_a_value_is_read_through() {
        let schema = simplified_schema::parse_yaml_schema(&serde_yaml_ng::from_str(&CUSTOM["$schema:\n".len()..]).unwrap()).unwrap();
        let selected = select("---\ntitle: T\n---\n", &[], |api, _| api.with_baseline(schema).unwrap());
        assert_eq!(read(&selected, "version", "1.10"), json!("1.10"));
    }

    #[test]
    fn a_custom_baseline_replaces_the_compiled_declarations() {
        assert_eq!(read(&custom("---\ntitle: T\n---\n"), "title", "1.10"), json!(1.1));
        assert_eq!(compose_words(&custom("---\ntitle: T\n---\n"), "sty"), Vec::<String>::new());
    }

    #[test]
    fn a_custom_baseline_offers_its_own_keys_and_values() {
        let selected = custom("---\ntitle: T\n---\n");
        assert_eq!(compose_words(&selected, "sta"), ["status="]);
        assert_eq!(compose_words(&selected, "status="), ["status=custom"]);
    }

    #[test]
    fn a_document_schema_outranks_a_custom_baseline() {
        let selected = custom("---\n$schema:\n  version: number\n---\n");
        assert_eq!(read(&selected, "version", "5"), json!(5));
    }

    #[test]
    fn no_baseline_leaves_the_compiled_declarations_undeclared() {
        let selected = select("---\n$schema:\n  other: string\n---\n", &[], |api, _| api);
        assert_eq!(read(&selected, "title", "1.10"), json!(1.1));
        assert_eq!(compose_words(&selected, "sty"), Vec::<String>::new());
    }

    #[test]
    fn the_compiled_baseline_still_declares_when_selected() {
        let selected = select("---\n$schema:\n  other: string\n---\n", &[], |api, _| api.with_darkmatter_baseline_json_schema().unwrap());
        assert_eq!(read(&selected, "title", "1.10"), json!("1.10"));
        assert_eq!(compose_words(&selected, "sty"), ["style.", "style="]);
    }

    #[test]
    fn a_raw_json_schema_baseline_declares_what_it_can_prove() {
        const RAW: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"version":{"type":"string"},"status":{"enum":["custom"]}}}"#;
        let selected = select("---\ntitle: T\n---\n", &[("base.json", RAW)], with_file("base.json"));
        assert_eq!(read(&selected, "version", "1.10"), json!("1.10"));
        assert_eq!(compose_words(&selected, "status="), ["status=custom"]);
    }

    #[test]
    fn a_raw_json_schema_value_as_the_baseline_declares_what_it_can_prove() {
        let raw = json!({"type": "object", "properties": {"version": {"type": "string"}}});
        let selected = select("---\ntitle: T\n---\n", &[], |api, _| api.with_baseline_json_schema(raw).unwrap());
        assert_eq!(read(&selected, "version", "1.10"), json!("1.10"));
    }

    #[test]
    fn a_raw_json_schema_document_replaces_a_baseline_declaration() {
        const RAW: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"title":{"type":"number"}}}"#;
        let selected = select("---\n$schema: ./doc.json\n---\n", &[("doc.json", RAW)], |api, _| {
            api.with_darkmatter_baseline_json_schema().unwrap()
        });
        assert_eq!(read(&selected, "title", "5"), json!(5));
    }

    #[test]
    fn a_custom_baselines_recursive_types_resolve_at_any_depth() {
        const RECURSIVE: &str = "types:\n  item:\n    name: enum(one, two)\n    children: item[]\n$schema:\n  tree: item\n";
        let selected = select("---\ntree:\n  children:\n    - name: one\n---\n", &[("base.yaml", RECURSIVE)], with_file("base.yaml"));
        assert_eq!(compose_words(&selected, "tree.children.0.name="), ["tree.children.0.name=one", "tree.children.0.name=two"]);
    }

    /// Completion's reads: each schema file one selection reaches is opened once,
    /// however many globals or import sites name it.
    #[cfg(feature = "work-counters")]
    mod read_once {
        use super::*;
        use std::collections::BTreeMap;

        const TYPES: &str = "kind: schema\ntypes:\n  shared: {name: string}\n";
        /// A custom catalog in the shipped catalog's arrangement: `ctx` and
        /// `current` import the same named type.
        const CATALOG: &str = concat!(
            "kind: schema\n",
            "$schema:\n",
            "  fm:\n",
            "    status: enum(custom)\n",
            "  ctx: shared@./types.yaml\n",
            "  current: shared@./types.yaml\n",
            "  env: object\n",
            "  err: {message: string}\n",
            "  timing: {elapsed: number}\n",
        );
        const DOCUMENT: &str = "---\nstatus: custom\n---\n";

        /// `document`'s effective schema under the baseline `configure` selects,
        /// with how many times each schema file was opened, by file name.
        fn opened(
            document: &str,
            files: &[(&str, &str)],
            configure: impl FnOnce(DarkmatterSchemas, &std::path::Path) -> DarkmatterSchemas,
        ) -> (Selected, BTreeMap<String, usize>) {
            let (selected, observed) = simplified_schema::resolve::observed::collect(|| select(document, files, configure));
            let mut counts = BTreeMap::new();
            for path in observed.opened {
                *counts.entry(path.file_name().unwrap().to_string_lossy().into_owned()).or_default() += 1;
            }
            (selected, counts)
        }

        fn once(names: &[&str]) -> BTreeMap<String, usize> {
            names.iter().map(|name| ((*name).to_string(), 1)).collect()
        }

        fn words(selected: &Selected, command: PathCommand, position: PathPosition<'_>) -> Vec<String> {
            path_candidates(&PathQuery {
                schema: PathSchema::Effective(&selected.effective),
                frontmatter: &selected.frontmatter,
                command,
                position,
                earlier_paths: &[],
            })
            .into_iter()
            .map(|candidate| candidate.insert)
            .collect()
        }

        fn catalog() -> (Selected, BTreeMap<String, usize>) {
            opened(DOCUMENT, &[("base.yaml", CATALOG), ("types.yaml", TYPES)], with_file("base.yaml"))
        }

        #[test]
        fn get_reads_a_custom_catalog_and_its_repeated_import_once() {
            let (selected, opened) = catalog();
            assert!(words(&selected, Get, PathPosition::Path("sta")).contains(&"status".to_string()));
            assert_eq!(opened, once(&["base.yaml", "types.yaml"]));
        }

        #[test]
        fn set_reads_a_custom_catalog_and_its_repeated_import_once() {
            let (selected, opened) = catalog();
            let path = PropertyPath::parse("status").unwrap();
            assert_eq!(words(&selected, Set, PathPosition::Value { path: &path, typed: "cu" }), ["custom"]);
            assert_eq!(opened, once(&["base.yaml", "types.yaml"]));
        }

        #[test]
        fn rm_reads_a_custom_catalog_and_its_repeated_import_once() {
            let (selected, opened) = catalog();
            assert!(words(&selected, Rm, PathPosition::Path("sta")).contains(&"status".to_string()));
            assert_eq!(opened, once(&["base.yaml", "types.yaml"]));
        }

        #[test]
        fn an_ordinary_yaml_baseline_and_its_repeated_import_are_read_once() {
            let base = "$schema:\n  status: enum(custom)\n  a: shared@./types.yaml\n  b: shared@./types.yaml\n";
            let (selected, opened) = opened(DOCUMENT, &[("base.yaml", base), ("types.yaml", TYPES)], with_file("base.yaml"));
            assert!(words(&selected, Get, PathPosition::Path("sta")).contains(&"status".to_string()));
            assert_eq!(opened, once(&["base.yaml", "types.yaml"]));
        }

        #[test]
        fn a_raw_json_schema_baseline_is_read_once() {
            const RAW: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"status":{"enum":["custom"]}}}"#;
            let (selected, opened) = opened(DOCUMENT, &[("base.json", RAW)], with_file("base.json"));
            let path = PropertyPath::parse("status").unwrap();
            assert_eq!(words(&selected, Set, PathPosition::Value { path: &path, typed: "cu" }), ["custom"]);
            assert_eq!(opened, once(&["base.json"]));
        }

        #[test]
        fn a_document_schema_and_its_repeated_import_are_read_once() {
            let schema = "$schema:\n  first: shared@./types.yaml\n  second: shared@./types.yaml\n";
            let document = "---\n$schema: ./s.yaml\nfirst: {name: x}\n---\n";
            let (selected, opened) = opened(document, &[("s.yaml", schema), ("types.yaml", TYPES)], |api, _| {
                api.with_darkmatter_baseline_json_schema().unwrap()
            });
            assert!(words(&selected, Get, PathPosition::Path("sec")).contains(&"second.".to_string()));
            assert_eq!(opened, once(&["s.yaml", "types.yaml"]));
        }

        #[test]
        fn the_built_in_and_a_disabled_baseline_open_no_file() {
            let document = "---\n$schema:\n  status: enum(custom)\n---\n";
            let (built_in, opened_built_in) =
                opened(document, &[("base.yaml", CATALOG)], |api, _| api.with_darkmatter_baseline_json_schema().unwrap());
            let (disabled, opened_disabled) = opened(document, &[("base.yaml", CATALOG)], |api, _| api);
            for (selected, opened) in [(built_in, opened_built_in), (disabled, opened_disabled)] {
                assert!(words(&selected, Get, PathPosition::Path("sta")).contains(&"status".to_string()));
                assert!(opened.is_empty(), "{opened:?}");
            }
        }
    }

    #[test]
    fn no_schema_at_all_offers_only_what_is_present() {
        let frontmatter = json!({ "title": "T" });
        let words = |word| -> Vec<String> {
            path_candidates(&PathQuery {
                schema: PathSchema::Nothing,
                frontmatter: &frontmatter,
                command: Compose,
                position: PathPosition::Path(word),
                earlier_paths: &[],
            })
            .into_iter()
            .map(|candidate| candidate.insert)
            .collect()
        };
        assert_eq!(words(""), ["title="]);
        assert_eq!(words("style.page.alignment="), Vec::<String>::new());
    }
}

mod declared_shape_paths {
    use super::*;
    use simplified_schema::coercion::Shape;

    const DOCUMENT: &str = concat!(
        "---\n",
        "$schema:\n",
        "  named:\n",
        "    \"<string>\": string\n",
        "  starting:\n",
        "    \"<starting::a>\": number\n",
        "  loose:\n",
        "    \"<string>\": unknown\n",
        "  closed:\n",
        "    only: string\n",
        "  deep:\n",
        "    \"<string>\":\n",
        "      name: string\n",
        "named: {}\n",
        "starting: {}\n",
        "loose: {}\n",
        "closed: {}\n",
        "deep: {}\n",
        "---\n",
    );

    fn shape_at(path: &[PathStep<'_>]) -> Option<Shape> {
        let (effective, frontmatter) = loaded(DOCUMENT);
        declared_shape(effective.as_ref().unwrap(), &frontmatter, path, None)
    }

    fn reads_as(shape: Option<&Shape>, text: &str) -> Value {
        SetterValue::parse(text).unwrap().bind(shape)
    }

    #[test]
    fn a_pattern_key_declares_the_keys_it_matches() {
        let named = shape_at(&[PathStep::Key("named"), PathStep::Key("any")]);
        assert_eq!(reads_as(named.as_ref(), "1.10"), json!("1.10"));
        let started = shape_at(&[PathStep::Key("starting"), PathStep::Key("abc")]);
        assert_eq!(reads_as(started.as_ref(), "1.10"), json!(1.1));
        let unknown = shape_at(&[PathStep::Key("loose"), PathStep::Key("any")]);
        assert_eq!(reads_as(unknown.as_ref(), "1.10"), json!("1.10"));
    }

    #[test]
    fn a_descent_through_a_pattern_key_continues() {
        let name = shape_at(&[PathStep::Key("deep"), PathStep::Key("any"), PathStep::Key("name")]);
        assert_eq!(reads_as(name.as_ref(), "1.10"), json!("1.10"));
    }

    #[test]
    fn a_key_no_pattern_matches_is_undeclared() {
        assert!(shape_at(&[PathStep::Key("starting"), PathStep::Key("zzz")]).is_none());
        assert!(shape_at(&[PathStep::Key("closed"), PathStep::Key("other")]).is_none());
    }
}
