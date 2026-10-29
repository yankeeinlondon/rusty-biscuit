//! DMLS against Claudine's recursive lifecycle schemas: the shared
//! `claudine-schemas` fixture corpus gets the verdicts and source positions
//! `md schema validate` reports, completion and hover reach generated verbs
//! and long-form parameters four conditionals deep in each event's `then` and
//! `else`, and none of it runs an action.
//!
//! Documents live in the [`Fixture`] repository of
//! [`crate::claudine_schema_discovery`], so Claudine's trigger payloads type
//! them with no `$schema` pointer, as they type a real Claudine prompt.

use crate::claudine_schema_discovery::{Ask, Fixture, editor_answers_at, has, initialize_params, open, uri_of};
use crate::common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use common::{LspFixture, LspWorkspace};
use dmls::capabilities::ClientProfile;
use dmls::config::DmlsConfig;
use dmls::context::RepositoryContexts;
use dmls::graph::{NoContexts, WorkspaceGraph};
use dmls::overlay::OverlayState;
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use lsp_types::{InitializeParams, Uri};
use serde_json::{Value, json};

/// The byte offset just past the last of `path`'s segments, each found after
/// the one before it. `["\nstart:", "else:", "- "]` is the cursor after the
/// first item marker of the first `else` under `start`.
fn after(text: &str, path: &[&str]) -> usize {
    path.iter().fold(0, |cursor, segment| {
        cursor + text[cursor..].find(segment).unwrap_or_else(|| panic!("`{segment}` after byte {cursor} in {text:?}"))
            + segment.len()
    })
}

/// The byte offset where the last of `path`'s segments starts.
fn at(text: &str, path: &[&str]) -> usize {
    after(text, path) - path.last().expect("a non-empty path").len()
}

/// The LSP position (ASCII fixtures, so bytes are UTF-16 units) of `offset`.
fn position(text: &str, offset: usize) -> Value {
    let before = &text[..offset];
    json!({
        "line": before.matches('\n').count(),
        "character": offset - before.rfind('\n').map_or(0, |index| index + 1),
    })
}

/// The path to each event's innermost `then` (the fourth under the event) and
/// `else` (the first, since only the innermost conditional has one) in
/// `depth4-completion.md`.
fn innermost(event: &str, branch: &str) -> Vec<&'static str> {
    let event: &'static str = match event {
        "initialize" => "\ninitialize:",
        "start" => "\nstart:",
        "cancelled" => "\ncancelled:",
        "gate" => "\n    gate:",
        other => panic!("{other}"),
    };
    match branch {
        "then" => vec![event, "then:", "then:", "then:", "then:"],
        "else" => vec![event, "else:"],
        other => panic!("{other}"),
    }
}

/// `text` with the item just below the innermost `branch` of `event` emptied
/// to a bare `- `, and the cursor offset after that marker.
fn emptied(text: &str, event: &str, branch: &str) -> (String, usize) {
    let mut path = innermost(event, branch);
    path.push("- ");
    let cursor = after(text, &path);
    let line_end = cursor + text[cursor..].find('\n').expect("the item's line ends");
    (format!("{}{}", &text[..cursor], &text[line_end..]), cursor)
}

/// Every diagnostic DMLS publishes for each `(name, text)`, opened under the
/// Claudine package in one session.
fn published(documents: &[(&str, &str)]) -> Vec<Vec<Value>> {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let mut session = LspFixture::start_with_snapshot(&workspace, fixture.snapshot());
    session.initialize(initialize_params(&fixture.repo()));
    let found = documents
        .iter()
        .map(|(name, text)| {
            let uri = uri_of(&fixture.document(&format!("{name}.md"), text));
            open(&session, &uri, text);
            session.wait_for_diagnostics(&uri)
        })
        .collect();
    session.shutdown();
    found
}

fn is_schema(diagnostic: &Value) -> bool {
    diagnostic["code"].as_str().is_some_and(|code| code.starts_with("dm.schema."))
}

/// Asserts DMLS publishes no schema diagnostic for any of `documents`.
fn assert_no_schema_diagnostic(documents: &[(&str, &str)]) {
    let found = published(documents);
    for ((name, _), diagnostics) in documents.iter().zip(&found) {
        let schema: Vec<&Value> = diagnostics.iter().filter(|diagnostic| is_schema(diagnostic)).collect();
        assert!(schema.is_empty(), "{name}: {schema:#?}");
    }
}

