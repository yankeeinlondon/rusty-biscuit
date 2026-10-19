//! Public validation-report evidence for item unions (`U[]`): problem
//! descriptions are found through `items.anyOf`, a literal tag narrows an item
//! to its arm exactly as it narrows a property union, and every other item
//! keeps ordinary union reporting.
//!
//! Each test fails if the report walker reads the `items.anyOf` wrapper as a
//! plain object: descriptions below an item come back `None`, or a tagged item
//! reports every arm's mismatches instead of its own arm's.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::DarkmatterSchemas;

use crate::request_support::context_at;
use crate::union_fixtures::{
    CONTENT_POLICY_FILE, CONTENT_POLICY_LIST, NESTED_UNION_FILE, NESTED_UNION_TYPES,
    OVERLAPPING_TYPES, TAGGED_TYPES, content_policy_union_schema, write_files,
};

/// A tagged union whose arms describe their fields. `shell` and `say` share
/// the optional tag `mode` and the field `body` (typed and described
/// differently); the two `dup` arms repeat one literal.
const DESCRIBED_TYPES: &str = "kind: schema\ntypes:\n  step:\n    - \"{ kind: literal(shell; required), mode: literal(run), run: string(required) -> the shell command, body: string -> the shell body }\"\n    - \"{ kind: literal(say; required), mode: literal(speak), text: string(required) -> the words to speak, body: number -> the say body }\"\n    - \"{ kind: literal(dup; required), x: number }\"\n    - \"{ kind: literal(dup; required), y: number }\"\n";

/// [`OVERLAPPING_TYPES`] with field descriptions.
const DESCRIBED_OVERLAPPING_TYPES: &str = "kind: schema\ntypes:\n  entry:\n    - \"{ name: string(required) -> the entry name }\"\n    - \"{ name: string(required) -> the entry name, size: number -> bytes on disk }\"\n";

/// A tagged arm beside an arm whose `kind` is any string.
const LOOSE_ARM_TYPES: &str = "kind: schema\ntypes:\n  step:\n    - \"{ kind: literal(a; required), x: number(required) -> the amount }\"\n    - \"{ kind: literal(b; required) }\"\n    - \"{ kind: string(required), note: string }\"\n";

/// One reported problem: path, message, missing property, description.
type Reported = (String, String, Option<String>, Option<String>);

fn temp_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

/// Writes a document whose `$schema` mapping body is `schema`, loads it from
/// disk, and validates it. Asserts validation leaves the file unchanged and
/// returns the problems sorted.
fn report(dir: &Path, schema: &str, frontmatter: &str) -> Vec<Reported> {
    let path = dir.join("doc.md");
    let text = format!("---\n$schema:\n{schema}{frontmatter}---\nBody\n");
    std::fs::write(&path, &text).expect("write document");
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path.clone()));
    let report = DarkmatterSchemas::new(context_at(dir))
        .validate(&markdown)
        .expect("schema resolves");
    assert_eq!(std::fs::read_to_string(&path).expect("read document"), text);
    assert_eq!(report.valid, report.problems.is_empty());
    let mut problems: Vec<Reported> = report
        .problems
        .into_iter()
        .map(|problem| (problem.path, problem.message, problem.property, problem.description))
        .collect();
    problems.sort();
    problems
}

fn problem(path: &str, message: &str, property: Option<&str>, description: Option<&str>) -> Reported {
    (
        path.to_string(),
        message.to_string(),
        property.map(str::to_string),
        description.map(str::to_string),
    )
}

/// The problems under `prefix`, re-rooted at `to`.
fn rerooted(problems: &[Reported], prefix: &str, to: &str) -> Vec<Reported> {
    let mut out: Vec<Reported> = problems
        .iter()
        .filter_map(|(path, message, property, description)| {
            let rest = path.strip_prefix(prefix)?;
            (rest.is_empty() || rest.starts_with('/')).then(|| {
                (format!("{to}{rest}"), message.clone(), property.clone(), description.clone())
            })
        })
        .collect();
    out.sort();
    out
}

