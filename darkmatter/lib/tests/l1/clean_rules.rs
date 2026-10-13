//! The `style.markdown` rule reader and precedence resolution
//! (`darkmatter::markdown::cleanup::rules`), driven from a checked-in
//! authored document.
//!
//! The YAML robustness matrix walks one mutation per cell over the control
//! fixture: every rule is a column, every input shape a row. Cell outcomes
//! follow the plan's notation: F falls through, N is an explicit null that
//! also falls through, D discards the whole `style.markdown` object with a
//! positioned diagnostic, and I is invalid YAML.

use biscuit_file::serde_yaml_ng;
use darkmatter::markdown::cleanup::diagnostic::{CleanCode, CleanDiagnostic, CleanStage};
use darkmatter::markdown::cleanup::rules::{
    DefaultLanguage, FixedWidth, Indentation, Leaf, ListInteriorPadding, ListItemPadding,
    MarkdownRuleOverrides, ResolvedMarkdownRules, RuleSource, document_rules, resolve,
};
use darkmatter::markdown::extract_frontmatter_block;

const CONTROL: &str = include_str!("../fixtures/clean/style-markdown-control.md");

/// Each rule's authored control value, as it appears in the fixture.
const COLUMNS: [(&str, &str); 10] = [
    ("frontmatter", "false"),
    ("table-layout", "false"),
    ("fixed-width", "80"),
    ("unwrap", "true"),
    ("indentation", "2"),
    ("ordered-lists", "false"),
    ("list-padding", "false"),
    ("list-item-padding", "one"),
    ("list-interior-padding", "all"),
    ("default-language", "sh"),
];

fn control_line(rule: &str, value: &str) -> String {
    format!("        {rule}: {value}\n")
}

/// The fixture with `rule`'s line replaced by `replacement` (whole lines,
/// terminators included).
fn mutate(rule: &str, control: &str, replacement: &str) -> String {
    let line = control_line(rule, control);
    assert!(CONTROL.contains(&line), "fixture must author `{rule}: {control}`");
    CONTROL.replacen(&line, replacement, 1)
}

/// Reads the rules exactly as a raw-source caller would: extract the block,
/// parse its YAML, then read `style.markdown` with document coordinates.
fn read(document: &str) -> Result<Leaf<MarkdownRuleOverrides>, Vec<CleanDiagnostic>> {
    let extraction = extract_frontmatter_block(document)
        .expect("frontmatter delimiters are well formed")
        .expect("fixture has frontmatter");
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(extraction.yaml).expect("mutation keeps YAML valid");
    document_rules(&yaml, extraction.yaml, extraction.yaml_span.start)
}

fn overrides(document: &str) -> MarkdownRuleOverrides {
    match read(document) {
        Ok(Leaf::Value(overrides)) => overrides,
        other => panic!("expected usable rules, got {other:?}"),
    }
}

/// The resolved value of `rule`, rendered the way the fixture spells it,
/// with its source.
fn resolved(rules: &ResolvedMarkdownRules, rule: &str) -> (String, RuleSource) {
    match rule {
        "frontmatter" => (rules.frontmatter.value.to_string(), rules.frontmatter.source),
        "table-layout" => (rules.table_layout.value.to_string(), rules.table_layout.source),
        "fixed-width" => (
            match rules.fixed_width.value {
                FixedWidth::Off => "false".to_string(),
                FixedWidth::Columns(columns) => columns.to_string(),
            },
            rules.fixed_width.source,
        ),
        "unwrap" => (rules.unwrap.value.to_string(), rules.unwrap.source),
        "indentation" => (
            rules
                .indentation
                .value
                .spaces()
                .map_or("preserve".to_string(), |spaces| spaces.to_string()),
            rules.indentation.source,
        ),
        "ordered-lists" => (rules.ordered_lists.value.to_string(), rules.ordered_lists.source),
        "list-padding" => (rules.list_padding.value.to_string(), rules.list_padding.source),
        "list-item-padding" => (
            match rules.list_item_padding.value {
                ListItemPadding::Preserve => "preserve",
                ListItemPadding::None => "none",
                ListItemPadding::One => "one",
            }
            .to_string(),
            rules.list_item_padding.source,
        ),
        "list-interior-padding" => (
            match rules.list_interior_padding.value {
                ListInteriorPadding::Preserve => "preserve",
                ListInteriorPadding::None => "none",
                ListInteriorPadding::ExceptNestedLists => "except-nested-lists",
                ListInteriorPadding::All => "all",
            }
            .to_string(),
            rules.list_interior_padding.source,
        ),
        "default-language" => (
            rules.default_language.value.as_str().to_string(),
            rules.default_language.source,
        ),
        other => panic!("unknown rule {other}"),
    }
}

