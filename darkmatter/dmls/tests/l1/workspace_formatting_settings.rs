//! Robustness matrices for the workspace `formatting` settings: the
//! `.dmls.toml` file and the editor's `didChangeConfiguration` settings.
//!
//! Each matrix starts from a checked-in control fixture that sets all ten
//! rules to non-default values, then applies exactly one mutation per cell
//! through `ConfigState` and asserts the effective workspace overrides, the
//! visible diagnostics (severity and key), and that unrelated settings
//! survive. Cell codes follow the plan's matrix: F falls through, N is an
//! explicit `null` that releases to the lower level, W rejects the source's
//! formatting overrides with an error, L is a migrated legacy value with one
//! warning, and an invalid file contributes nothing.

use std::path::Path;

use darkmatter::markdown::cleanup::rules::{
    DefaultLanguage, FixedWidth, Indentation, Leaf, ListInteriorPadding, ListItemPadding,
    MarkdownRuleOverrides, RuleSource, resolve,
};
use dmls::config::{CONFIG_FILE_NAME, ConfigSeverity, ConfigSource, ConfigState};
use serde_json::{Value, json};

use ConfigSeverity::{Error, Warning};

/// Debounce the TOML fixture sets: proves the file's other settings loaded.
const FILE_DEBOUNCE: u64 = 150;
/// Debounce the editor fixture sets: proves the overlay's other settings applied.
const EDITOR_DEBOUNCE: u64 = 300;
/// The built-in debounce: nothing from the file applied.
const DEFAULT_DEBOUNCE: u64 = 200;

const RULES: [&str; 10] = [
    "frontmatter",
    "table-layout",
    "fixed-width",
    "unwrap",
    "indentation",
    "ordered-lists",
    "list-padding",
    "list-item-padding",
    "list-interior-padding",
    "default-language",
];

fn language(text: &str) -> DefaultLanguage {
    DefaultLanguage::new(text).expect("test language is valid")
}

/// The overrides both control fixtures author.
fn control() -> MarkdownRuleOverrides {
    MarkdownRuleOverrides {
        frontmatter: Leaf::Value(false),
        table_layout: Leaf::Value(false),
        fixed_width: Leaf::Value(FixedWidth::Columns(80)),
        unwrap: Leaf::Value(true),
        indentation: Leaf::Value(Indentation::Two),
        ordered_lists: Leaf::Value(false),
        list_padding: Leaf::Value(false),
        list_item_padding: Leaf::Value(ListItemPadding::One),
        list_interior_padding: Leaf::Value(ListInteriorPadding::All),
        default_language: Leaf::Value(language("sh")),
    }
}

fn none() -> MarkdownRuleOverrides {
    MarkdownRuleOverrides::default()
}

/// `target` with `rule`'s leaf taken from `from`.
fn copy_leaf(
    mut target: MarkdownRuleOverrides,
    from: &MarkdownRuleOverrides,
    rule: &str,
) -> MarkdownRuleOverrides {
    match rule {
        "frontmatter" => target.frontmatter = from.frontmatter.clone(),
        "table-layout" => target.table_layout = from.table_layout.clone(),
        "fixed-width" => target.fixed_width = from.fixed_width.clone(),
        "unwrap" => target.unwrap = from.unwrap.clone(),
        "indentation" => target.indentation = from.indentation.clone(),
        "ordered-lists" => target.ordered_lists = from.ordered_lists.clone(),
        "list-padding" => target.list_padding = from.list_padding.clone(),
        "list-item-padding" => target.list_item_padding = from.list_item_padding.clone(),
        "list-interior-padding" => {
            target.list_interior_padding = from.list_interior_padding.clone();
        }
        "default-language" => target.default_language = from.default_language.clone(),
        other => panic!("unknown rule {other}"),
    }
    target
}

/// Whether `rule` holds a value in `overrides`.
fn has_value(overrides: &MarkdownRuleOverrides, rule: &str) -> bool {
    copy_leaf(none(), overrides, rule) != none()
}

/// `target` with `rule`'s leaf set to an explicit `null`.
fn null_leaf(mut target: MarkdownRuleOverrides, rule: &str) -> MarkdownRuleOverrides {
    match rule {
        "frontmatter" => target.frontmatter = Leaf::Null,
        "table-layout" => target.table_layout = Leaf::Null,
        "fixed-width" => target.fixed_width = Leaf::Null,
        "unwrap" => target.unwrap = Leaf::Null,
        "indentation" => target.indentation = Leaf::Null,
        "ordered-lists" => target.ordered_lists = Leaf::Null,
        "list-padding" => target.list_padding = Leaf::Null,
        "list-item-padding" => target.list_item_padding = Leaf::Null,
        "list-interior-padding" => target.list_interior_padding = Leaf::Null,
        "default-language" => target.default_language = Leaf::Null,
        other => panic!("unknown rule {other}"),
    }
    target
}