#[test]
fn content_policy_rule_lookup_reads_the_union_inside_the_object_arm() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    write_files(&root, &[(CONTENT_POLICY_FILE, &content_policy_union_schema(shipped))]);
    let schema = format!(
        "  content_policy: \"{CONTENT_POLICY_LIST}\"\n  rule: \"short_form(required)@./{CONTENT_POLICY_FILE}\"\n"
    );
    let short_form = "Never triggers | Always triggers | Interval from the default date property | Interval from a property | Interval from an inline date | Fixed deadline | Deadline held in a property | Content change of a file, fingerprint held in a property";

    let null_rule = report(
        &root,
        &schema,
        "content_policy: [\"ValidFor(3mo)\", {rule: null, action: archive}]\nrule: \"ValidFor(3mo)\"\n",
    );
    let rule_union = format!("a union type of: {short_form}");
    assert_eq!(
        null_rule,
        [problem(
            "/content_policy/1/rule",
            "null is not valid under any of the schemas listed in the 'anyOf' keyword",
            None,
            Some(&rule_union),
        )]
    );

    // The same `short_form(required)` used as a property is described alike.
    let property = report(&root, &schema, "rule: null\n");
    assert_eq!(rerooted(&property, "/rule", "/content_policy/1/rule"), null_rule);

    // A missing `rule` fails every arm at the item, which is described by all
    // of its alternatives.
    let missing_rule = report(
        &root,
        &schema,
        "content_policy: [\"ValidFor(3mo)\", {action: archive}]\nrule: \"ValidFor(3mo)\"\n",
    );
    let item_union = format!("a union type of: {short_form} | object");
    assert_eq!(
        missing_rule,
        [problem(
            "/content_policy/1",
            "{\"action\":\"archive\"} is not valid under any of the schemas listed in the 'anyOf' keyword",
            None,
            Some(&item_union),
        )]
    );
}

#[test]
fn a_literal_tag_narrows_each_item_to_its_own_arm() {
    let (_dir, root) = temp_root();
    write_files(&root, &[("tagged.yaml", TAGGED_TYPES), ("described.yaml", DESCRIBED_TYPES)]);

    let tagged = report(
        &root,
        "  steps: step[]@./tagged.yaml\n",
        "steps: [{kind: shell}, {kind: say}, {kind: shell, run: ls}, {kind: say, run: ls}]\n",
    );
    assert_eq!(
        tagged,
        [
            problem("/steps/0", "\"run\" is a required property", Some("run"), None),
            problem("/steps/1", "\"text\" is a required property", Some("text"), None),
            problem("/steps/3", "\"text\" is a required property", Some("text"), None),
            problem(
                "/steps/3",
                "Additional properties are not allowed ('run' was unexpected)",
                None,
                None
            ),
        ]
    );

    // Both arms declare `body`; each item's tag picks whose type and
    // description apply.
    let described = report(
        &root,
        "  steps: step[]@./described.yaml\n",
        "steps: [{kind: shell, run: ls, body: [1]}, {kind: say, text: hi, body: x}, {kind: say}]\n",
    );
    assert_eq!(
        described,
        [
            problem("/steps/0/body", "[1] is not of type \"string\"", None, Some("the shell body")),
            problem("/steps/1/body", "\"x\" is not of type \"number\"", None, Some("the say body")),
            problem(
                "/steps/2",
                "\"text\" is a required property",
                Some("text"),
                Some("the words to speak")
            ),
        ]
    );
}

#[test]
fn missing_unknown_duplicate_and_conflicting_tags_keep_ordinary_union_reporting() {
    let (_dir, root) = temp_root();
    write_files(&root, &[("described.yaml", DESCRIBED_TYPES)]);
    let schema = "  steps: step[]@./described.yaml\n";
    let run = problem("/steps/0/run", "1 is not of type \"string\"", None, Some("the shell command"));
    let expected = |tag: &str| problem("/steps/0/kind", &format!("\"{tag}\" was expected"), None, None);

    assert_eq!(report(&root, schema, "steps: [{run: 1}]\n"), std::slice::from_ref(&run), "missing tag");
    assert_eq!(
        report(&root, schema, "steps: [{kind: dance, run: 1}]\n"),
        [expected("dup"), expected("say"), expected("shell"), run.clone()],
        "unknown tag"
    );
    assert_eq!(
        report(&root, schema, "steps: [{kind: dup, run: 1}]\n"),
        [expected("say"), expected("shell"), run.clone()],
        "a literal two arms share"
    );
    assert_eq!(
        report(&root, schema, "steps: [{kind: shell, mode: speak, run: 1}]\n"),
        [
            expected("dup"),
            expected("say"),
            problem("/steps/0/mode", "\"run\" was expected", None, None),
            run,
        ],
        "tags selecting different arms"
    );
}