fn leaf_is_null(overrides: &MarkdownRuleOverrides, rule: &str) -> bool {
    match rule {
        "frontmatter" => overrides.frontmatter == Leaf::Null,
        "table-layout" => overrides.table_layout == Leaf::Null,
        "fixed-width" => overrides.fixed_width == Leaf::Null,
        "unwrap" => overrides.unwrap == Leaf::Null,
        "indentation" => overrides.indentation == Leaf::Null,
        "ordered-lists" => overrides.ordered_lists == Leaf::Null,
        "list-padding" => overrides.list_padding == Leaf::Null,
        "list-item-padding" => overrides.list_item_padding == Leaf::Null,
        "list-interior-padding" => overrides.list_interior_padding == Leaf::Null,
        "default-language" => overrides.default_language == Leaf::Null,
        other => panic!("unknown rule {other}"),
    }
}

/// A discarded object: one `clean.invalid-rules` diagnostic per problem,
/// positioned on the authored text `expected_span_text`.
fn assert_discarded(document: &str, cell: &str, expected_span_text: &str) {
    let diagnostics = match read(document) {
        Err(diagnostics) => diagnostics,
        Ok(other) => panic!("{cell}: expected the object to be discarded, got {other:?}"),
    };
    assert_eq!(diagnostics.len(), 1, "{cell}: {diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.code, CleanCode::InvalidRules, "{cell}");
    assert_eq!(diagnostic.stage, CleanStage::Rules, "{cell}");
    assert!(diagnostic.repairs.is_empty(), "{cell}: rule findings are report-only");
    assert_eq!(
        &document[diagnostic.span.clone()],
        expected_span_text,
        "{cell}: diagnostic must sit on the authored value"
    );
    assert!(
        diagnostic.message.contains("ignored"),
        "{cell}: message must say the object was ignored: {}",
        diagnostic.message
    );
}

#[test]
fn yaml_matrix_walks_every_rule_and_shape() {
    // Control row: the unedited fixture sets every rule from the document,
    // so each mutation below is known to be load-bearing.
    let control = resolve(&MarkdownRuleOverrides::default(), Some(&overrides(CONTROL)), None);
    for (rule, value) in COLUMNS {
        assert_eq!(
            resolved(&control, rule),
            (value.to_string(), RuleSource::Document),
            "control: {rule}"
        );
    }
    let defaults = ResolvedMarkdownRules::defaults();

    // Wrong whole type, chosen per column to include the coercion traps:
    // quoted booleans and numerals are strings, never coerced.
    let wrong_type = [
        ("frontmatter", "123"),
        ("table-layout", "\"true\""),
        ("fixed-width", "\"80\""),
        ("unwrap", "1"),
        ("indentation", "\"2\""),
        ("ordered-lists", "\"false\""),
        ("list-padding", "0"),
        ("list-item-padding", "1"),
        ("list-interior-padding", "true"),
        ("default-language", "5"),
    ];

    for ((rule, value), (_, wrong)) in COLUMNS.into_iter().zip(wrong_type) {
        // Absent: falls through to the default; every other rule still comes
        // from the document.
        let absent = mutate(rule, value, "");
        let rules = resolve(&MarkdownRuleOverrides::default(), Some(&overrides(&absent)), None);
        assert_eq!(resolved(&rules, rule), resolved(&defaults, rule), "absent: {rule}");
        for (other, other_value) in COLUMNS.into_iter().filter(|(name, _)| *name != rule) {
            assert_eq!(
                resolved(&rules, other),
                (other_value.to_string(), RuleSource::Document),
                "absent {rule} must not disturb {other}"
            );
        }

        // Explicit null, both spellings: recorded as null, and falls
        // through exactly like absent.
        for spelling in ["null", ""] {
            let null = mutate(rule, value, &format!("        {rule}:{}{spelling}\n", if spelling.is_empty() { "" } else { " " }));
            let read = overrides(&null);
            assert!(leaf_is_null(&read, rule), "null `{spelling}`: {rule}");
            let rules = resolve(&MarkdownRuleOverrides::default(), Some(&read), None);
            assert_eq!(resolved(&rules, rule), resolved(&defaults, rule), "null: {rule}");
        }

        // Wrong whole type.
        assert_discarded(&mutate(rule, value, &control_line(rule, wrong)), &format!("wrong type: {rule}"), wrong);

        // One wrong element, and every element wrong: a list is never a
        // rule value.
        let one_wrong = format!("[{value}, 123]");
        assert_discarded(&mutate(rule, value, &control_line(rule, &one_wrong)), &format!("one wrong element: {rule}"), &one_wrong);
        assert_discarded(&mutate(rule, value, &control_line(rule, "[123]")), &format!("every element wrong: {rule}"), "[123]");

        // Empty values. `default-language: ""` is the explicit "add no
        // language" value; every other empty shape is invalid.
        for empty in ["\"\"", "[]", "{}"] {
            let document = mutate(rule, value, &control_line(rule, empty));
            if rule == "default-language" && empty == "\"\"" {
                let rules = resolve(&MarkdownRuleOverrides::default(), Some(&overrides(&document)), None);
                assert_eq!(resolved(&rules, rule), (String::new(), RuleSource::Document));
            } else {
                assert_discarded(&document, &format!("empty {empty}: {rule}"), empty);
            }
        }

        // Duplicate key and trailing garbage: invalid YAML, so no rule is
        // read at all and there is no arbitrary winner.
        let duplicate = mutate(rule, value, &format!("{}{}", control_line(rule, value), control_line(rule, value)));
        let invalid = mutate(rule, value, &format!("{}        ]garbage: [\n", control_line(rule, value)));
        for (cell, document) in [("duplicate key", &duplicate), ("trailing garbage", &invalid)] {
            let extraction = extract_frontmatter_block(document).unwrap().unwrap();
            assert!(
                serde_yaml_ng::from_str::<serde_yaml_ng::Value>(extraction.yaml).is_err(),
                "{cell}: {rule} must be invalid YAML"
            );
        }
        let extraction = extract_frontmatter_block(&duplicate).unwrap().unwrap();
        assert!(
            biscuit_file::analyze_yaml(extraction.yaml)
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == biscuit_file::YamlDiagnosticCode::DuplicateKey),
            "duplicate {rule} must be reported as a duplicate key"
        );
    }
}