/// What one cell must produce.
struct Expected {
    rules: MarkdownRuleOverrides,
    diagnostics: Vec<(ConfigSeverity, Option<String>)>,
    debounce_ms: u64,
}

impl Expected {
    fn accepted(rules: MarkdownRuleOverrides, debounce_ms: u64) -> Self {
        Self {
            rules,
            diagnostics: Vec::new(),
            debounce_ms,
        }
    }

    fn migrated(rules: MarkdownRuleOverrides, keys: &[&str], debounce_ms: u64) -> Self {
        Self {
            rules,
            diagnostics: keys
                .iter()
                .map(|key| (Warning, Some((*key).to_string())))
                .collect(),
            debounce_ms,
        }
    }

    fn rejected(rules: MarkdownRuleOverrides, key: &str, debounce_ms: u64) -> Self {
        Self {
            rules,
            diagnostics: vec![(Error, Some(key.to_string()))],
            debounce_ms,
        }
    }

    fn invalid_file() -> Self {
        Self {
            rules: none(),
            diagnostics: vec![(Error, None)],
            debounce_ms: DEFAULT_DEBOUNCE,
        }
    }
}

/// Compares `state` with `expected`, recording a readable failure for `label`.
fn check(
    label: &str,
    state: &ConfigState,
    expected: &Expected,
    source: &ConfigSource,
    failures: &mut Vec<String>,
) {
    let effective = state.effective();
    let diagnostics: Vec<_> = state
        .diagnostics()
        .iter()
        .map(|diagnostic| (diagnostic.severity, diagnostic.key.clone()))
        .collect();
    let mut problems = Vec::new();
    if effective.formatting.markdown != expected.rules {
        problems.push(format!(
            "rules: expected {:?}, found {:?}",
            expected.rules, effective.formatting.markdown
        ));
    }
    if diagnostics != expected.diagnostics {
        problems.push(format!(
            "diagnostics: expected {:?}, found {:?}",
            expected.diagnostics,
            state.diagnostics()
        ));
    }
    for diagnostic in state.diagnostics() {
        if &diagnostic.source != source
            || !diagnostic.to_string().starts_with(&source.to_string())
        {
            problems.push(format!("diagnostic does not name {source}: {diagnostic}"));
        }
    }
    if effective.diagnostics.debounce_ms != expected.debounce_ms {
        problems.push(format!(
            "debounce_ms: expected {}, found {}",
            expected.debounce_ms, effective.diagnostics.debounce_ms
        ));
    }
    if !problems.is_empty() {
        failures.push(format!("[{label}]\n  {}", problems.join("\n  ")));
    }
}