#[test]
fn overlapping_item_arms_accept_either_shape_and_report_a_field_once() {
    let (_dir, root) = temp_root();
    write_files(
        &root,
        &[("overlap.yaml", OVERLAPPING_TYPES), ("described.yaml", DESCRIBED_OVERLAPPING_TYPES)],
    );
    assert_eq!(
        report(&root, "  e: entry[]@./overlap.yaml\n", "e: [{name: a}, {name: b, size: 2}]\n"),
        []
    );

    assert_eq!(
        report(&root, "  e: entry[]@./described.yaml\n", "e: [{name: 1}, {name: b, size: x}]\n"),
        [
            problem("/e/0/name", "1 is not of type \"string\"", None, Some("the entry name")),
            problem("/e/1/size", "\"x\" is not of type \"number\"", None, Some("bytes on disk")),
        ]
    );
}

#[test]
fn an_item_another_arm_accepts_stays_valid_despite_its_tag() {
    let (_dir, root) = temp_root();
    write_files(&root, &[("loose.yaml", LOOSE_ARM_TYPES)]);
    let schema = "  steps: step[]@./loose.yaml\n  one: step@./loose.yaml\n";

    // `{kind: a}` fails its tagged arm but the loose arm accepts it.
    assert_eq!(report(&root, schema, "steps: [{kind: a}]\none: {kind: a}\n"), []);

    // No arm accepts `x: no`, so the tag narrows the report to arm `a`.
    let narrowed = report(&root, schema, "steps: [{kind: a, x: no}]\none: {kind: a, x: no}\n");
    let x = problem("/steps/0/x", "\"no\" is not of type \"number\"", None, Some("the amount"));
    assert_eq!(rerooted(&narrowed, "/steps/0", "/steps/0"), std::slice::from_ref(&x));
    assert_eq!(rerooted(&narrowed, "/one", "/steps/0"), [x]);
}

#[test]
fn an_item_union_reports_like_the_same_union_used_as_a_property() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    write_files(
        &root,
        &[
            (CONTENT_POLICY_FILE, &content_policy_union_schema(shipped)),
            ("described.yaml", DESCRIBED_TYPES),
        ],
    );
    let cases: [(&str, &[&str]); 2] = [
        (
            "  list: step[]@./described.yaml\n  one: step@./described.yaml\n",
            &[
                "{kind: shell}",
                "{kind: say, run: ls}",
                "{kind: shell, run: ls, body: [1]}",
                "{kind: say, text: hi, body: x}",
                "{run: 1}",
                "{kind: dance, run: 1}",
                "{kind: dup, run: 1}",
                "{kind: shell, mode: speak, run: 1}",
                "{kind: shell, run: ls}",
            ],
        ),
        (
            "  list: policy[]@./content-policy.yaml\n  one: policy@./content-policy.yaml\n",
            &[
                "\"ValidFor(3mo)\"",
                "\"Duration(3mo)\"",
                "{action: archive}",
                "{rule: null, action: archive}",
                "{rule: \"ValidFor(3mo)\", action: delete}",
                "{rule: \"Duration(3mo)\", action: archive, extra: 1}",
            ],
        ),
    ];
    for (schema, values) in cases {
        for value in values {
            let problems = report(&root, schema, &format!("list: [{value}]\none: {value}\n"));
            let item = rerooted(&problems, "/list/0", "/x");
            let property = rerooted(&problems, "/one", "/x");
            assert_eq!(item, property, "{value}");
            assert_eq!(item.len() + property.len(), problems.len(), "{value}: {problems:?}");
        }
    }
}

#[test]
fn a_literal_tag_inside_a_postfixed_nested_union_narrows_as_property_and_item() {
    let (_dir, root) = temp_root();
    write_files(&root, &[(NESTED_UNION_FILE, NESTED_UNION_TYPES)]);
    let reported = |type_expr: &str, frontmatter: &str| {
        report(&root, &format!("  p: \"{type_expr}\"\n"), frontmatter)
    };

    let direct = reported("direct_tag@./types.yaml", "p: {kind: a}\n");
    assert_eq!(direct.len(), 1, "{direct:?}");
    assert_eq!(direct[0].0, "/p");
    assert_eq!(direct[0].2.as_deref(), Some("a"), "the selected arm's missing field: {direct:?}");
    assert_eq!(reported("nested_tag@./types.yaml", "p: {kind: a}\n"), direct);

    let direct_items = reported("direct_tag[]@./types.yaml", "p: [{kind: a}]\n");
    assert_eq!(direct_items.len(), 1, "{direct_items:?}");
    assert_eq!(direct_items[0].0, "/p/0");
    assert_eq!(direct_items[0].2.as_deref(), Some("a"), "{direct_items:?}");
    assert_eq!(reported("nested_tag[]@./types.yaml", "p: [{kind: a}]\n"), direct_items);
}
