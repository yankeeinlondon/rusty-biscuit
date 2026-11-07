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
        assert!(out.ends_with(launch.to_str().unwrap()), "{out}");
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