fn assert_no_failures(failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} cell(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// TOML: `.dmls.toml`
// ---------------------------------------------------------------------------

/// The fixture with `rule`'s line replaced by `replacement` (or removed).
fn with_rule_line(fixture: &str, rule: &str, replacement: Option<&str>) -> String {
    let prefix = format!("{rule} = ");
    let mut found = false;
    let mut lines = Vec::new();
    for line in fixture.lines() {
        if line.starts_with(&prefix) {
            found = true;
            if let Some(replacement) = replacement {
                lines.push(replacement.to_string());
            }
        } else {
            lines.push(line.to_string());
        }
    }
    assert!(found, "fixture has no `{rule}` line");
    lines.join("\n") + "\n"
}

/// The fixture up to (not including) its `[formatting.markdown]` table.
fn without_formatting(fixture: &str) -> &str {
    let start = fixture
        .find("[formatting.markdown]")
        .expect("fixture has a [formatting.markdown] table");
    &fixture[..start]
}

struct TomlLeaf {
    rule: &'static str,
    control: &'static str,
    wrong_type: &'static str,
    one_wrong: &'static str,
    every_wrong: &'static str,
}

const TOML_LEAVES: [TomlLeaf; 10] = [
    TomlLeaf { rule: "frontmatter", control: "false", wrong_type: "\"false\"", one_wrong: "[false, 1]", every_wrong: "[1]" },
    TomlLeaf { rule: "table-layout", control: "false", wrong_type: "\"false\"", one_wrong: "[false, 1]", every_wrong: "[1]" },
    TomlLeaf { rule: "fixed-width", control: "80", wrong_type: "\"80\"", one_wrong: "[80, \"x\"]", every_wrong: "[\"x\"]" },
    TomlLeaf { rule: "unwrap", control: "true", wrong_type: "\"true\"", one_wrong: "[true, 1]", every_wrong: "[1]" },
    TomlLeaf { rule: "indentation", control: "2", wrong_type: "true", one_wrong: "[2, \"x\"]", every_wrong: "[\"x\"]" },
    TomlLeaf { rule: "ordered-lists", control: "false", wrong_type: "\"false\"", one_wrong: "[false, 1]", every_wrong: "[1]" },
    TomlLeaf { rule: "list-padding", control: "false", wrong_type: "\"false\"", one_wrong: "[false, 1]", every_wrong: "[1]" },
    TomlLeaf { rule: "list-item-padding", control: "\"one\"", wrong_type: "1", one_wrong: "[\"one\", 1]", every_wrong: "[1]" },
    TomlLeaf { rule: "list-interior-padding", control: "\"all\"", wrong_type: "true", one_wrong: "[\"all\", 1]", every_wrong: "[1]" },
    TomlLeaf { rule: "default-language", control: "\"sh\"", wrong_type: "5", one_wrong: "[\"sh\", 1]", every_wrong: "[1]" },
];

fn toml_cells(fixture: &str) -> Vec<(String, String, Expected)> {
    let mut cells = Vec::new();
    let mut cell = |label: String, text: String, expected: Expected| {
        cells.push((label, text, expected));
    };
    let rejected = |key: &str| Expected::rejected(none(), key, FILE_DEBOUNCE);

    // Leaf cells: one mutation of one rule line.
    for leaf in &TOML_LEAVES {
        let rule = leaf.rule;
        let key = format!("formatting.markdown.{rule}");
        let set = |value: &str| with_rule_line(fixture, rule, Some(&format!("{rule} = {value}")));
        cell(
            format!("{rule}: absent"),
            with_rule_line(fixture, rule, None),
            Expected::accepted(copy_leaf(control(), &none(), rule), FILE_DEBOUNCE),
        );
        cell(format!("{rule}: wrong whole type"), set(leaf.wrong_type), rejected(&key));
        cell(format!("{rule}: one wrong element"), set(leaf.one_wrong), rejected(&key));
        cell(format!("{rule}: every element wrong"), set(leaf.every_wrong), rejected(&key));
        let empty_string = if rule == "default-language" {
            let mut rules = control();
            rules.default_language = Leaf::Value(language(""));
            Expected::accepted(rules, FILE_DEBOUNCE)
        } else {
            rejected(&key)
        };
        cell(format!("{rule}: empty \"\""), set("\"\""), empty_string);
        cell(format!("{rule}: empty []"), set("[]"), rejected(&key));
        cell(format!("{rule}: empty {{}}"), set("{}"), rejected(&key));
        cell(
            format!("{rule}: duplicate key"),
            with_rule_line(
                fixture,
                rule,
                Some(&format!("{rule} = {}\n{rule} = {}", leaf.control, leaf.control)),
            ),
            Expected::invalid_file(),
        );
        cell(
            format!("{rule}: trailing garbage"),
            set(&format!("{} trailing", leaf.control)),
            Expected::invalid_file(),
        );
    }

    // Semantic-invalid leaves.
    let semantic = [
        ("indentation", "\"4\""),
        ("fixed-width", "80.5"),
        ("fixed-width", "0"),
        ("fixed-width", "1001"),
        ("default-language", "\"a`b\""),
        ("default-language", "\"a\\nb\""),
        ("default-language", "1979-05-27"),
        ("list-item-padding", "\"two\""),
    ];
    for (rule, value) in semantic {
        cell(
            format!("{rule}: semantic-invalid {value}"),
            with_rule_line(fixture, rule, Some(&format!("{rule} = {value}"))),
            rejected(&format!("formatting.markdown.{rule}")),
        );
    }
    let mut true_width = control();
    true_width.fixed_width = Leaf::Value(FixedWidth::Columns(120));
    cell(
        "fixed-width: true means 120".to_string(),
        with_rule_line(fixture, "fixed-width", Some("fixed-width = true")),
        Expected::accepted(true_width, FILE_DEBOUNCE),
    );
    cell(
        "markdown: unknown rule".to_string(),
        format!("{fixture}colour = true\n"),
        rejected("formatting.markdown.colour"),
    );
    cell(
        "unrelated section keeps its lenient policy".to_string(),
        format!("[wiki]\nmystery = true\n\n{fixture}"),
        Expected::accepted(control(), FILE_DEBOUNCE),
    );

    // Object roots: `formatting` and `formatting.markdown`.
    let base = without_formatting(fixture);
    let root = |snippet: &str| format!("{base}{snippet}\n");
    // A bare `formatting = …` key must precede every table header.
    let top = |snippet: &str| format!("{snippet}\n{base}");
    cell("formatting: absent".into(), root(""), Expected::accepted(none(), FILE_DEBOUNCE));
    for (label, value) in [
        ("wrong whole type", "5"),
        ("one wrong element", "[{ markdown = {} }, 1]"),
        ("every element wrong", "[1]"),
        ("empty \"\"", "\"\""),
        ("empty []", "[]"),
    ] {
        cell(format!("formatting: {label}"), top(&format!("formatting = {value}")), rejected("formatting"));
    }
    cell("formatting: empty {}".into(), top("formatting = {}"), Expected::accepted(none(), FILE_DEBOUNCE));
    cell("formatting: duplicate key".into(), root("[formatting]\n[formatting]"), Expected::invalid_file());
    cell("formatting: trailing garbage".into(), top("formatting = {} x"), Expected::invalid_file());

    cell("markdown: absent".into(), root("[formatting]"), Expected::accepted(none(), FILE_DEBOUNCE));
    for (label, value) in [
        ("wrong whole type", "5"),
        ("one wrong element", "[{}, 1]"),
        ("every element wrong", "[1]"),
        ("empty \"\"", "\"\""),
        ("empty []", "[]"),
    ] {
        cell(
            format!("markdown: {label}"),
            root(&format!("[formatting]\nmarkdown = {value}")),
            rejected("formatting.markdown"),
        );
    }
    cell(
        "markdown: empty {}".into(),
        root("[formatting]\nmarkdown = {}"),
        Expected::accepted(none(), FILE_DEBOUNCE),
    );
    cell(
        "markdown: duplicate key".into(),
        root("[formatting]\nmarkdown = {}\nmarkdown = {}"),
        Expected::invalid_file(),
    );
    cell(
        "markdown: trailing garbage".into(),
        root("[formatting]\nmarkdown = {} x"),
        Expected::invalid_file(),
    );
    cell(
        "formatting: unknown key".into(),
        root("[formatting]\nwidth = 80"),
        rejected("formatting.width"),
    );

    // Legacy keys.
    let padding = |value| MarkdownRuleOverrides {
        list_item_padding: Leaf::Value(value),
        ..none()
    };
    let width = |columns| MarkdownRuleOverrides {
        fixed_width: Leaf::Value(FixedWidth::Columns(columns)),
        ..none()
    };
    let legacy_cleanup = ["formatting.cleanup"];
    let legacy_width = ["formatting.fixed_width"];
    for (value, rules) in [
        ("\"compact\"", padding(ListItemPadding::None)),
        ("\"loose\"", padding(ListItemPadding::One)),
        ("\"default\"", none()),
    ] {
        cell(
            format!("cleanup: {value}"),
            root(&format!("[formatting]\ncleanup = {value}")),
            Expected::migrated(rules, &legacy_cleanup, FILE_DEBOUNCE),
        );
    }
    for (label, value) in [
        ("wrong whole type", "1"),
        ("one wrong element", "[\"compact\", 1]"),
        ("every element wrong", "[1]"),
        ("empty \"\"", "\"\""),
        ("empty {}", "{}"),
        ("unsupported", "\"tight\""),
    ] {
        cell(
            format!("cleanup: {label}"),
            root(&format!("[formatting]\ncleanup = {value}")),
            rejected("formatting.cleanup"),
        );
    }
    cell(
        "cleanup: duplicate key".into(),
        root("[formatting]\ncleanup = \"compact\"\ncleanup = \"loose\""),
        Expected::invalid_file(),
    );
    cell(
        "cleanup: trailing garbage".into(),
        root("[formatting]\ncleanup = \"compact\" x"),
        Expected::invalid_file(),
    );
    for (value, columns) in [("80", 80), ("1", 1), ("1000", 1000)] {
        cell(
            format!("fixed_width: {value}"),
            root(&format!("[formatting]\nfixed_width = {value}")),
            Expected::migrated(width(columns), &legacy_width, FILE_DEBOUNCE),
        );
    }
    for (label, value) in [
        ("wrong whole type", "\"80\""),
        ("one wrong element", "[80, \"x\"]"),
        ("every element wrong", "[\"x\"]"),
        ("empty \"\"", "\"\""),
        ("empty {}", "{}"),
        ("zero", "0"),
        ("too wide", "1001"),
        ("fractional", "80.5"),
        ("negative", "-80"),
    ] {
        cell(
            format!("fixed_width: {label}"),
            root(&format!("[formatting]\nfixed_width = {value}")),
            rejected("formatting.fixed_width"),
        );
    }
    cell(
        "fixed_width: duplicate key".into(),
        root("[formatting]\nfixed_width = 80\nfixed_width = 90"),
        Expected::invalid_file(),
    );
    cell(
        "fixed_width: trailing garbage".into(),
        root("[formatting]\nfixed_width = 80 x"),
        Expected::invalid_file(),
    );
    cell(
        "legacy: both keys, one warning each".into(),
        root("[formatting]\ncleanup = \"compact\"\nfixed_width = 80"),
        Expected::migrated(
            MarkdownRuleOverrides {
                list_item_padding: Leaf::Value(ListItemPadding::None),
                fixed_width: Leaf::Value(FixedWidth::Columns(80)),
                ..none()
            },
            &["formatting.cleanup", "formatting.fixed_width"],
            FILE_DEBOUNCE,
        ),
    );
    cell(
        "mixed: cleanup with empty markdown".into(),
        root("[formatting]\ncleanup = \"compact\"\nmarkdown = {}"),
        rejected("formatting"),
    );
    cell(
        "mixed: fixed_width with the control markdown".into(),
        fixture.replace(
            "[formatting.markdown]",
            "[formatting]\nfixed_width = 80\n\n[formatting.markdown]",
        ),
        rejected("formatting"),
    );

    cells
}

/// The `.dmls.toml` matrix: control, every leaf cell, object roots, legacy
/// keys, and semantic-invalid values, each loaded from a real file.
#[test]
fn toml_formatting_matrix() {
    let fixture = std::fs::read_to_string(
        biscuit_test_harness::manifest_dir!().join("tests/fixtures/formatting/dmls.toml"),
    )
    .expect("read the dmls.toml fixture");
    let mut failures = Vec::new();

    let load = |text: &str| {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join(CONFIG_FILE_NAME);
        std::fs::write(&path, text).expect("write config");
        let state = ConfigState::load(vec![temp.path().to_path_buf()], None);
        (state, ConfigSource::File(path), temp)
    };

    // Control: the unedited fixture yields every value, at Workspace level.
    let (state, source, _temp) = load(&fixture);
    check(
        "control",
        &state,
        &Expected::accepted(control(), FILE_DEBOUNCE),
        &source,
        &mut failures,
    );
    let resolved = resolve(&none(), None, Some(&state.effective().formatting.markdown));
    let sources = [
        resolved.frontmatter.source,
        resolved.table_layout.source,
        resolved.fixed_width.source,
        resolved.unwrap.source,
        resolved.indentation.source,
        resolved.ordered_lists.source,
        resolved.list_padding.source,
        resolved.list_item_padding.source,
        resolved.list_interior_padding.source,
        resolved.default_language.source,
    ];
    assert_eq!(sources, [RuleSource::Workspace; 10]);
    for rule in RULES {
        assert!(has_value(&control(), rule), "control sets {rule}");
    }

    for (label, text, expected) in toml_cells(&fixture) {
        let (state, source, _temp) = load(&text);
        check(&label, &state, &expected, &source, &mut failures);
    }
    assert_no_failures(&failures);
}

// ---------------------------------------------------------------------------
// JSON: editor `didChangeConfiguration` settings
// ---------------------------------------------------------------------------

/// The `.dmls.toml` under the editor matrix: the lower level a `null` or
/// absent editor leaf releases to.
const JSON_BASE_FILE: &str = "[formatting.markdown]\nfixed-width = 100\nindentation = 8\ndefault-language = \"rust\"\n";

fn json_base_file_rules() -> MarkdownRuleOverrides {
    MarkdownRuleOverrides {
        fixed_width: Leaf::Value(FixedWidth::Columns(100)),
        indentation: Leaf::Value(Indentation::Eight),
        default_language: Leaf::Value(language("rust")),
        ..none()
    }
}

/// `settings` with the member at `path` (under `dmls.formatting`) replaced,
/// or removed when `value` is `None`.
fn edit(settings: &Value, path: &[&str], value: Option<Value>) -> Value {
    let mut settings = settings.clone();
    let mut node = &mut settings["dmls"]["formatting"];
    let (last, parents) = path.split_last().expect("non-empty path");
    for segment in parents {
        node = &mut node[*segment];
    }
    let map = node.as_object_mut().expect("parent is an object");
    match value {
        Some(value) => {
            map.insert((*last).to_string(), value);
        }
        None => {
            map.remove(*last);
        }
    }
    settings
}

/// `settings` with `dmls.formatting` replaced (or removed).
fn with_formatting(settings: &Value, formatting: Option<Value>) -> Value {
    let mut settings = settings.clone();
    let dmls = settings["dmls"].as_object_mut().expect("dmls object");
    match formatting {
        Some(formatting) => {
            dmls.insert("formatting".to_string(), formatting);
        }
        None => {
            dmls.remove("formatting");
        }
    }
    settings
}

struct JsonLeaf {
    rule: &'static str,
    wrong_type: Value,
    one_wrong: Value,
    every_wrong: Value,
}

fn json_leaves() -> Vec<JsonLeaf> {
    let boolean = |rule, control: bool| JsonLeaf {
        rule,
        wrong_type: json!(control.to_string()),
        one_wrong: json!([control, 1]),
        every_wrong: json!([1]),
    };
    vec![
        boolean("frontmatter", false),
        boolean("table-layout", false),
        JsonLeaf { rule: "fixed-width", wrong_type: json!("80"), one_wrong: json!([80, "x"]), every_wrong: json!(["x"]) },
        boolean("unwrap", true),
        JsonLeaf { rule: "indentation", wrong_type: json!(true), one_wrong: json!([2, "x"]), every_wrong: json!(["x"]) },
        boolean("ordered-lists", false),
        boolean("list-padding", false),
        JsonLeaf { rule: "list-item-padding", wrong_type: json!(1), one_wrong: json!(["one", 1]), every_wrong: json!([1]) },
        JsonLeaf { rule: "list-interior-padding", wrong_type: json!(true), one_wrong: json!(["all", 1]), every_wrong: json!([1]) },
        JsonLeaf { rule: "default-language", wrong_type: json!(5), one_wrong: json!(["sh", 1]), every_wrong: json!([1]) },
    ]
}

fn json_cells(settings: &Value) -> Vec<(String, Value, Expected)> {
    let file = json_base_file_rules();
    let mut cells = Vec::new();
    let mut cell = |label: String, settings: Value, expected: Expected| {
        cells.push((label, settings, expected));
    };
    // W: the editor's formatting is dropped, the file's survives, and the
    // editor's other settings still apply.
    let rejected = |key: &str| Expected::rejected(json_base_file_rules(), key, EDITOR_DEBOUNCE);
    let file_only = || Expected::accepted(json_base_file_rules(), EDITOR_DEBOUNCE);

    for leaf in json_leaves() {
        let rule = leaf.rule;
        let key = format!("formatting.markdown.{rule}");
        let set = |value: Value| edit(settings, &["markdown", rule], Some(value));
        cell(
            format!("{rule}: absent"),
            edit(settings, &["markdown", rule], None),
            Expected::accepted(copy_leaf(control(), &file, rule), EDITOR_DEBOUNCE),
        );
        // N releases to the file's value where it has one; otherwise the
        // explicit null stays recorded and never reads as false/preserve/"".
        let released = if has_value(&file, rule) {
            copy_leaf(control(), &file, rule)
        } else {
            null_leaf(control(), rule)
        };
        cell(
            format!("{rule}: explicit null"),
            set(Value::Null),
            Expected::accepted(released, EDITOR_DEBOUNCE),
        );
        cell(format!("{rule}: wrong whole type"), set(leaf.wrong_type.clone()), rejected(&key));
        cell(format!("{rule}: one wrong element"), set(leaf.one_wrong.clone()), rejected(&key));
        cell(format!("{rule}: every element wrong"), set(leaf.every_wrong.clone()), rejected(&key));
        let empty_string = if rule == "default-language" {
            let mut rules = control();
            rules.default_language = Leaf::Value(language(""));
            Expected::accepted(rules, EDITOR_DEBOUNCE)
        } else {
            rejected(&key)
        };
        cell(format!("{rule}: empty \"\""), set(json!("")), empty_string);
        cell(format!("{rule}: empty []"), set(json!([])), rejected(&key));
        cell(format!("{rule}: empty {{}}"), set(json!({})), rejected(&key));
    }

    for (rule, value) in [
        ("indentation", json!("4")),
        ("fixed-width", json!(80.5)),
        ("fixed-width", json!(0)),
        ("fixed-width", json!(1001)),
        ("default-language", json!("a`b")),
        ("default-language", json!("a\nb")),
        ("list-item-padding", json!("two")),
    ] {
        cell(
            format!("{rule}: semantic-invalid {value}"),
            edit(settings, &["markdown", rule], Some(value)),
            rejected(&format!("formatting.markdown.{rule}")),
        );
    }
    cell(
        "markdown: unknown rule".into(),
        edit(settings, &["markdown", "colour"], Some(json!(true))),
        rejected("formatting.markdown.colour"),
    );

    // Object roots.
    cell("formatting: absent".into(), with_formatting(settings, None), file_only());
    cell("formatting: explicit null".into(), with_formatting(settings, Some(Value::Null)), file_only());
    cell("formatting: empty {}".into(), with_formatting(settings, Some(json!({}))), file_only());
    for (label, value) in [
        ("wrong whole type", json!(5)),
        ("one wrong element", json!([{ "markdown": {} }, 1])),
        ("every element wrong", json!([1])),
        ("empty \"\"", json!("")),
        ("empty []", json!([])),
    ] {
        cell(format!("formatting: {label}"), with_formatting(settings, Some(value)), rejected("formatting"));
    }
    cell("markdown: absent".into(), edit(settings, &["markdown"], None), file_only());
    cell("markdown: explicit null".into(), edit(settings, &["markdown"], Some(Value::Null)), file_only());
    cell("markdown: empty {}".into(), edit(settings, &["markdown"], Some(json!({}))), file_only());
    for (label, value) in [
        ("wrong whole type", json!(5)),
        ("one wrong element", json!([{}, 1])),
        ("every element wrong", json!([1])),
        ("empty \"\"", json!("")),
        ("empty []", json!([])),
    ] {
        cell(format!("markdown: {label}"), edit(settings, &["markdown"], Some(value)), rejected("formatting.markdown"));
    }
    cell(
        "formatting: unknown key".into(),
        with_formatting(settings, Some(json!({ "width": 80 }))),
        rejected("formatting.width"),
    );

    // Legacy keys, merged over the file's rules.
    let over_file = |edit: fn(&mut MarkdownRuleOverrides)| {
        let mut rules = json_base_file_rules();
        edit(&mut rules);
        rules
    };
    let legacy = |formatting: Value| with_formatting(settings, Some(formatting));
    for (value, rules) in [
        (json!("compact"), over_file(|rules| rules.list_item_padding = Leaf::Value(ListItemPadding::None))),
        (json!("loose"), over_file(|rules| rules.list_item_padding = Leaf::Value(ListItemPadding::One))),
        (json!("default"), json_base_file_rules()),
        (Value::Null, json_base_file_rules()),
    ] {
        cell(
            format!("cleanup: {value}"),
            legacy(json!({ "cleanup": value })),
            Expected::migrated(rules, &["formatting.cleanup"], EDITOR_DEBOUNCE),
        );
    }
    for (label, value) in [
        ("wrong whole type", json!(1)),
        ("one wrong element", json!(["compact", 1])),
        ("every element wrong", json!([1])),
        ("empty \"\"", json!("")),
        ("empty {}", json!({})),
        ("unsupported", json!("tight")),
    ] {
        cell(format!("cleanup: {label}"), legacy(json!({ "cleanup": value })), rejected("formatting.cleanup"));
    }
    cell(
        "fixed_width: 80".into(),
        legacy(json!({ "fixed_width": 80 })),
        Expected::migrated(
            over_file(|rules| rules.fixed_width = Leaf::Value(FixedWidth::Columns(80))),
            &["formatting.fixed_width"],
            EDITOR_DEBOUNCE,
        ),
    );
    cell(
        "fixed_width: null".into(),
        legacy(json!({ "fixed_width": null })),
        Expected::migrated(json_base_file_rules(), &["formatting.fixed_width"], EDITOR_DEBOUNCE),
    );
    for (label, value) in [
        ("wrong whole type", json!("80")),
        ("one wrong element", json!([80, "x"])),
        ("every element wrong", json!(["x"])),
        ("empty \"\"", json!("")),
        ("empty {}", json!({})),
        ("zero", json!(0)),
        ("too wide", json!(1001)),
        ("fractional", json!(80.5)),
        ("negative", json!(-80)),
    ] {
        cell(format!("fixed_width: {label}"), legacy(json!({ "fixed_width": value })), rejected("formatting.fixed_width"));
    }
    cell(
        "mixed: cleanup with empty markdown".into(),
        legacy(json!({ "cleanup": "compact", "markdown": {} })),
        rejected("formatting"),
    );
    cell(
        "mixed: null fixed_width with the control markdown".into(),
        edit(settings, &["fixed_width"], Some(Value::Null)),
        rejected("formatting"),
    );
    cell(
        "envelope: bare section without the dmls wrapper".into(),
        settings["dmls"].clone(),
        Expected::accepted(control(), EDITOR_DEBOUNCE),
    );

    cells
}

/// The editor-settings matrix: control, every leaf cell including explicit
/// null, object roots, legacy keys, and semantic-invalid values, each applied
/// over a file that sets three rules.
#[test]
fn json_formatting_matrix() {
    let settings: Value = serde_json::from_str(
        &std::fs::read_to_string(
            biscuit_test_harness::manifest_dir!().join("tests/fixtures/formatting/settings.json"),
        )
        .expect("read the settings.json fixture"),
    )
    .expect("settings fixture is JSON");
    let temp = tempfile::tempdir().expect("temp dir");
    std::fs::write(temp.path().join(CONFIG_FILE_NAME), JSON_BASE_FILE).expect("write config");
    let apply = |settings: Value| {
        let mut state = ConfigState::load(vec![temp.path().to_path_buf()], None);
        assert_eq!(state.diagnostics(), [], "the base file is valid");
        assert_eq!(state.effective().formatting.markdown, json_base_file_rules());
        state.apply_client_settings(settings);
        state
    };
    let source = ConfigSource::EditorSettings;
    let mut failures = Vec::new();

    let state = apply(settings.clone());
    check(
        "control",
        &state,
        &Expected::accepted(control(), EDITOR_DEBOUNCE),
        &source,
        &mut failures,
    );
    for (label, settings, expected) in json_cells(&settings) {
        check(&label, &apply(settings), &expected, &source, &mut failures);
    }
    assert_no_failures(&failures);
}

// ---------------------------------------------------------------------------
// Merging, reloads, and boundaries
// ---------------------------------------------------------------------------

fn state_with_file(dir: &Path, text: &str) -> ConfigState {
    std::fs::write(dir.join(CONFIG_FILE_NAME), text).expect("write config");
    ConfigState::load(vec![dir.to_path_buf()], None)
}

/// File legacy keys and editor `formatting.markdown` are separate sources:
/// each is normalized on its own, so together they are not "mixed".
#[test]
fn file_legacy_keys_merge_with_editor_rules() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut state = state_with_file(temp.path(), "[formatting]\ncleanup = \"compact\"\nfixed_width = 100\n");
    state.apply_client_settings(json!({
        "dmls": { "formatting": { "markdown": { "fixed-width": 72, "unwrap": true } } }
    }));

    assert_eq!(
        state.effective().formatting.markdown,
        MarkdownRuleOverrides {
            list_item_padding: Leaf::Value(ListItemPadding::None),
            fixed_width: Leaf::Value(FixedWidth::Columns(72)),
            unwrap: Leaf::Value(true),
            ..none()
        }
    );
    let keys: Vec<_> = state
        .diagnostics()
        .iter()
        .map(|diagnostic| (diagnostic.severity, diagnostic.key.as_deref()))
        .collect();
    assert_eq!(
        keys,
        [
            (Warning, Some("formatting.cleanup")),
            (Warning, Some("formatting.fixed_width")),
        ]
    );
}