#[test]
fn ordinary_markdown_after_the_closing_delimiter_is_body_not_trailing_yaml() {
    let extraction = extract_frontmatter_block(CONTROL).unwrap().unwrap();
    assert!(CONTROL[extraction.body_span.clone()].contains("Intro line one"));
    assert!(read(CONTROL).is_ok());
}

#[test]
fn semantic_invalid_values_discard_the_whole_object() {
    let cells: [(&str, &str, &str); 10] = [
        ("fixed-width", "80", "0"),
        ("fixed-width", "80", "1001"),
        ("fixed-width", "80", "80.5"),
        ("fixed-width", "80", "\"{{ width }}\""),
        ("indentation", "2", "3"),
        ("indentation", "2", "\"preserve \""),
        ("list-item-padding", "one", "two"),
        ("list-interior-padding", "all", "some"),
        ("default-language", "sh", "\"a`b\""),
        ("default-language", "sh", "\"$(echo sh)\""),
    ];
    for (rule, control, value) in cells {
        assert_discarded(&mutate(rule, control, &control_line(rule, value)), &format!("{rule}: {value}"), value);
    }
    // A line break and a control character are rejected even when quoted.
    for value in ["\"a\\nb\"", "\"a\\tb\""] {
        assert_discarded(&mutate("default-language", "sh", &control_line("default-language", value)), value, value);
    }
}