/// The depth fixtures `md schema validate` accepts, both depth-six ones
/// included, raise no schema diagnostic in DMLS; nor does an alias (`speak`,
/// of `say`) in the innermost `then` and `else` of a depth-four `start`.
#[test]
fn the_shared_corpus_valid_depth_fixtures_raise_no_schema_diagnostic() {
    let depth4 = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth4-completion.md");
    let alias = depth4
        .replacen("- ensure_file: { file: notes.md, content: \"# Notes\" }", "- speak: \"then alias\"", 1)
        .replacen("- pr_list: { count: 3 }", "- speak: { text: \"else alias\" }", 1);
    assert_eq!(alias.matches("speak").count(), 2, "the alias replaced both innermost items");
    assert_no_schema_diagnostic(&[
        ("depth1", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth1.md")),
        ("depth3", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth3.md")),
        ("depth5", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth5.md")),
        ("depth6", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth6.md")),
        ("depth5-guarded-bundle", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth5-guarded-bundle.md")),
        ("depth4-completion", depth4),
        ("depth4-alias", alias.as_str()),
    ]);
}

/// The generated-verb fixtures `md schema validate` accepts (typed,
/// convertible, and deferred operands, `ensure_file`'s overloads, the
/// variadic `and`/`or`, and the numeric-recovery fallbacks the runtime
/// registration binds) raise no schema diagnostic in DMLS.
#[test]
fn the_shared_corpus_valid_generated_verb_fixtures_raise_no_schema_diagnostic() {
    assert_no_schema_diagnostic(&[
        ("generated-verb-typed-value", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-typed-value.md")),
        ("generated-verb-convertible-text", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-convertible-text.md")),
        ("generated-verb-deferred", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred.md")),
        ("ensure-file-overloads", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/ensure-file-overloads.md")),
        ("variadic-and-or", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/variadic-and-or.md")),
        ("generated-verb-numeric-recovery", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-numeric-recovery.md")),
    ]);
}

/// A single-action dictionary in long form at a stack root, in either
/// branch, and as a loop's one loose verb raises no schema diagnostic in
/// DMLS, as `md schema validate` and the lifecycle parser accept it.
#[test]
fn the_shared_corpus_single_action_dictionaries_raise_no_schema_diagnostic() {
    assert_no_schema_diagnostic(&[
        ("single-action-root-dictionary", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/single-action-root-dictionary.md")),
        ("single-action-branch-dictionaries", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/single-action-branch-dictionaries.md")),
        ("single-action-loop-loose", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/single-action-loop-loose.md")),
    ]);
}

/// Asserts DMLS rejects `text` with the set of instance paths in `expected`,
/// each starting at the node its path of segments reaches (the CLI's 1-based
/// line and column is this 0-based position plus one), with the needle in the
/// message where the verdict names the offending text.
fn assert_rejected_where_the_cli_rejects(name: &str, text: &str, expected: &[(&str, &[&str], &str)]) {
    let found = published(&[(name, text)]);
    let schema: Vec<&Value> = found[0].iter().filter(|diagnostic| is_schema(diagnostic)).collect();
    let paths: BTreeSet<&str> = schema.iter().filter_map(|diagnostic| diagnostic["data"]["instance_path"].as_str()).collect();
    let expected_paths: BTreeSet<&str> = expected.iter().map(|(pointer, _, _)| *pointer).collect();
    assert_eq!(paths, expected_paths, "{name}: {schema:#?}");
    for (pointer, node, needle) in expected {
        let diagnostic = schema
            .iter()
            .find(|diagnostic| diagnostic["data"]["instance_path"] == json!(pointer))
            .expect("present: the path sets match");
        assert_eq!(diagnostic["range"]["start"], position(text, at(text, node)), "{name} {pointer}: {diagnostic:#?}");
        assert!(diagnostic["message"].as_str().is_some_and(|message| message.contains(needle)), "{name} {pointer}: {diagnostic:#?}");
    }
}

/// One test per fixture `md schema validate` rejects, each opening its one
/// document in its own session, so the corpus's growth adds tests rather than
/// time to one test. `INVALID_FIXTURES` is every generated test's fixture.
macro_rules! invalid_fixture_tests {
    ($($test:ident => ($name:literal, $text:expr, [$(($pointer:literal, [$($node:literal),+], $needle:literal)),+ $(,)?]);)*) => {
        $(
            #[test]
            fn $test() {
                assert_rejected_where_the_cli_rejects($name, $text, &[$(($pointer, &[$($node),+], $needle)),+]);
            }
        )*

        const INVALID_FIXTURES: &[&str] = &[$($name),*];
    };
}

invalid_fixture_tests! {
    the_cli_rejected_depth6_wrong_verb_is_rejected_at_the_verb => (
        "depth6-wrong-verb",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth6-wrong-verb.md"),
        [("/success/0/then/0/then/0/then/0/then/0/then/0/then/0", ["sya"], "sya")]
    );
    the_cli_rejected_generated_verb_wrong_type_is_rejected_at_the_operand => (
        "generated-verb-wrong-type",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-wrong-type.md"),
        [("/start/0/pr_list", ["three"], "three")]
    );
    // A malformed template is never deferred: its syntax is the problem.
    the_cli_rejected_generated_verb_malformed_template_is_rejected_at_the_template => (
        "generated-verb-malformed-template",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-malformed-template.md"),
        [("/start/0/pr_list", ["pr_list: ", "\""], "malformed template `{{ 1 + }}`")]
    );
    // ... even where the operand's type accepts the text.
    the_cli_rejected_generated_verb_malformed_string_template_is_rejected_at_each_template => (
        "generated-verb-malformed-string-template",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-malformed-string-template.md"),
        [
            ("/start/0/ensure_file", ["ensure_file: ", "\""], "malformed template `{{ 1 + }}`"),
            ("/start/1/trim/text", ["text: ", "'"], "malformed template `hello {{ 1 + }}`"),
            ("/start/2/ensure_file/1", ["notes.txt, ", "'"], "malformed template `out/{{ 1 + }}.md`"),
        ]
    );
    // ... and in a verb the parser owns itself, wherever it reads a template.
    the_cli_rejected_handwritten_action_malformed_template_is_rejected_at_each_template => (
        "handwritten-action-malformed-template",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/handwritten-action-malformed-template.md"),
        [
            ("/start/0/say", ["say: ", "\""], "malformed template `{{ 1 + }}`"),
            ("/start/1/info/text", ["text: ", "'"], "malformed template `hello {{ 1 + }}`"),
            ("/start/2/set/round", ["round: ", "'"], "malformed template `{{ fm.round + }}`"),
            ("/start/3/shell", ["shell: ", "'"], "malformed template `echo {{ 2 * }}`"),
            ("/start/4/then/0/error/reason", ["reason: ", "'"], "malformed template `failed {{ 3 - }}`"),
            ("/start/4/else/0/proxy/with/note", ["note: ", "'"], "malformed template `next {{ 4 / }}`"),
        ]
    );
    the_cli_rejected_deferred_beside_unknown_parameter_is_rejected_at_the_call => (
        "generated-verb-deferred-beside-unknown-parameter",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred-beside-unknown-parameter.md"),
        [("/start/0/pr_list", ["pr_list: ", "{"], "limit")]
    );
    the_cli_rejected_deferred_beside_extra_argument_is_rejected_at_the_argument => (
        "generated-verb-deferred-beside-extra-argument",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred-beside-extra-argument.md"),
        [("/start/0/min/2", ["2, ", "3"], "3")]
    );
    the_cli_rejected_deferred_beside_bad_sibling_is_rejected_at_the_sibling => (
        "generated-verb-deferred-beside-bad-sibling",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred-beside-bad-sibling.md"),
        [("/start/0/min/0", ["three"], "three")]
    );
    the_cli_rejected_optional_nonnullable_null_is_rejected_at_each_null => (
        "optional-nonnullable-null",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/optional-nonnullable-null.md"),
        [
            ("/start/0/ping/1", ["null"], "null"),
            ("/start/1/ping/timeout", ["timeout: ", "null"], "null"),
            ("/start/1/ping", ["ping: ", "ping: ", "{"], "timeout"),
        ]
    );
    // The runtime registration refuses what the projected type converts.
    the_cli_rejected_generated_verb_excluded_source_is_rejected_at_each_boolean => (
        "generated-verb-excluded-source",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-excluded-source.md"),
        [
            ("/start/0/length/0", ["true"], "boolean"),
            ("/start/1/then/0/length/val", ["val: ", "false"], "boolean"),
        ]
    );
    // A loose long form beside another loose verb.
    the_cli_rejected_long_forms_in_loop_bundle_is_rejected_at_the_long_form => (
        "long-forms-in-loop-bundle",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/long-forms-in-loop-bundle.md"),
        [("/loop/ensure_file", ["ensure_file: ", "{"], "file")]
    );
    // A conditional is a list item only, never a stack's dictionary.
    the_cli_rejected_conditional_root_dictionary_is_rejected_at_the_stack => (
        "conditional-root-dictionary",
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/conditional-root-dictionary.md"),
        [("/start", ["when"], "anyOf")]
    );
}

/// Corpus fixtures this file judges in no test: the CLI and Claudine's own
/// suites judge them, and their verdict is not part of the editor corpus.
const OUTSIDE_EDITOR_PARITY: &[&str] = &[
    "agent-empty-list", "bare-stop", "bundle-in-then", "controls-beside-ticket", "empty-frontmatter",
    "linked-prompt", "long-form-in-bundle", "long-form-in-root-dictionary", "long-forms", "loop-gate",
    "loop-number", "message-shell-bundle-with-when", "misspelled-parameter", "misspelled-verb-in-then",
    "misspelled-verb", "null-identifiers", "prompt-with-malformed-sequence", "root-dictionary",
    "shell-expansion-in-initialize-set", "shell-in-initialize-depth3-then", "shell-in-initialize-list",
    "shell-in-initialize-root-dictionary", "shell-in-initialize-then", "shell-in-start", "start-number",
    "unrelated-ticket", "when-with-then-and-sibling",
];

/// No corpus fixture escapes the editor parity unnoticed: each is named by a
/// test in this file or listed in [`OUTSIDE_EDITOR_PARITY`], and each
/// generated invalid-fixture test names a distinct corpus fixture.
#[test]
fn every_corpus_fixture_is_judged_here_or_listed_outside_editor_parity() {
    let corpus_dir = biscuit_test_harness::manifest_dir!().join("../../claudine/lib/tests/fixtures/claudine-schemas");
    let corpus: BTreeSet<String> = std::fs::read_dir(&corpus_dir)
        .unwrap_or_else(|error| panic!("{}: {error}", corpus_dir.display()))
        .map(|entry| entry.expect("a directory entry").file_name().to_string_lossy().into_owned())
        .filter_map(|file| file.strip_suffix(".md").map(str::to_owned))
        .collect();
    let invalid: BTreeSet<&str> = INVALID_FIXTURES.iter().copied().collect();
    assert_eq!(invalid.len(), INVALID_FIXTURES.len(), "one test per invalid fixture: {INVALID_FIXTURES:?}");
    for name in INVALID_FIXTURES.iter().chain(OUTSIDE_EDITOR_PARITY) {
        assert!(corpus.contains(*name), "`{name}` is not a corpus fixture in {}", corpus_dir.display());
    }
    let source = include_str!("claudine_editor_parity.rs");
    for name in &corpus {
        let named = source.contains(&format!("claudine-schemas/{name}.md\")"));
        let outside = OUTSIDE_EDITOR_PARITY.contains(&name.as_str());
        assert!(
            named != outside,
            "`{name}.md`: add an editor-parity test for it (an `invalid_fixture_tests!` row if the CLI rejects it) or list it in OUTSIDE_EDITOR_PARITY, not both"
        );
    }
}

/// A stored literal token is data: DMLS types the text it holds as the
/// generated verb's argument, at the operand, as `md schema validate` does,
/// and never defers it as a template.
#[test]
fn a_stored_literal_token_argument_is_rejected_at_its_operand() {
    // The token holds `{{ fm.count }}`, which no number converts from.
    let text = "---\nstart:\n    - abs: \"{{!data:v1:e3sgZm0uY291bnQgfX0}}\"\n---\n# literal token\n";
    let found = published(&[("literal-token", text)]);
    let schema: Vec<&Value> = found[0].iter().filter(|diagnostic| is_schema(diagnostic)).collect();
    let paths: BTreeSet<&str> = schema.iter().filter_map(|diagnostic| diagnostic["data"]["instance_path"].as_str()).collect();
    assert_eq!(paths, BTreeSet::from(["/start/0/abs"]), "{schema:#?}");
    assert_eq!(schema[0]["range"]["start"], position(text, at(text, &["abs: ", "\""])), "{schema:#?}");
}

/// Asserts the completion `labels` at `start`'s innermost `branch`, emptied
/// to a bare item marker, offer the generated verbs, `shell`, and `when`.
fn assert_depth4_start_branch_completes_generated_verbs(branch: &str, labels: &[String]) {
    for verb in ["ensure_file", "pr_list", "set_frontmatter", "shell", "say", "when"] {
        assert!(has(labels, verb), "start {branch}: `{verb}` in {labels:?}");
    }
}

/// Each hover answers from the schema at depth four: bold `name`, naming its
/// item path.
fn assert_depth4_hovers(hovers: &[(&Vec<String>, &str)], item: &str) {
    for (hover, name) in hovers {
        let hover = &hover[0];
        assert!(hover.contains(&format!("**`{name}`**")), "`{name}` hover: {hover:?}");
        assert!(hover.contains(item), "`{name}` hover names its item {item}: {hover:?}");
    }
}

/// In `start`, four conditionals deep, the innermost `then` completes the
/// generated canonical verbs and `shell`, and the long-form parameters of
/// `ensure_file`; hover on the verb and on a parameter answers from the schema
/// at that depth, including a parameter only a later overload declares
/// (`content`). The `else` branch is a separate test so neither opens all
/// nine documents in one session.
#[test]
fn depth4_start_then_branch_completes_and_hovers_generated_verbs_and_parameters() {
    let depth4 = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth4-completion.md");
    let (then_item, then_cursor) = emptied(depth4, "start", "then");
    // A key-less block long form under the verb; flow-mapping keys (`{ fi }`)
    // are not a completion position in DMLS at any depth.
    let then_parameters = depth4.replacen(
        "- ensure_file: { file: notes.md, content: \"# Notes\" }",
        "- ensure_file:\n                                  file: notes.md\n                                  co: null",
        1,
    );
    let then_path = innermost("start", "then");

    let found = editor_answers_at(&[
        ("then-item.md", &then_item, then_cursor, Ask::Complete),
        ("then-parameters.md", &then_parameters, after(&then_parameters, &[then_path.as_slice(), &["\n", "co"]].concat()), Ask::Complete),
        ("then-verb.md", depth4, at(depth4, &[then_path.as_slice(), &["ensure_file"]].concat()), Ask::Hover),
        ("then-parameter.md", depth4, at(depth4, &[then_path.as_slice(), &["ensure_file", "file:"]].concat()), Ask::Hover),
        ("then-overload.md", depth4, at(depth4, &[then_path.as_slice(), &["content"]].concat()), Ask::Hover),
    ]);
    let [then_labels, then_keys, then_verb, then_parameter, then_overload] = &found[..] else {
        panic!("{found:?}")
    };

    assert_depth4_start_branch_completes_generated_verbs("then", then_labels);
    assert_eq!(then_keys, &["content"], "the `ensure_file` long-form parameter starting `co`");
    assert_depth4_hovers(
        &[(then_verb, "ensure_file"), (then_parameter, "file"), (then_overload, "content")],
        "`start[0].then[0].then[0].then[0].then[0]`",
    );
    assert!(then_parameter[0].contains("Required"), "`file` is the required parameter: {then_parameter:?}");
}

/// In `start`, four conditionals deep, the innermost `else` completes the
/// generated canonical verbs and `shell`, and the long-form parameters of
/// `pr_list`; hover on the verb and on a parameter only a later overload
/// declares (`count`) answers from the schema at that depth.
#[test]
fn depth4_start_else_branch_completes_and_hovers_generated_verbs_and_parameters() {
    let depth4 = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth4-completion.md");
    let (else_item, else_cursor) = emptied(depth4, "start", "else");
    let else_parameters =
        depth4.replacen("- pr_list: { count: 3 }", "- pr_list:\n                                  c: null", 1);
    let else_path = innermost("start", "else");

    let found = editor_answers_at(&[
        ("else-item.md", &else_item, else_cursor, Ask::Complete),
        ("else-parameters.md", &else_parameters, after(&else_parameters, &[else_path.as_slice(), &["\n", "c"]].concat()), Ask::Complete),
        ("else-verb.md", depth4, at(depth4, &[else_path.as_slice(), &["pr_list"]].concat()), Ask::Hover),
        ("else-parameter.md", depth4, at(depth4, &[else_path.as_slice(), &["count"]].concat()), Ask::Hover),
    ]);
    let [else_labels, else_keys, else_verb, else_parameter] = &found[..] else {
        panic!("{found:?}")
    };

    assert_depth4_start_branch_completes_generated_verbs("else", else_labels);
    assert_eq!(else_keys, &["count"], "the `pr_list` long-form parameter starting `c`");
    assert_depth4_hovers(&[(else_verb, "pr_list"), (else_parameter, "count")], "`start[0].then[0].then[0].then[0].else[0]`");
}

/// Four conditionals deep, each event keeps its own action profile in both
/// branches: `initialize` offers the generated verbs but not `shell`, the loop
/// gate the ordinary profile with `shell`, and `cancelled` its own `stop` but
/// none of the other flow controls.
#[test]
fn depth4_branches_keep_each_events_profile() {
    let depth4 = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth4-completion.md");
    let positions: Vec<(String, (String, usize))> = ["initialize", "gate", "cancelled"]
        .into_iter()
        .flat_map(|event| ["then", "else"].map(|branch| (format!("{event} {branch}"), emptied(depth4, event, branch))))
        .collect();
    let names: Vec<String> = (0..positions.len()).map(|index| format!("profile-{index}.md")).collect();
    let cases: Vec<(&str, &str, usize, Ask)> = positions
        .iter()
        .zip(&names)
        .map(|((_, (text, cursor)), name)| (name.as_str(), text.as_str(), *cursor, Ask::Complete))
        .collect();

    let found = editor_answers_at(&cases);
    let labels: BTreeMap<&str, &Vec<String>> =
        positions.iter().map(|(label, _)| label.as_str()).zip(found.iter()).collect();
    for branch in ["then", "else"] {
        let initialize = labels[format!("initialize {branch}").as_str()];
        for verb in ["ensure_file", "pr_list", "set_frontmatter", "say", "when"] {
            assert!(has(initialize, verb), "initialize {branch}: `{verb}` in {initialize:?}");
        }
        assert!(!has(initialize, "shell"), "initialize {branch} is shell-free: {initialize:?}");

        let gate = labels[format!("gate {branch}").as_str()];
        for verb in ["ensure_file", "pr_list", "set_frontmatter", "shell", "say", "retry", "when"] {
            assert!(has(gate, verb), "gate {branch}: `{verb}` in {gate:?}");
        }

        let cancelled = labels[format!("cancelled {branch}").as_str()];
        for verb in ["stop", "shell", "say", "pr_list", "when"] {
            assert!(has(cancelled, verb), "cancelled {branch}: `{verb}` in {cancelled:?}");
        }
        for verb in ["retry", "resume", "proxy", "prep", "skip", "error", "break", "defer"] {
            assert!(!has(cancelled, verb), "cancelled {branch} does not offer `{verb}`: {cancelled:?}");
        }
    }
}

/// Schema validation, completion, and hover over lifecycle stacks that would
/// create files, run a shell, and call expression functions execute none of
/// it. They run in process on this thread, so the recording test function and
/// the per-thread handler counters see every expression-function entry; the
/// process-wide counters see any effect engine or network attempt.
#[test]
fn claudine_schema_validation_and_editor_requests_run_no_actions() {
    let depth4 = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth4-completion.md");
    let generated = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred.md");
    let nullable = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/optional-nonnullable-null.md");
    let variadic = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/variadic-and-or.md");

    #[cfg(feature = "effects-instrumentation")]
    let before = (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count());
    #[cfg(feature = "effects-instrumentation")]
    let _recorded = {
        use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
        use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
        let registration = FunctionRegistration::builder("recorded", "function(parameters(void); returns(boolean))")
            .build()
            .expect("the test signature compiles");
        let guard = hooks::register(registration, Arc::new(|_| Ok(json!(true))));
        hooks::reset_handler_entries();
        guard
    };

    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let spelled = |name: &str| workspace.path().join(name).to_string_lossy().replace('\\', "/");
    let (ensured, touched, gated) = (spelled("ENSURED_AT_DEPTH_4"), spelled("TOUCHED_AT_DEPTH_4"), spelled("GATE_ENSURED"));
    let acting = depth4
        .replacen("ctx.level == 1", "recorded()", 4)
        .replacen(
            "- ensure_file: { file: notes.md, content: \"# Notes\" }",
            &format!("- ensure_file: {{ file: \"{ensured}\", content: \"{{{{ recorded() }}}}\" }}"),
            1,
        )
        .replacen("- pr_list: { count: 3 }", &format!("- shell: \"touch {touched}\""), 1)
        .replacen("- ensure_file: notes.md", &format!("- ensure_file: \"{gated}\""), 1);
    assert!(acting.contains("TOUCHED_AT_DEPTH_4") && acting.contains("GATE_ENSURED"), "{acting}");
    let documents = [("acting.md", acting.as_str()), ("generated.md", generated), ("nullable.md", nullable), ("variadic.md", variadic)];

    let config = DmlsConfig::default();
    let contexts = RepositoryContexts::new(fixture.snapshot());
    let registry = ProviderRegistry::with_substrate();
    let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &NoContexts);
    let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
    let repo = [fixture.repo()];
    let mut answered = 0;
    for (name, text) in documents {
        let path = fixture.document(name, text);
        let uri: Uri = uri_of(&path).parse().unwrap();
        let resolution = contexts.for_document(&path);
        let overlay = OverlayState::default().for_document(&uri, text, &path, &config, &repo, &resolution);
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let ctx = DocumentContext {
            uri: &uri,
            path: &path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &config,
            profile: &profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
        };
        let diagnostics = registry.diagnostics(&ctx);
        if name == "nullable.md" {
            // Validation ran: the trigger typed the document and rejected it.
            assert!(
                diagnostics.iter().any(|diagnostic| matches!(&diagnostic.code, Some(lsp_types::NumberOrString::String(code)) if code.starts_with("dm.schema."))),
                "{diagnostics:#?}"
            );
        }
        // The first of each verb, parameter, template call, and item marker.
        for needle in ["ensure_file", "{ file:", "content", "recorded", "shell", "touch", "pr_list", "min", "ping", "and", "- "] {
            if let Some(offset) = text.find(needle) {
                let hover = registry.hover(&ctx, offset);
                if name == "acting.md" && ["ensure_file", "shell", "content"].contains(&needle) {
                    assert!(hover.is_some(), "the depth-4 `{needle}` is analyzed");
                }
                registry.completion(&ctx, offset + needle.len());
                answered += 1;
            }
        }
    }
    assert!(answered >= 15, "the editor requests reached the stacks: {answered}");

    for sentinel in [&ensured, &touched, &gated] {
        assert!(!Path::new(sentinel).exists(), "{sentinel} was created: a lifecycle action ran");
    }
    #[cfg(feature = "effects-instrumentation")]
    {
        use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
        for function in ["recorded", "pr_list", "min", "ping", "and", "or"] {
            assert_eq!(hooks::handler_entries(function), 0, "`{function}` was evaluated");
        }
        // The counter is live on this thread: one real evaluation registers.
        struct NoValues;
        impl darkmatter::markdown::compose::expression::EvaluationLookup for NoValues {
            fn get(&self, _: &str) -> Option<Value> {
                None
            }
        }
        use darkmatter::markdown::compose::expression::{evaluate, parse};
        assert_eq!(evaluate(&parse("recorded()").expect("parses"), &NoValues).expect("evaluates"), json!(true));
        assert_eq!(hooks::handler_entries("recorded"), 1, "the recording seam observes an evaluation");
        assert_eq!(
            (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count()),
            before,
            "validation and editor requests built no effect engine and attempted no network access",
        );
    }
}