/// Diagnostics describe the latest change only: reloading and re-applying
/// settings never accumulate migration warnings.
#[test]
fn migration_warnings_are_recomputed_not_accumulated() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut state = state_with_file(temp.path(), "[formatting]\ncleanup = \"loose\"\n");
    assert_eq!(state.diagnostics().len(), 1);

    state.reload();
    state.reload();
    assert_eq!(state.diagnostics().len(), 1);

    let editor_legacy = json!({ "dmls": { "formatting": { "fixed_width": 90 } } });
    state.apply_client_settings(editor_legacy.clone());
    state.apply_client_settings(editor_legacy);
    let sources: Vec<_> = state
        .diagnostics()
        .iter()
        .map(|diagnostic| (diagnostic.source.clone(), diagnostic.key.as_deref()))
        .collect();
    assert_eq!(
        sources,
        [
            (
                ConfigSource::File(temp.path().join(CONFIG_FILE_NAME)),
                Some("formatting.cleanup")
            ),
            (ConfigSource::EditorSettings, Some("formatting.fixed_width")),
        ]
    );

    // Fixing the file clears its warning on the next change.
    std::fs::write(
        temp.path().join(CONFIG_FILE_NAME),
        "[formatting.markdown]\nlist-item-padding = \"one\"\n",
    )
    .expect("rewrite config");
    state.apply_client_settings(Value::Null);
    assert_eq!(state.diagnostics(), []);
    assert_eq!(
        state.effective().formatting.markdown.list_item_padding,
        Leaf::Value(ListItemPadding::One)
    );
}