#[test]
fn unknown_and_snake_case_rule_names_are_rejected_not_ignored() {
    for name in ["bogus", "fixed_width", "list_item_padding"] {
        let document = CONTROL.replacen(
            "        unwrap: true\n",
            &format!("        unwrap: true\n        {name}: 1\n"),
            1,
        );
        assert_discarded(&document, name, name);
    }
}

#[test]
fn every_problem_is_reported_in_source_order() {
    let document = CONTROL
        .replacen("        unwrap: true\n", "        unwrap: maybe\n", 1)
        .replacen("        frontmatter: false\n", "        frontmatter: 7\n", 1);
    let diagnostics = read(&document).expect_err("two invalid leaves");
    let texts: Vec<&str> = diagnostics
        .iter()
        .map(|diagnostic| &document[diagnostic.span.clone()])
        .collect();
    assert_eq!(texts, ["7", "maybe"]);
}

#[test]
fn object_roots_cover_absent_null_wrong_and_empty_shapes() {
    let with_style = |style: &str| format!("---\ntitle: t\n{style}---\nbody\n");

    // `style` and `style.markdown` absent or null: no document override.
    assert_eq!(read(&with_style("")).unwrap(), Leaf::Absent);
    assert_eq!(read(&with_style("style: null\n")).unwrap(), Leaf::Null);
    assert_eq!(read(&with_style("style:\n")).unwrap(), Leaf::Null);
    assert_eq!(read(&with_style("style:\n  page: {}\n")).unwrap(), Leaf::Absent);
    assert_eq!(read(&with_style("style:\n  markdown: null\n")).unwrap(), Leaf::Null);
    assert_eq!(read(&with_style("style:\n  markdown:\n")).unwrap(), Leaf::Null);

    // `{}` is an explicit, valid object with no leaves.
    assert_eq!(read(&with_style("style: {}\n")).unwrap(), Leaf::Absent);
    let empty = read(&with_style("style:\n  markdown: {}\n")).unwrap();
    assert_eq!(empty, Leaf::Value(MarkdownRuleOverrides::default()));

    // Wrong types and the other empty shapes discard the object.
    for (style, span_text) in [
        ("style: 5\n", "5"),
        ("style: []\n", "[]"),
        ("style: \"\"\n", "\"\""),
        ("style: [markdown]\n", "[markdown]"),
        ("style:\n  markdown: 5\n", "5"),
        ("style:\n  markdown: []\n", "[]"),
        ("style:\n  markdown: \"\"\n", "\"\""),
        ("style:\n  markdown: [unwrap, 1]\n", "[unwrap, 1]"),
        ("style:\n  markdown: [1]\n", "[1]"),
        ("style:\n  markdown: \"{{ rules }}\"\n", "\"{{ rules }}\""),
    ] {
        assert_discarded(&with_style(style), style, span_text);
    }
}

