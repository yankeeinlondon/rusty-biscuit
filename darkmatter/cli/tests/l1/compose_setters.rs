//! `md compose` setters through the real binary: `key.path=value` on the command
//! line, against fixture-owned documents and schemas. A run composes through the
//! reference check, the pre-flight walk, and the pipeline, which each prepare the
//! frontmatter, so one success or failure here crosses all three.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};

fn with_document(name: &str, document: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/doc.md", document);
    fixture
}

/// Marks the fixture's directory a repository, which turns on trigger discovery.
fn with_repository(fixture: &CliProcessFixture) {
    fixture.write_file("cwd/.git/HEAD", "ref: refs/heads/main\n");
    fixture.write_file("cwd/.git/config", "[core]\n\trepositoryformatversion = 0\n");
    std::fs::create_dir_all(fixture.cwd().join(".git/objects")).unwrap();
    std::fs::create_dir_all(fixture.cwd().join(".git/refs")).unwrap();
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> (Option<i32>, String, String) {
    let output = fixture.command().args(args).output().unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// The composed body of a run that must succeed.
fn composed(fixture: &CliProcessFixture, args: &[&str]) -> String {
    let mut full = vec!["compose"];
    full.extend_from_slice(args);
    let (code, stdout, stderr) = run(fixture, &full);
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    stdout.trim().to_string()
}

/// The envelope of a run that must fail, as data output.
fn failure(fixture: &CliProcessFixture, args: &[&str]) -> Value {
    let mut full = vec!["compose"];
    full.extend_from_slice(args);
    full.extend_from_slice(&["--output", "json"]);
    let (code, stdout, stderr) = run(fixture, &full);
    assert_eq!(code, Some(1), "{stdout}{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["outcome"], "failure", "{report:#}");
    report
}

fn assert_setter_failure(report: &Value, argument: &str, path: &str) -> String {
    let diagnostic = &report["diagnostics"][0];
    assert_eq!(diagnostic["code"], "dm.setter.invalid", "{report:#}");
    assert_eq!(diagnostic["source"], Value::Null, "{report:#}");
    let message = diagnostic["message"].as_str().unwrap().to_string();
    assert!(message.contains(argument), "{message}");
    assert!(message.contains(&format!("`{path}`")), "{message}");
    message
}

const AUTHOR: &str = "---\nauthor:\n  name: Ann\n  email: a@x\n---\n{{ author.name }} {{ author.email }}\n";

mod applying {
    use super::*;

    #[test]
    fn a_nested_setter_keeps_its_siblings_and_reaches_expressions() {
        let fixture = with_document("setters-nested", AUTHOR);
        assert_eq!(composed(&fixture, &["doc.md", "author.name=Alice"]), "Alice a@x");
    }

    #[test]
    fn a_setter_may_come_before_the_document() {
        let fixture = with_document("setters-before", AUTHOR);
        assert_eq!(composed(&fixture, &["author.name=Alice", "doc.md"]), "Alice a@x");
    }

    #[test]
    fn a_missing_parent_is_created_and_shows_in_the_composed_frontmatter() {
        let fixture = with_document("setters-new-parent", "---\ntitle: T\n---\n{{ a.b.c }}\n");
        let out = composed(&fixture, &["doc.md", "a.b.c=7", "--frontmatter"]);
        assert!(out.contains("a:\n  b:\n    c: 7"), "{out}");
        assert!(out.ends_with('7'), "{out}");
    }

    #[test]
    fn an_undotted_setter_behaves_as_before() {
        let fixture = with_document("setters-undotted", "---\ntitle: T\n---\n{{ title }} {{ count + 1 }} {{ on }}\n");
        assert_eq!(composed(&fixture, &["doc.md", "title=New", "count=5", "on=true"]), "New 6 true");
    }

    #[test]
    fn a_setter_applies_to_stdin_input() {
        let fixture = CliProcessFixture::named("setters-stdin");
        let output = fixture
            .command()
            .args(["compose", "-", "author.name=Alice"])
            .write_stdin(AUTHOR)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Alice a@x");
    }

    #[test]
    fn an_array_element_is_overridden_for_this_run() {
        let fixture = with_document(
            "setters-element",
            "---\nitems:\n  - name: a\n  - name: b\n---\n{{ items[0].name }} {{ items[1].name }}\n",
        );
        assert_eq!(composed(&fixture, &["doc.md", "items.1.name=z"]), "a z");
    }

    #[test]
    fn the_file_on_disk_is_not_changed() {
        let fixture = with_document("setters-no-write", AUTHOR);
        composed(&fixture, &["doc.md", "author.name=Alice"]);
        assert_eq!(std::fs::read_to_string(fixture.cwd().join("doc.md")).unwrap(), AUTHOR);
    }

    #[test]
    fn an_escaped_dot_names_a_literal_key() {
        let fixture = with_document("setters-literal", "---\na:\n  b: nested\n---\n{{ a.b }}\n");
        let out = composed(&fixture, &["doc.md", r"a\.b=literal", "--frontmatter"]);
        assert!(out.contains("a.b: literal"), "{out}");
        assert!(out.contains("b: nested"), "{out}");
    }
}

mod classifying {
    use super::*;

    #[test]
    fn a_dotted_key_is_a_setter_and_dot_slash_makes_it_a_path() {
        let fixture = with_document("setters-classify", "---\na:\n  b: old\n---\n{{ a.b }}\n");
        fixture.write_file("cwd/a.b=c.md", "from the file\n");
        assert_eq!(composed(&fixture, &["doc.md", "a.b=c.md"]), "c.md");
        assert_eq!(composed(&fixture, &["./a.b=c.md"]), "from the file");
    }

    /// `\` separates folders only on native Windows, and classification does not
    /// know that: the word without the leading `.\` is an escaped key.
    #[cfg(windows)]
    #[test]
    fn a_windows_path_with_a_dot_folder_needs_its_leading_dot_backslash() {
        let fixture = with_document("setters-classify-windows", "---\ntitle: old\n---\n{{ title }}\n");
        fixture.write_file("cwd/dir/.config=1.md", "from the file\n");
        assert_eq!(composed(&fixture, &[r".\dir\.config=1.md"]), "from the file");
        assert_eq!(composed(&fixture, &["doc.md", r"dir\.config=1.md"]), "old");
    }

    #[test]
    fn two_paths_are_still_refused() {
        let fixture = with_document("setters-two-paths", AUTHOR);
        let (code, _, stderr) = run(&fixture, &["compose", "doc.md", "./a=b.md"]);
        assert_eq!(code, Some(1), "{stderr}");
        assert!(stderr.contains("expected at most one input path"), "{stderr}");
    }

    #[test]
    fn an_empty_key_is_a_setter_error() {
        let fixture = with_document("setters-empty-key", AUTHOR);
        let (code, _, stderr) = run(&fixture, &["compose", "doc.md", "=value"]);
        assert_eq!(code, Some(1), "{stderr}");
        assert!(stderr.contains("setter key must not be empty"), "{stderr}");
    }
}

mod precedence {
    use super::*;

    #[test]
    fn a_setter_outranks_set_and_state() {
        let fixture = with_document("setters-precedence", "---\ntitle: T\n---\n{{ a.b }} {{ a.c }} {{ d.e }} {{ d.f }}\n");
        let out = composed(
            &fixture,
            &[
                "doc.md",
                "--state",
                r#"{"d":{"e":"state","f":"state"}}"#,
                "--set",
                r#"{"a":{"b":"set","c":"set"}}"#,
                "a.b=setter",
                "d.e=setter",
            ],
        );
        assert_eq!(out, "setter set setter state");
    }

    #[test]
    fn a_whole_object_setter_outranks_a_more_specific_set_value() {
        let fixture = with_document("setters-whole-over-set", "---\ntitle: T\n---\n{{ a.b }}|{{ a.c }}|\n");
        let out = composed(&fixture, &["doc.md", "--set", r#"{"a":{"b":"set","c":"set"}}"#, "a={b:\"setter\"}"]);
        assert_eq!(out, "setter||");
    }

    #[test]
    fn a_dotted_set_key_stays_a_literal_key() {
        let fixture = with_document("setters-set-literal", "---\ntitle: T\n---\n[{{ a.b }}]\n");
        let out = composed(&fixture, &["doc.md", "--set", r#"{"a.b":1}"#, "--frontmatter"]);
        assert!(out.contains("a.b: 1"), "{out}");
        assert!(out.ends_with("[]"), "{out}");
    }

    #[test]
    fn whole_object_plus_descendant_is_the_same_in_both_orders() {
        let fixture = with_document("setters-both-orders", AUTHOR);
        let whole = r#"author={name:"Ann",email:"a@x"}"#;
        assert_eq!(composed(&fixture, &["doc.md", whole, "author.name=Bob"]), "Bob a@x");
        assert_eq!(composed(&fixture, &["doc.md", "author.name=Bob", whole]), "Bob a@x");
    }

    #[test]
    fn a_scalar_ancestor_fails_in_both_orders_and_composes_nothing() {
        let fixture = with_document("setters-scalar-ancestor", AUTHOR);
        for args in [["doc.md", "author=x", "author.name=Bob"], ["doc.md", "author.name=Bob", "author=x"]] {
            assert_setter_failure(&failure(&fixture, &args), "author.name=Bob", "author.name");
        }
        assert_setter_failure(&failure(&fixture, &["doc.md", "author=null", "author.name=Bob"]), "author.name=Bob", "author.name");
    }

    #[test]
    fn a_scalar_ancestor_is_found_before_the_document_is_read() {
        let fixture = CliProcessFixture::named("setters-no-read");
        let report = failure(&fixture, &["missing.md", "a=1", "a.b=2"]);
        assert_setter_failure(&report, "a.b=2", "a.b");
    }

    #[test]
    fn a_repeated_path_takes_the_last_value() {
        let fixture = with_document("setters-repeated", AUTHOR);
        assert_eq!(composed(&fixture, &["doc.md", "author.name=One", "author.name=Two"]), "Two a@x");
    }

    #[test]
    fn an_overridden_value_needs_only_valid_syntax() {
        let fixture = with_document("setters-overridden", "---\na: 1\n---\n{{ a }}\n");
        assert_eq!(composed(&fixture, &["doc.md", "a={x:1,x:2}", "a=2"]), "2");
    }
}

mod failing {
    use super::*;

    #[test]
    fn an_index_out_of_range_names_the_length_and_has_no_source() {
        let fixture = with_document("setters-range", "---\nitems: [a, b]\n---\n{{ items }}\n");
        let message = assert_setter_failure(&failure(&fixture, &["doc.md", "items.9=x"]), "items.9=x", "items.9");
        assert!(message.contains("2 element(s)"), "{message}");
    }

    #[test]
    fn bounds_are_checked_against_an_ancestor_setter() {
        let fixture = with_document("setters-range-ancestor", "---\nitems: [a, b, c]\n---\n{{ items }}\n");
        let report = failure(&fixture, &["doc.md", "items=[\"x\"]", "items.2=y"]);
        let message = assert_setter_failure(&report, "items.2=y", "items.2");
        assert!(message.contains("1 element(s)"), "{message}");
    }

    #[test]
    fn a_scalar_parent_and_a_number_under_a_missing_parent_fail() {
        let fixture = with_document("setters-parents", "---\nauthor: Ken\n---\n{{ author }}\n");
        assert_setter_failure(&failure(&fixture, &["doc.md", "author.name=Bob"]), "author.name=Bob", "author.name");
        assert_setter_failure(&failure(&fixture, &["doc.md", "items.0.name=x"]), "items.0.name=x", "items.0.name");
    }

    #[test]
    fn an_inadmissible_container_never_becomes_text() {
        let fixture = with_document("setters-inadmissible", AUTHOR);
        let message = assert_setter_failure(&failure(&fixture, &["doc.md", "meta={a:1,a:2}"]), "meta={a:1,a:2}", "meta");
        assert!(message.contains("duplicate key `a`"), "{message}");
        assert_setter_failure(&failure(&fixture, &["doc.md", "meta={a:NaN}"]), "meta={a:NaN}", "meta");
        assert_setter_failure(&failure(&fixture, &["doc.md", "meta={a:9007199254740993}"]), "meta={a:9007199254740993}", "meta");
    }

    #[test]
    fn a_failure_prints_to_stderr_in_markdown_mode() {
        let fixture = with_document("setters-stderr", "---\nitems: [a]\n---\nbody\n");
        let (code, stdout, stderr) = run(&fixture, &["compose", "doc.md", "items.9=x"]);
        assert_eq!(code, Some(1), "{stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.contains("items.9=x"), "{stderr}");
    }
}

mod values {
    use super::*;

    const TYPED: &str = "---\n$schema:\n  version: string\n  count: number\n  zip: numberlike\n  flag: boolish\n  mixed: string | number\n---\n{{ version }}|{{ count }}|{{ zip }}|{{ flag }}|{{ mixed }}\n";

    #[test]
    fn a_declared_type_reads_the_text() {
        let fixture = with_document("setters-typed", TYPED);
        let out = composed(&fixture, &["doc.md", "version=1.10", "count=5", "zip=02134", "flag=On", "mixed=1.10"]);
        assert_eq!(out, "1.10|5|02134|On|1.10");
    }

    #[test]
    fn an_undeclared_key_keeps_the_guess() {
        let fixture = with_document("setters-guess", "---\ntitle: T\n---\n{{ version }} {{ count + 1 }}\n");
        assert_eq!(composed(&fixture, &["doc.md", "version=1.10", "count=5"]), "1.1 6");
    }

    #[test]
    fn a_quoted_string_decodes_once_and_containers_read_alike() {
        let fixture = with_document("setters-forms", "---\ntitle: T\n---\n{{ a }}|{{ b.x }}|{{ c.max-width }}\n");
        assert_eq!(composed(&fixture, &["doc.md", "a=\"Alice\"", "b={x:1}", "c={ max-width: 50 }"]), "Alice|1|50");
    }

    #[test]
    fn a_nested_value_reads_through_its_declared_type() {
        let fixture = with_document("setters-nested-type", "---\n$schema:\n  meta:\n    version: string\n    \"<string>\": unknown\n---\n{{ meta.version }}\n");
        assert_eq!(composed(&fixture, &["doc.md", "meta.version=1.10"]), "1.10");
    }

    #[test]
    fn a_wrong_value_fails_schema_validation_and_is_not_placed_on_disk() {
        let fixture = with_document("setters-wrong", "---\n$schema:\n  count: number\ncount: 3\n---\n{{ count }}\n");
        let report = failure(&fixture, &["doc.md", "count=abc"]);
        let diagnostic = &report["diagnostics"][0];
        assert_eq!(diagnostic["code"], "dm.schema.type_mismatch", "{report:#}");
        let message = diagnostic["message"].as_str().unwrap();
        assert!(message.contains("abc") && message.contains("`count=abc`") && message.contains("at `count`"), "{message}");
        // The command-line value was never at the on-disk `count: 3`.
        assert_eq!(diagnostic["source"]["line"], Value::Null, "{report:#}");
        assert_eq!(diagnostic["source"]["column"], Value::Null, "{report:#}");
    }

    #[test]
    fn null_is_accepted_only_where_the_type_permits() {
        let fixture = with_document("setters-null", "---\n$schema:\n  a: string\n---\n[{{ a }}]\n");
        assert_eq!(composed(&fixture, &["doc.md", "a=null"]), "[]");
        let fixture = with_document("setters-non-null", "---\n$schema:\n  b: string(non-null)\nb: text\n---\n{{ b }}\n");
        let report = failure(&fixture, &["doc.md", "b=null"]);
        assert!(report["diagnostics"][0]["code"].as_str().unwrap().starts_with("dm.schema."), "{report:#}");
    }

    #[test]
    fn a_closed_object_refuses_an_undeclared_child_and_a_pattern_opens_it() {
        let closed = with_document("setters-closed", "---\n$schema:\n  author:\n    name: string\nauthor:\n  name: Ann\n---\n{{ author.name }}\n");
        let report = failure(&closed, &["doc.md", "author.mistake=1"]);
        assert_eq!(report["diagnostics"][0]["code"], "dm.schema.unknown_key", "{report:#}");
        assert!(report["diagnostics"][0]["message"].as_str().unwrap().contains("`author.mistake=1`"), "{report:#}");

        let open = with_document(
            "setters-open",
            "---\n$schema:\n  author:\n    name: string\n    \"<string>\": unknown\nauthor:\n  name: Ann\n---\n{{ author.mistake }}\n",
        );
        assert_eq!(composed(&open, &["doc.md", "author.mistake=1"]), "1");
    }

    #[test]
    fn an_undeclared_child_beside_an_authored_one_is_still_given_to_its_setter() {
        let fixture = with_document(
            "setters-closed-beside",
            "---\n$schema:\n  author:\n    name: string\nauthor:\n  old: 1\n---\nbody\n",
        );
        let report = failure(&fixture, &["doc.md", "author.mistake=1"]);
        let diagnostic = &report["diagnostics"][0];
        assert_eq!(diagnostic["code"], "dm.schema.unknown_key", "{report:#}");
        assert!(diagnostic["message"].as_str().unwrap().contains("`author.mistake=1`"), "{report:#}");
        assert_eq!(diagnostic["source"]["line"], Value::Null, "{report:#}");
    }

    #[test]
    fn a_required_child_a_more_specific_setter_supplies_is_satisfied() {
        let fixture = with_document(
            "setters-required-child",
            "---\n$schema:\n  author:\n    name: string(required)\n    email: string(required)\n---\n{{ author.name }}\n",
        );
        assert_eq!(composed(&fixture, &["doc.md", "author={email:\"a@x\"}", "author.name=Bob"]), "Bob");
    }
}

mod selecting_a_schema {
    use super::*;

    const DOCUMENT: &str = "---\ntitle: T\n---\n{{ edition }}\n";
    const LANG_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  lang: enum(rust; required)\n$schema:\n  edition: string\n";

    #[test]
    fn a_setter_selects_the_trigger_that_reads_another_in_either_order() {
        let fixture = with_document("setters-trigger", DOCUMENT);
        with_repository(&fixture);
        fixture.write_file("cwd/schemas/lang.trigger.yaml", LANG_TRIGGER);
        assert_eq!(composed(&fixture, &["doc.md", "lang=rust", "edition=1.10"]), "1.10");
        assert_eq!(composed(&fixture, &["doc.md", "edition=1.10", "lang=rust"]), "1.10");
        // Control: without the selecting setter the key is undeclared.
        assert_eq!(composed(&fixture, &["doc.md", "edition=1.10"]), "1.1");
    }

    #[test]
    fn a_set_that_replaces_the_schema_selects_the_same_way() {
        let fixture = with_document("setters-set-schema", "---\n$schema:\n  version: number\n---\n{{ version }}\n");
        let out = composed(&fixture, &["doc.md", "--set", r#"{"$schema":{"version":"string"}}"#, "version=1.10"]);
        assert_eq!(out, "1.10");
    }

    #[test]
    fn a_trigger_on_a_converted_type_fails_clearly_with_no_retry() {
        let fixture = with_document("setters-converted", "---\n$schema:\n  count: string\n---\n{{ count }}\n");
        with_repository(&fixture);
        fixture.write_file(
            "cwd/schemas/count.trigger.yaml",
            "kind: schema-trigger\nmatch:\n  count: string(required)\n$schema:\n  extra: string(required)\n",
        );
        let report = failure(&fixture, &["doc.md", "count=5"]);
        let diagnostic = &report["diagnostics"][0];
        assert!(diagnostic["code"].as_str().unwrap().starts_with("dm.schema."), "{report:#}");
        let message = diagnostic["message"].as_str().unwrap();
        assert!(message.contains("extra"), "{message}");
        // The missing key is the trigger's, and no setter supplied it.
        assert!(!message.contains("from the setter"), "{message}");
    }
}

mod reaching_children {
    use super::*;

    #[test]
    fn a_child_reads_a_setter_through_its_own_schema() {
        let fixture = with_document("setters-child", "---\n$schema:\n  v: number\n---\nparent {{ v }}\n\n::file ./child.md\n");
        fixture.write_file("cwd/child.md", "---\n$schema:\n  v: string\n---\nchild {{ v }}\n");
        let out = composed(&fixture, &["doc.md", "v=1.10"]);
        assert!(out.contains("parent 1.1"), "{out}");
        assert!(out.contains("child 1.10"), "{out}");
    }

    #[test]
    fn a_nested_setter_keeps_a_childs_siblings() {
        let fixture = with_document("setters-child-nested", "---\ntitle: T\n---\n::file ./child.md\n");
        fixture.write_file("cwd/child.md", "---\na:\n  b: old\n  c: kept\n---\n{{ a.b }} {{ a.c }}\n");
        assert_eq!(composed(&fixture, &["doc.md", "a.b=new"]), "new kept");
    }

    #[test]
    fn a_path_a_child_cannot_hold_fails_the_run_and_names_the_child() {
        let fixture = with_document("setters-child-scalar", "---\ntitle: T\n---\n::file ./child.md\n");
        fixture.write_file("cwd/child.md", "---\na: 1\n---\nchild\n");
        let report = failure(&fixture, &["doc.md", "a.b=2"]);
        let message = assert_setter_failure(&report, "a.b=2", "a.b");
        assert!(message.contains("child.md"), "{message}");
    }

    #[test]
    fn a_setter_chooses_a_transclusion_target() {
        let fixture = with_document("setters-target", "---\nwhich:\n  name: other\n---\n::file ./{{ which.name }}.md\n");
        fixture.write_file("cwd/chosen.md", "chosen\n");
        assert_eq!(composed(&fixture, &["doc.md", "which.name=chosen"]), "chosen");
    }
}

mod origins {
    use super::*;

    const SCHEMA: &str = "$schema:\n  author:\n    name: string\n    avatar: file\n";

    fn fixture_with_files(name: &str, avatar: &str) -> CliProcessFixture {
        let fixture = CliProcessFixture::named(name);
        fixture.write_file(
            "cwd/sub/doc.md",
            &format!("---\n{SCHEMA}author:\n  name: Ann\n  avatar: {avatar}\n---\n{{{{ author.name }}}} {{{{ author.avatar }}}}\n"),
        );
        fixture.write_file("cwd/sub/beside.png", "doc");
        fixture.write_file("cwd/launch.png", "launch");
        fixture
    }

    #[test]
    fn an_authored_file_value_beside_a_setter_keeps_its_own_origin() {
        let fixture = fixture_with_files("setters-origin-authored", "./beside.png");
        let out = composed(&fixture, &["sub/doc.md", "author.name=Bob"]);
        assert!(out.starts_with("Bob "), "{out}");
        assert!(out.contains("beside.png"), "{out}");
    }

    #[test]
    fn a_nested_file_setter_resolves_from_the_launch_directory() {
        let fixture = fixture_with_files("setters-origin-nested", "./beside.png");
        let out = composed(&fixture, &["sub/doc.md", "author.avatar=launch.png"]);
        let launch = biscuit_file::canonicalize_simplified(fixture.cwd()).unwrap().join("launch.png");
        assert!(out.ends_with(&biscuit_file::to_portable_string(&launch)), "{out}");
    }

    #[test]
    fn a_nested_file_setter_does_not_fall_back_to_the_documents_directory() {
        let fixture = fixture_with_files("setters-origin-miss", "./beside.png");
        let out = composed(&fixture, &["sub/doc.md", "author.avatar=beside.png"]);
        assert!(!out.contains("sub/beside.png") && !out.contains(r"sub\beside.png"), "{out}");
    }

    #[test]
    fn a_top_level_file_setter_keeps_resolving_from_the_launch_directory() {
        let fixture = CliProcessFixture::named("setters-origin-top");
        fixture.write_file("cwd/sub/doc.md", "---\n$schema:\n  spec: file\n---\n{{ spec }}\n");
        fixture.write_file("cwd/launch.md", "launch");
        let out = composed(&fixture, &["sub/doc.md", "spec=launch.md"]);
        assert!(out.contains("launch.md"), "{out}");
    }

    const UNION: &str = "kind: schema\ntypes:\n  node:\n    avatar: file(eager; match(./assets/*.png))\n    cover: file(eager; match(./assets/*.png))\n    children: node[]\n$schema:\n  - kind: literal(image)\n    author: node\n  - kind: literal(other)\n    title: string(required)\n";

    fn union_fixture(name: &str) -> CliProcessFixture {
        let fixture = CliProcessFixture::named(name);
        fixture.write_file("cwd/s.yaml", UNION);
        fixture.write_file("cwd/assets/avatar.png", "launch");
        fixture.write_file(
            "cwd/sub/doc.md",
            "---\n$schema: ../s.yaml\nkind: image\nauthor:\n  cover: ./assets/mine.png\n  children: []\n---\n{{ author.avatar }} {{ author.cover }}\n",
        );
        fixture.write_file("cwd/sub/assets/mine.png", "document");
        fixture
    }

    #[test]
    fn a_union_arm_selects_with_the_origin_of_a_nested_file_setter() {
        let fixture = union_fixture("setters-origin-union");
        let out = composed(&fixture, &["sub/doc.md", "author.avatar=assets/avatar.png"]);
        let launch = biscuit_file::canonicalize_simplified(fixture.cwd()).unwrap().join("assets").join("avatar.png");
        let (avatar, cover) = out.split_once(' ').unwrap();
        assert_eq!(avatar, biscuit_file::to_portable_string(&launch));
        assert!(!cover.contains(fixture.cwd().to_str().unwrap()) && cover.ends_with("mine.png"), "{out}");
    }

    #[test]
    fn a_union_arm_selects_with_the_origin_of_a_whole_set_object() {
        let fixture = union_fixture("setters-origin-union-set");
        let out = composed(&fixture, &["sub/doc.md", "--set", r#"{"author":{"avatar":"assets/avatar.png","cover":"assets/avatar.png","children":[]}}"#]);
        let launch = biscuit_file::canonicalize_simplified(fixture.cwd()).unwrap().join("assets").join("avatar.png");
        assert_eq!(out, format!("{0} {0}", biscuit_file::to_portable_string(&launch)));
    }
}

mod pending_values {
    use super::*;

    #[test]
    fn a_template_in_a_setter_is_authored_at_any_depth() {
        let fixture = with_document("setters-template", "---\ny: 2\na:\n  c: 3\n---\n{{ x }} {{ a.b }} {{ a.c }}\n");
        assert_eq!(composed(&fixture, &["doc.md", "x={{ y + 1 }}", "a.b={{ y + 2 }}"]), "3 4 3");
    }

    #[test]
    fn a_template_under_a_number_stays_pending_and_binds_after() {
        let fixture = with_document("setters-template-typed", "---\n$schema:\n  n: number\ny: 2\n---\n{{ n + 1 }}\n");
        assert_eq!(composed(&fixture, &["doc.md", "n={{ y }}"]), "3");
    }

    #[test]
    fn a_failing_template_names_its_setter() {
        let fixture = with_document("setters-template-failure", "---\na:\n  c: ok\n---\n{{ a.b }}\n");
        let (code, _, stderr) = run(&fixture, &["compose", "doc.md", "a.b={{ nosuch_function() }}"]);
        assert_eq!(code, Some(1), "{stderr}");
        assert!(stderr.contains("a.b={{ nosuch_function() }}"), "{stderr}");
    }

    #[test]
    fn the_shell_report_sees_a_command_a_setter_supplies() {
        let fixture = with_document("setters-shell", "---\ntitle: T\n---\n{{ x }}\n");
        let (code, stdout, stderr) = run(&fixture, &["compose", "doc.md", "x=$(echo from-setter)", "--shell"]);
        assert_eq!(code, Some(0), "{stdout}{stderr}");
        assert!(stdout.contains("echo from-setter"), "{stdout}");
    }

    #[test]
    fn the_shell_report_fails_on_a_setter_the_document_cannot_hold() {
        let fixture = with_document("setters-shell-failure", "---\nitems: [a]\n---\nbody\n");
        let (code, _, stderr) = run(&fixture, &["compose", "doc.md", "items.9=x", "--shell"]);
        assert_eq!(code, Some(1), "{stderr}");
        assert!(stderr.contains("items.9=x"), "{stderr}");
    }
}

#[test]
fn a_data_output_reports_success_with_the_setter_applied() {
    let fixture = with_document("setters-envelope", AUTHOR);
    let (code, stdout, stderr) = run(&fixture, &["compose", "doc.md", "author.name=Alice", "--output", "json"]);
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["outcome"], json!("success"), "{report:#}");
    assert!(report["document"].as_str().unwrap().contains("Alice a@x"), "{report:#}");
}

/// A setter is read through the schema the request selected, not the compiled one.
mod selected_baseline {
    use super::*;

    const CUSTOM: &str = "$schema:\n  version: string\n";
    const DOCUMENT: &str = "---\n$schema:\n  other: string\n---\n{{ title }}\n";

    fn frontmatter(fixture: &CliProcessFixture, args: &[&str]) -> Value {
        let mut full = args.to_vec();
        full.extend_from_slice(&["--frontmatter", "--output", "markdown"]);
        let text = composed(fixture, &full);
        let yaml = text.split("---").nth(1).unwrap_or_default();
        serde_yaml_ng::from_str(yaml).unwrap()
    }

    #[test]
    fn a_custom_baseline_flag_declares_the_type() {
        let fixture = with_document("setters-custom-baseline", "---\ntitle: T\n---\n");
        fixture.write_file("cwd/base.yaml", CUSTOM);
        let found = frontmatter(&fixture, &["doc.md", "version=1.10", "--baseline-schema", "base.yaml"]);
        assert_eq!(found["version"], json!("1.10"));
    }

    #[test]
    fn a_disabled_baseline_flag_leaves_a_compiled_key_undeclared() {
        let fixture = with_document("setters-disabled-baseline", DOCUMENT);
        let found = frontmatter(&fixture, &["doc.md", "title=1.10", "--disable", "baseline-schema"]);
        assert_eq!(found["title"], json!(1.1));
    }

    #[test]
    fn a_disabled_baseline_environment_leaves_a_compiled_key_undeclared() {
        let fixture = with_document("setters-disabled-baseline-env", DOCUMENT);
        let output = fixture
            .command_builder()
            .application_input("DARKMATTER_NO_BASELINE_SCHEMA", "1")
            .build()
            .args(["compose", "doc.md", "title=1.10", "--frontmatter"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("title: 1.1\n"), "{text}");
    }

    #[test]
    fn a_document_schema_declares_the_type() {
        let fixture = with_document("setters-document-schema", "---\n$schema:\n  version: string\n---\n");
        let found = frontmatter(&fixture, &["doc.md", "version=1.10"]);
        assert_eq!(found["version"], json!("1.10"));
    }

    #[test]
    fn a_raw_json_schema_document_declares_the_type() {
        let fixture = with_document("setters-raw-document", "---\n$schema: ./doc.json\n---\n");
        fixture.write_file(
            "cwd/doc.json",
            r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"version":{"type":"string"}}}"#,
        );
        let found = frontmatter(&fixture, &["doc.md", "version=1.10"]);
        assert_eq!(found["version"], json!("1.10"));
    }

    #[test]
    fn a_raw_json_schema_trigger_declares_the_type() {
        let fixture = with_document("setters-raw-trigger", "---\nlang: rust\n---\n");
        with_repository(&fixture);
        fixture.write_file(
            "cwd/schemas/lang.trigger.yaml",
            "kind: schema-trigger\nmatch:\n  lang: enum(rust; required)\n$schema: ./payload.json\n",
        );
        fixture.write_file(
            "cwd/schemas/payload.json",
            r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"version":{"type":"string"}}}"#,
        );
        let found = frontmatter(&fixture, &["doc.md", "version=1.10"]);
        assert_eq!(found["version"], json!("1.10"));
    }

    #[test]
    fn a_disabled_trigger_declares_nothing() {
        let fixture = with_document("setters-disabled-trigger", "---\nlang: rust\n---\n");
        with_repository(&fixture);
        fixture.write_file(
            "cwd/schemas/lang.trigger.yaml",
            "kind: schema-trigger\nmatch:\n  lang: enum(rust; required)\n$schema:\n  edition: string\n",
        );
        let found = frontmatter(&fixture, &["doc.md", "edition=1.10", "--disable", "schema-triggers"]);
        assert_eq!(found["edition"], json!(1.1));
    }
}

/// `md set` and `md compose` read one value the same way.
mod set_parity {
    use super::*;

    const DOCUMENT: &str = concat!(
        "---\n",
        "$schema:\n",
        "  x: unknown\n",
        "  count: number\n",
        "  author:\n",
        "    age: number\n",
        "    any: unknown\n",
        "  scores:\n",
        "    \"<string>\": number\n",
        "  names:\n",
        "    \"<string>\": string\n",
        "author: {age: 1, any: 1}\n",
        "scores: {a: 1}\n",
        "names: {a: b}\n",
        "---\n",
        "Body\n",
    );

    fn frontmatter_of(text: &str) -> Value {
        serde_yaml_ng::from_str(text.split("---").nth(1).unwrap_or_default()).unwrap()
    }

    fn at<'v>(frontmatter: &'v Value, key: &str) -> &'v Value {
        key.split('.').fold(frontmatter, |value, segment| &value[segment])
    }

    /// One value through both commands, each a composition or an edit of its own.
    fn both_read(key: &str, raw: &str, expected: &Value) {
        let fixture = with_document("setters-set-parity", DOCUMENT);
        let setter = format!("{key}={raw}");
        let composed_text = composed(&fixture, &["doc.md", &setter, "--frontmatter", "--output", "markdown"]);
        assert_eq!(at(&frontmatter_of(&composed_text), key), expected, "compose {setter}");

        let (code, set_text, stderr) = run(&fixture, &["set", "doc.md", key, raw, "--output", "markdown"]);
        assert_eq!(code, Some(0), "set {setter}: {stderr}");
        assert_eq!(at(&frontmatter_of(&set_text), key), expected, "set {setter}");
    }

    #[test]
    fn unknown_keeps_text() {
        both_read("x", "1.10", &json!("1.10"));
        both_read("author.any", "1.10", &json!("1.10"));
    }

    #[test]
    fn a_container_under_unknown_is_a_container() {
        both_read("x", "{a:1}", &json!({ "a": 1 }));
    }

    #[test]
    fn a_quoted_number_is_a_number() {
        both_read("count", r#""5""#, &json!(5));
        both_read("author.age", r#""5""#, &json!(5));
    }

    #[test]
    fn a_pattern_key_declares_its_type() {
        both_read("scores.a", r#""5""#, &json!(5));
        both_read("names.a", "1.10", &json!("1.10"));
    }

    /// `$ref` and `allOf` project to `unknown`, yet neither keeps text.
    #[test]
    fn a_number_a_raw_schema_cannot_name_stays_a_number() {
        let fixture = with_document("setters-raw-parity", "---\n$schema: ./schema.json\n---\nBody\n");
        fixture.write_file(
            "cwd/schema.json",
            r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object",
                "$defs":{"number":{"type":"number"}},
                "properties":{"by_ref":{"$ref":"#/$defs/number"},"all_of":{"allOf":[{"type":"number"},{"minimum":0}]}}}"##,
        );
        for (key, setter) in [("by_ref", "by_ref=5"), ("all_of", "all_of=5")] {
            let text = composed(&fixture, &["doc.md", setter, "--frontmatter", "--output", "markdown"]);
            assert_eq!(at(&frontmatter_of(&text), key), &json!(5), "{setter}");
        }
    }

    /// A `$ref`'d string's constraint may ask something of the text's content, so
    /// the text itself is what is asked about.
    #[test]
    fn text_a_raw_constrained_string_accepts_keeps_its_spelling() {
        let fixture = with_document("setters-raw-constrained", "---\n$schema: ./schema.json\n---\nBody\n");
        fixture.write_file(
            "cwd/schema.json",
            r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object",
                "$defs":{"version":{"type":"string","pattern":"^[0-9.]+$"},"pick":{"enum":["1.10","2.0"]}},
                "properties":{"version":{"$ref":"#/$defs/version"},"pick":{"$ref":"#/$defs/pick"}}}"##,
        );
        for (key, setter) in [("version", "version=1.10"), ("pick", "pick=1.10")] {
            let text = composed(&fixture, &["doc.md", setter, "--frontmatter", "--output", "markdown"]);
            assert_eq!(at(&frontmatter_of(&text), key), &json!("1.10"), "{setter}");
        }
    }

    /// `allOf` drives no conversion, and a quoted value says it is a string.
    #[test]
    fn a_quoted_number_behind_an_all_of_is_refused() {
        let fixture = with_document("setters-raw-all-of", "---\n$schema: ./schema.json\n---\nBody\n");
        fixture.write_file(
            "cwd/schema.json",
            r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object",
                "properties":{"all_of":{"allOf":[{"type":"number"},{"minimum":0}]}}}"##,
        );
        let envelope = failure(&fixture, &["doc.md", r#"all_of="5""#]);
        assert_eq!(envelope["diagnostics"][0]["code"], json!("dm.schema.type_mismatch"), "{envelope}");
    }

    #[test]
    fn an_undeclared_key_keeps_its_guess_and_its_quotes() {
        both_read("other", "1.10", &json!(1.1));
        both_read("other", r#""5""#, &json!("5"));
    }
}