/// Whole `settings: null` clears the overlay, restoring the file's rules
/// (the existing clear-overlay contract).
#[test]
fn null_settings_clear_the_editor_overlay() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut state = state_with_file(temp.path(), "[formatting.markdown]\nindentation = 8\n");
    state.apply_client_settings(json!({ "formatting": { "markdown": { "indentation": 2 } } }));
    assert_eq!(
        state.effective().formatting.markdown.indentation,
        Leaf::Value(Indentation::Two)
    );

    state.apply_client_settings(Value::Null);
    assert_eq!(
        state.effective().formatting.markdown,
        MarkdownRuleOverrides {
            indentation: Leaf::Value(Indentation::Eight),
            ..none()
        }
    );
}

/// A rejected file `formatting` table drops only the file's formatting; the
/// editor's valid rules still apply over defaults.
#[test]
fn rejected_file_formatting_leaves_editor_rules_usable() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut state = state_with_file(temp.path(), "[formatting.markdown]\nindentation = \"4\"\n");
    state.apply_client_settings(json!({ "formatting": { "markdown": { "unwrap": true } } }));
    assert_eq!(
        state.effective().formatting.markdown,
        MarkdownRuleOverrides {
            unwrap: Leaf::Value(true),
            ..none()
        }
    );
    assert_eq!(state.diagnostics().len(), 1);
    assert_eq!(state.diagnostics()[0].severity, Error);
}