#[test]
fn precedence_resolves_each_rule_independently() {
    let cli = MarkdownRuleOverrides {
        indentation: Leaf::Value(Indentation::Eight),
        ..MarkdownRuleOverrides::default()
    };
    let document = MarkdownRuleOverrides {
        indentation: Leaf::Value(Indentation::Two),
        unwrap: Leaf::Value(true),
        list_padding: Leaf::Null,
        ..MarkdownRuleOverrides::default()
    };
    let workspace = MarkdownRuleOverrides {
        indentation: Leaf::Value(Indentation::Preserve),
        unwrap: Leaf::Value(false),
        list_padding: Leaf::Value(false),
        default_language: Leaf::Value(DefaultLanguage::new("plain").unwrap()),
        ..MarkdownRuleOverrides::default()
    };
    let rules = resolve(&cli, Some(&document), Some(&workspace));

    // CLI beats the document.
    assert_eq!((rules.indentation.value, rules.indentation.source), (Indentation::Eight, RuleSource::Cli));
    // The document beats the workspace.
    assert_eq!((rules.unwrap.value, rules.unwrap.source), (true, RuleSource::Document));
    // A document null releases the rule to the workspace.
    assert_eq!((rules.list_padding.value, rules.list_padding.source), (false, RuleSource::Workspace));
    // The workspace beats the runtime default.
    assert_eq!(rules.default_language.value.as_str(), "plain");
    assert_eq!(rules.default_language.source, RuleSource::Workspace);
    // Nothing set anywhere: the runtime default.
    assert_eq!((rules.ordered_lists.value, rules.ordered_lists.source), (true, RuleSource::Default));

    // Dropping a level, as an invalid `style.markdown` does, leaves the
    // others in force.
    let without_document = resolve(&cli, None, Some(&workspace));
    assert_eq!(without_document.unwrap.source, RuleSource::Workspace);
    assert_eq!(without_document.indentation.source, RuleSource::Cli);
}

#[test]
fn an_invalid_document_object_falls_back_to_the_other_levels() {
    let document = mutate("indentation", "2", &control_line("indentation", "3"));
    let workspace = MarkdownRuleOverrides {
        unwrap: Leaf::Value(true),
        ..MarkdownRuleOverrides::default()
    };
    let document_level = match read(&document) {
        Ok(Leaf::Value(overrides)) => Some(overrides),
        Ok(_) => None,
        Err(diagnostics) => {
            assert_eq!(diagnostics.len(), 1);
            None
        }
    };
    let rules = resolve(&MarkdownRuleOverrides::default(), document_level.as_ref(), Some(&workspace));
    // None of the fixture's otherwise-valid values survive the invalid leaf.
    assert_eq!(rules.frontmatter.source, RuleSource::Default);
    assert_eq!(rules.indentation.value, Indentation::Four);
    assert_eq!((rules.unwrap.value, rules.unwrap.source), (true, RuleSource::Workspace));
}

#[test]
fn width_implies_unwrapping_only_after_precedence() {
    let width = |width| MarkdownRuleOverrides {
        fixed_width: Leaf::Value(width),
        ..MarkdownRuleOverrides::default()
    };

    // A positive width at any level implies unwrapping.
    let rules = resolve(&MarkdownRuleOverrides::default(), Some(&width(FixedWidth::Columns(80))), None);
    assert!(!rules.unwrap.value);
    assert!(rules.joins_soft_breaks());
    assert_eq!(rules.wrap_columns(), Some(80));

    // `fixed-width: true` means 120 columns.
    let document = overrides(&mutate("fixed-width", "80", &control_line("fixed-width", "true")));
    let rules = resolve(&MarkdownRuleOverrides::default(), Some(&document), None);
    assert_eq!(rules.wrap_columns(), Some(120));

    // `fixed-width: false` does not cancel an independent `unwrap: true`.
    let workspace = MarkdownRuleOverrides {
        unwrap: Leaf::Value(true),
        ..MarkdownRuleOverrides::default()
    };
    let rules = resolve(&MarkdownRuleOverrides::default(), Some(&width(FixedWidth::Off)), Some(&workspace));
    assert!(rules.joins_soft_breaks());
    assert_eq!(rules.wrap_columns(), None);

    // `--ignore-incidental-newlines` is CLI `unwrap: false` plus
    // `fixed-width: false`, which overrides a document width.
    let ignore_incidental = MarkdownRuleOverrides {
        unwrap: Leaf::Value(false),
        fixed_width: Leaf::Value(FixedWidth::Off),
        ..MarkdownRuleOverrides::default()
    };
    let rules = resolve(&ignore_incidental, Some(&width(FixedWidth::Columns(80))), None);
    assert!(!rules.joins_soft_breaks());
}