/// Settings that are not an object keep the previous overlay and say so.
#[test]
fn non_object_settings_keep_the_previous_overlay() {
    let mut state = ConfigState::load(Vec::new(), None);
    state.apply_client_settings(json!({ "formatting": { "markdown": { "unwrap": true } } }));
    state.apply_client_settings(json!("not settings"));
    assert_eq!(state.effective().formatting.markdown.unwrap, Leaf::Value(true));
    assert_eq!(state.diagnostics().len(), 1);
    assert_eq!(state.diagnostics()[0].source, ConfigSource::EditorSettings);
    assert_eq!(state.diagnostics()[0].severity, Error);
}

/// Boundary (cell "X: Value"): editor settings reach the loader as a
/// `serde_json::Value`, which already kept only the last of duplicate member
/// names. The loader cannot see the duplicate and does not claim to detect
/// it; the last value is the one applied, with no diagnostic.
#[test]
fn duplicate_json_member_names_are_resolved_before_the_loader() {
    let raw = r#"{ "formatting": { "markdown": { "indentation": 8, "indentation": 2 } } }"#;
    let settings: Value = serde_json::from_str(raw).expect("serde_json accepts duplicate names");
    assert_eq!(settings["formatting"]["markdown"].as_object().map(|map| map.len()), Some(1));

    let mut state = ConfigState::load(Vec::new(), None);
    state.apply_client_settings(settings);
    assert_eq!(
        state.effective().formatting.markdown.indentation,
        Leaf::Value(Indentation::Two)
    );
    assert_eq!(state.diagnostics(), []);
}