#[test]
fn tightness_changes_are_permitted_only_by_an_explicit_padding_value() {
    let defaults = ResolvedMarkdownRules::defaults();
    assert!(!defaults.permits_tightness_change());

    let preserve = MarkdownRuleOverrides {
        list_item_padding: Leaf::Value(ListItemPadding::Preserve),
        list_interior_padding: Leaf::Value(ListInteriorPadding::Preserve),
        ..MarkdownRuleOverrides::default()
    };
    assert!(!resolve(&MarkdownRuleOverrides::default(), Some(&preserve), None).permits_tightness_change());

    let item = MarkdownRuleOverrides {
        list_item_padding: Leaf::Value(ListItemPadding::None),
        ..MarkdownRuleOverrides::default()
    };
    let rules = resolve(&item, None, None);
    assert!(rules.permits_item_padding_change());
    assert!(!rules.permits_interior_padding_change());

    let interior = MarkdownRuleOverrides {
        list_interior_padding: Leaf::Value(ListInteriorPadding::ExceptNestedLists),
        ..MarkdownRuleOverrides::default()
    };
    let rules = resolve(&MarkdownRuleOverrides::default(), None, Some(&interior));
    assert!(!rules.permits_item_padding_change());
    assert!(rules.permits_interior_padding_change());
    assert!(rules.permits_tightness_change());
}

#[test]
fn default_and_all_body_rules_off_profiles() {
    let defaults = ResolvedMarkdownRules::defaults();
    let expected_defaults = [
        ("frontmatter", "true"),
        ("table-layout", "true"),
        ("fixed-width", "false"),
        ("unwrap", "false"),
        ("indentation", "4"),
        ("ordered-lists", "true"),
        ("list-padding", "true"),
        ("list-item-padding", "preserve"),
        ("list-interior-padding", "preserve"),
        ("default-language", "text"),
    ];
    for (rule, value) in expected_defaults {
        assert_eq!(resolved(&defaults, rule), (value.to_string(), RuleSource::Default), "{rule}");
    }
    assert!(!defaults.joins_soft_breaks());

    let off = ResolvedMarkdownRules::all_body_rules_off();
    let expected_off = [
        ("frontmatter", "true", RuleSource::Default),
        ("table-layout", "false", RuleSource::Cli),
        ("fixed-width", "false", RuleSource::Cli),
        ("unwrap", "false", RuleSource::Cli),
        ("indentation", "preserve", RuleSource::Cli),
        ("ordered-lists", "false", RuleSource::Cli),
        ("list-padding", "false", RuleSource::Cli),
        ("list-item-padding", "preserve", RuleSource::Cli),
        ("list-interior-padding", "preserve", RuleSource::Cli),
        ("default-language", "", RuleSource::Cli),
    ];
    for (rule, value, source) in expected_off {
        assert_eq!(resolved(&off, rule), (value.to_string(), source), "{rule}");
    }
    assert!(!off.joins_soft_breaks());
    assert!(!off.permits_tightness_change());
}

/// Defaults are runtime values: they hold with the baseline disabled, and a
/// document schema that loosens `style` cannot authorize a value the rules
/// reject.
#[test]
fn rules_do_not_depend_on_the_baseline_or_a_document_schema() {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::schemas::DarkmatterSchemas;

    let document = "---\n$schema:\n  style: any\nstyle:\n  markdown:\n    indentation: 3\n---\nbody\n";
    let no_baseline = DarkmatterSchemas::new(crate::request_support::cwd_context());
    let md: Markdown = document.into();
    let report = no_baseline.validate(&md).expect("validation runs");
    assert!(report.valid, "the document schema authorizes any style: {:?}", report.problems);
    assert!(read(document).is_err(), "the rule validator still rejects indentation 3");

    assert_eq!(ResolvedMarkdownRules::defaults().indentation.value, Indentation::Four);
}
