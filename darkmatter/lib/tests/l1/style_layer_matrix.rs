//! The input-robustness matrix of the sparse style-layer validators, one
//! `#[test]` per cell, at the public preparation boundary
//! ([`prepare_style_layers`]).
//!
//! Each validator family gets its full shape walk once (control, absent,
//! explicit null, wrong whole type, empty, and the family's own invalid
//! values); every other field sharing that validator gets one control and one
//! invalid case. Each cell edits one thing in a valid layer. A failing cell
//! asserts the problem's code, its canonical property path, and the layer
//! that supplied it (the `--style` argument or the setter argument; a setter
//! has no position). A passing cell asserts the typed effective value.
//!
//! Loader document shapes (duplicate keys, trailing content, format syntax)
//! are the CLI's: `darkmatter-cli`'s `render_style_matrix` covers them.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{CodeBlockMode, ThemePair};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use darkmatter::style::schema::page::PageStyle;
use darkmatter::style::{
    PreparedStyle, StyleLayer, StylePreparationError, StylePreparationPolicy, StyleSetter, StyleSourceKind,
    prepare_style_layers,
};
use renderable::layout::{Alignment, Length};
use renderable::style::BorderWeight;
use serde_json::{Value, json};
use simplified_schema::PropertyPath;

const FILE: &str = "style.yaml";
/// A custom style schema admitting one number array and one string, for the
/// collection cells; it replaces the built-in style definition.
const ARRAY_SCHEMA: &str = "$schema:\n    style:\n        tags: number[]\n        foo: string\n";
/// A custom style schema requiring `page.color`.
const REQUIRED_SCHEMA: &str =
    "$schema:\n    style:\n        page:\n            color: string(required)\n            theme: string\n";

/// One layer of a cell, in precedence order.
enum L {
    Doc(Value),
    File(Value),
    Set(&'static [&'static str]),
}

/// The document schema a cell prepares under.
enum Schema {
    Baseline,
    Custom(&'static str),
}

enum Expect {
    /// Preparation succeeds; the check reads the effective style.
    Typed(fn(&PreparedStyle)),
    /// Preparation fails with a problem holding `code` at the canonical
    /// `path`, supplied by `source` (the `--style` argument, the setter
    /// argument, or `""` for the merged style, which has no origin).
    Fails { code: &'static str, path: &'static str, source: &'static str },
}

fn fails(code: &'static str, path: &'static str, source: &'static str) -> Expect {
    Expect::Fails { code, path, source }
}

const TYPE_MISMATCH: &str = "dm.schema.type_mismatch";
const CONSTRAINT: &str = "dm.schema.constraint";
const INVALID_VALUE: &str = "dm.style.invalid_value";
const INVALID_ROOT: &str = "dm.style.invalid_root";
const INVALID_SETTER: &str = "dm.style.invalid_setter";
const EMPTY_FILE: &str = "dm.style.empty_file";
const UNKNOWN_KEY: &str = "dm.schema.unknown_key";
const MISSING_REQUIRED: &str = "dm.schema.missing_required";

fn run(schema: Schema, layers: Vec<L>, expect: Expect) {
    let dir = tempfile::tempdir().expect("temp dir");
    let context = FileResolutionContext::new(dir.path().to_path_buf());
    let frontmatter = match schema {
        Schema::Baseline => "title: Notes\n",
        Schema::Custom(frontmatter) => frontmatter,
    };
    let markdown: Markdown = format!("---\n{frontmatter}---\nBody\n").as_str().into();
    let schema: EffectiveSchema = DarkmatterSchemas::new(context.clone())
        .with_darkmatter_baseline_json_schema()
        .expect("the compiled baseline loads")
        .effective_for(&markdown)
        .expect("the schema resolves")
        .expect("the baseline applies");
    let layers: Vec<StyleLayer> = layers
        .into_iter()
        .map(|layer| match layer {
            L::Doc(value) => StyleLayer::document(value, "doc.md", context.clone(), None),
            L::File(value) => StyleLayer::file(value, FILE, context.clone(), None),
            L::Set(arguments) => {
                let setters = arguments
                    .iter()
                    .map(|argument| {
                        let (key, raw) = argument.split_once('=').expect("a setter has `=`");
                        StyleSetter::new(*argument, PropertyPath::parse(key).expect("a setter path"), raw)
                    })
                    .collect();
                StyleLayer::setters(setters, context.clone())
            }
        })
        .collect();
    let result = prepare_style_layers(&layers, Some(&schema), StylePreparationPolicy::render());
    match (expect, result) {
        (Expect::Typed(check), Ok(prepared)) => check(&prepared),
        (Expect::Typed(_), Err(error)) => panic!("expected success, got {}", describe(&error)),
        (Expect::Fails { .. }, Ok(prepared)) => panic!("expected a failure, got {:#}", prepared.value()),
        (Expect::Fails { code, path, source }, Err(error)) => {
            let found = error.problems.iter().any(|problem| {
                let origin = problem.origin.as_ref().map_or("", |origin| origin.source.as_str());
                problem.code == code && problem.path == path && origin == source
            });
            assert!(found, "expected {code} at `{path}` from `{source}`, got {}", describe(&error));
            for problem in &error.problems {
                if let Some(origin) = &problem.origin
                    && origin.kind == StyleSourceKind::Setters
                {
                    assert_eq!(origin.span, None, "a setter has no position: {problem:?}");
                }
            }
        }
    }
}

fn describe(error: &StylePreparationError) -> String {
    let problems: Vec<String> = error
        .problems
        .iter()
        .map(|problem| {
            let origin = problem.origin.as_ref().map_or("", |origin| origin.source.as_str());
            format!("{} at `{}` from `{origin}`: {}", problem.code, problem.path, problem.message)
        })
        .collect();
    format!("{problems:#?}")
}

fn page(prepared: &PreparedStyle) -> PageStyle {
    prepared.style().page.clone().unwrap_or_default()
}

fn table_border(prepared: &PreparedStyle) -> renderable::style::Border {
    let table = prepared.style().table.clone().expect("table");
    table.common.border.expect("border").as_border().clone()
}

/// One `#[test]` per cell.
macro_rules! cells {
    ($($name:ident: $schema:expr, [$($layer:expr),* $(,)?] => $expect:expr;)+) => {
        $(
            #[test]
            fn $name() {
                run($schema, vec![$($layer),*], $expect);
            }
        )+
    };
}

use L::{Doc, File, Set};
use Schema::{Baseline, Custom};

// CLI style root: a mapping, never a scalar, array, or null; an inline `{}`
// is a no-op, a file `{}` fails, and no layer at all is the default style.
cells! {
    root_mapping_control_sets_every_margin: Baseline, [File(json!({"margin": 2}))] => Expect::Typed(|prepared| {
        let page = page(prepared);
        assert_eq!(
            (page.margin_top, page.margin_right, page.margin_bottom, page.margin_left),
            (Some(2), Some(Length::Ch(2)), Some(2), Some(Length::Ch(2)))
        );
    });
    root_file_array_fails: Baseline, [File(json!([{"margin": 2}]))] => fails(INVALID_ROOT, "", FILE);
    root_file_string_fails: Baseline, [File(json!("margin: 2"))] => fails(INVALID_ROOT, "", FILE);
    root_setter_array_fails: Baseline, [Set(&["style=[1]"])] => fails(INVALID_ROOT, "", "style=[1]");
    root_setter_null_fails: Baseline, [Set(&["style=null"])] => fails(INVALID_ROOT, "", "style=null");
    root_inline_empty_over_a_document_is_a_no_op: Baseline, [Doc(json!({"margin": 3})), Set(&["style={}"])] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_top, Some(3)));
    root_file_empty_over_a_document_fails: Baseline, [Doc(json!({"margin": 3})), File(json!({}))] =>
        fails(EMPTY_FILE, "", FILE);
    root_no_layer_is_the_default_style: Baseline, [] => Expect::Typed(|prepared| {
        assert_eq!(prepared.value(), &json!({}));
        assert!(prepared.style().page.is_none());
    });
}

// Page rows (u16 terminal rows): the full walk on `margin-top`.
cells! {
    rows_control: Baseline, [File(json!({"page": {"margin-top": 2}}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_top, Some(2)));
    rows_absent_preserves_the_lower_layer: Baseline, [Doc(json!({"page": {"margin-top": 3}})), File(json!({"ml": 1}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_top, Some(3)));
    rows_file_null_is_absent: Baseline, [Doc(json!({"page": {"margin-top": 3}})), File(json!({"page": {"margin-top": null}, "ml": 1}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_top, Some(3)));
    rows_setter_null_is_absent: Baseline, [Doc(json!({"page": {"margin-top": 3}})), Set(&["style.page.margin-top=null"])] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_top, Some(3)));
    rows_zero_is_explicit: Baseline, [Doc(json!({"page": {"margin-top": 3}})), File(json!({"page": {"margin-top": 0}}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_top, Some(0)));
    rows_max_is_65535: Baseline, [File(json!({"page": {"margin-top": 65535}}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_top, Some(65535)));
    rows_wrong_type_fails: Baseline, [File(json!({"page": {"margin-top": true}}))] => fails(TYPE_MISMATCH, "page.margin-top", FILE);
    rows_empty_string_fails: Baseline, [File(json!({"page": {"margin-top": ""}}))] => fails(TYPE_MISMATCH, "page.margin-top", FILE);
    rows_negative_fails: Baseline, [File(json!({"page": {"margin-top": -1}}))] => fails(CONSTRAINT, "page.margin-top", FILE);
    rows_fraction_fails: Baseline, [File(json!({"page": {"margin-top": 1.5}}))] => fails(TYPE_MISMATCH, "page.margin-top", FILE);
    rows_over_65535_fails: Baseline, [File(json!({"page": {"margin-top": 65536}}))] => fails(CONSTRAINT, "page.margin-top", FILE);
    rows_unit_bearing_fails: Baseline, [File(json!({"page": {"margin-top": "2ch"}}))] => fails(TYPE_MISMATCH, "page.margin-top", FILE);
    rows_setter_wrong_type_fails: Baseline, [Set(&["style.page.margin-top=abc"])] =>
        fails(TYPE_MISMATCH, "page.margin-top", "style.page.margin-top=abc");
}

// Fields sharing the row validator: one control and one invalid case each.
cells! {
    rows_mb_control: Baseline, [Set(&["style.mb=4"])] => Expect::Typed(|prepared| assert_eq!(page(prepared).margin_bottom, Some(4)));
    rows_mb_negative_fails: Baseline, [Set(&["style.mb=-4"])] => fails(TYPE_MISMATCH, "page.mb", "style.mb=-4");
    rows_padding_top_control: Baseline, [File(json!({"padding-top": 1}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).padding_top, Some(1)));
    rows_padding_top_unit_fails: Baseline, [File(json!({"padding-top": "1ch"}))] => fails(TYPE_MISMATCH, "page.padding-top", FILE);
    rows_my_control: Baseline, [File(json!({"my": 2}))] => Expect::Typed(|prepared| {
        let page = page(prepared);
        assert_eq!((page.margin_top, page.margin_bottom, page.margin_left), (Some(2), Some(2), None));
    });
    rows_my_unit_fails: Baseline, [File(json!({"my": "2ch"}))] => fails(TYPE_MISMATCH, "page.my", FILE);
    rows_py_control: Baseline, [Set(&["style.page.py=1"])] => Expect::Typed(|prepared| assert_eq!(page(prepared).padding_bottom, Some(1)));
    rows_py_fraction_fails: Baseline, [Set(&["style.page.py=0.5"])] => fails(TYPE_MISMATCH, "page.py", "style.page.py=0.5");
    all_sides_padding_control: Baseline, [File(json!({"padding": 1}))] => Expect::Typed(|prepared| {
        let page = page(prepared);
        assert_eq!((page.padding_top, page.padding_left), (Some(1), Some(Length::Ch(1))));
    });
    all_sides_margin_percent_fails: Baseline, [File(json!({"margin": "10%"}))] => fails(TYPE_MISMATCH, "page.margin", FILE);
    all_sides_padding_over_65535_fails: Baseline, [Set(&["style.padding=70000"])] => fails(TYPE_MISMATCH, "page.padding", "style.padding=70000");
}

// Horizontal lengths (`2ch`, `10%`, or cells): the full walk on `margin-left`.
cells! {
    length_control: Baseline, [File(json!({"ml": "2ch"}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_left, Some(Length::Ch(2))));
    length_integer_cells: Baseline, [File(json!({"page": {"margin-left": 5}}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_left, Some(Length::Ch(5))));
    length_percent: Baseline, [File(json!({"page": {"margin-left": "10%"}}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_left, Some(Length::Percent(10.0))));
    length_absent_preserves_the_lower_layer: Baseline, [Doc(json!({"ml": 4})), File(json!({"mt": 1}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_left, Some(Length::Ch(4))));
    length_file_null_is_absent: Baseline, [Doc(json!({"ml": 4})), File(json!({"ml": null, "mt": 1}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_left, Some(Length::Ch(4))));
    length_setter_null_is_absent: Baseline, [Doc(json!({"ml": 4})), Set(&["style.ml=null"])] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).margin_left, Some(Length::Ch(4))));
    length_zero_is_explicit: Baseline, [Doc(json!({"ml": 4})), Set(&["style.ml=0"])] =>
        Expect::Typed(|prepared| assert!(matches!(page(prepared).margin_left, Some(Length::Zero | Length::Ch(0))), "{:?}", page(prepared).margin_left));
    length_wrong_type_fails: Baseline, [File(json!({"ml": true}))] => fails(INVALID_VALUE, "page.margin-left", FILE);
    length_empty_string_fails: Baseline, [File(json!({"ml": ""}))] => fails(INVALID_VALUE, "page.margin-left", FILE);
    length_unknown_unit_fails: Baseline, [File(json!({"ml": "2px"}))] => fails(INVALID_VALUE, "page.margin-left", FILE);
    length_negative_fails: Baseline, [File(json!({"ml": -2}))] => fails(INVALID_VALUE, "page.margin-left", FILE);
    length_negative_text_fails: Baseline, [Set(&["style.ml=-2ch"])] => fails(INVALID_VALUE, "page.margin-left", "style.ml=-2ch");
    length_percent_over_100_fails: Baseline, [File(json!({"ml": "150%"}))] => fails(INVALID_VALUE, "page.margin-left", FILE);
}

// Fields sharing the horizontal length validator.
cells! {
    length_mx_percent_control: Baseline, [File(json!({"mx": "10%"}))] => Expect::Typed(|prepared| {
        let page = page(prepared);
        assert_eq!((page.margin_left, page.margin_right), (Some(Length::Percent(10.0)), Some(Length::Percent(10.0))));
    });
    length_mx_unit_fails: Baseline, [File(json!({"mx": "2em"}))] => fails(INVALID_VALUE, "page.margin-right", FILE);
    length_padding_right_control: Baseline, [Set(&["style.page.padding-right=3ch"])] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).padding_right, Some(Length::Ch(3))));
    length_padding_right_empty_fails: Baseline, [Set(&["style.page.padding-right=''"])] =>
        fails(INVALID_VALUE, "page.padding-right", "style.page.padding-right=''");
    length_component_padding_left_control: Baseline, [File(json!({"table": {"padding-left": "2ch"}}))] =>
        Expect::Typed(|prepared| assert_eq!(prepared.style().table.clone().expect("table").common.padding_left, Some(Length::Ch(2))));
    length_component_padding_left_unit_fails: Baseline, [File(json!({"table": {"padding-left": "2px"}}))] =>
        fails(INVALID_VALUE, "table.padding-left", FILE);
}

// The page content cap (`max-width`): positive only.
cells! {
    cap_control: Baseline, [File(json!({"max-width": 80}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).max_width, Some(Length::Ch(80))));
    cap_percent_control: Baseline, [File(json!({"max-width": "50%"}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).max_width, Some(Length::Percent(50.0))));
    cap_null_preserves_the_lower_layer: Baseline, [Doc(json!({"max-width": 60})), Set(&["style.max-width=null"])] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).max_width, Some(Length::Ch(60))));
    cap_zero_fails: Baseline, [File(json!({"max-width": 0}))] => fails(CONSTRAINT, "page.max-width", FILE);
    cap_zero_cells_text_fails: Baseline, [File(json!({"max-width": "0ch"}))] => fails(CONSTRAINT, "page.max-width", FILE);
    cap_zero_percent_fails: Baseline, [Set(&["style.max-width=0%"])] => fails(CONSTRAINT, "page.max-width", "style.max-width=0%");
    cap_empty_fails: Baseline, [File(json!({"max-width": ""}))] => fails(CONSTRAINT, "page.max-width", FILE);
    cap_wrong_type_fails: Baseline, [File(json!({"max-width": [80]}))] => fails(CONSTRAINT, "page.max-width", FILE);
}

// Enum, theme, and bool fields: the full walk on `page.theme`.
cells! {
    theme_control: Baseline, [File(json!({"page": {"theme": "github"}}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).theme, Some(ThemePair::Github)));
    theme_alias_is_accepted: Baseline, [Set(&["style.theme=Visual_Studio_Dark"])] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).theme, Some(ThemePair::VisualStudioDark)));
    theme_absent_preserves_the_lower_layer: Baseline, [Doc(json!({"theme": "nord"})), File(json!({"ml": 1}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).theme, Some(ThemePair::Nord)));
    theme_null_preserves_the_lower_layer: Baseline, [Doc(json!({"theme": "nord"})), File(json!({"theme": null, "ml": 1}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).theme, Some(ThemePair::Nord)));
    theme_wrong_type_fails: Baseline, [File(json!({"theme": 5}))] => fails(INVALID_VALUE, "page.theme", FILE);
    theme_unknown_fails: Baseline, [File(json!({"theme": "neon"}))] => fails(INVALID_VALUE, "page.theme", FILE);
    theme_empty_fails: Baseline, [File(json!({"theme": ""}))] => fails(CONSTRAINT, "page.theme", FILE);
    theme_setter_empty_fails: Baseline, [Set(&["style.page.theme="])] => fails(CONSTRAINT, "page.theme", "style.page.theme=");
}

// Fields sharing the enum, theme, and bool validators.
cells! {
    code_mode_control: Baseline, [Set(&["style.page.code.mode=dark"])] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).code.expect("code").mode, Some(CodeBlockMode::Dark)));
    code_mode_unknown_fails: Baseline, [Set(&["style.page.code.mode=sideways"])] =>
        fails(CONSTRAINT, "page.code.mode", "style.page.code.mode=sideways");
    code_theme_control: Baseline, [File(json!({"code": {"theme": "dracula"}}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).code.expect("code").theme, Some(ThemePair::Dracula)));
    code_theme_unknown_fails: Baseline, [File(json!({"code": {"theme": "neon"}}))] => fails(INVALID_VALUE, "page.code.theme", FILE);
    alignment_control: Baseline, [File(json!({"page": {"alignment": "right"}}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).alignment, Some(Alignment::Right)));
    alignment_unknown_fails: Baseline, [File(json!({"page": {"alignment": "middle"}}))] => fails(CONSTRAINT, "page.alignment", FILE);
    line_numbers_false_control: Baseline, [Doc(json!({"code-block": {"line-numbers": true}})), Set(&["style.code-block.line-numbers=false"])] =>
        Expect::Typed(|prepared| assert_eq!(prepared.style().code_block.clone().expect("code-block").line_numbers, Some(false)));
    line_numbers_null_preserves_the_lower_layer: Baseline, [Doc(json!({"code-block": {"line-numbers": true}})), Set(&["style.code-block.line-numbers=null"])] =>
        Expect::Typed(|prepared| assert_eq!(prepared.style().code_block.clone().expect("code-block").line_numbers, Some(true)));
    line_numbers_text_fails: Baseline, [File(json!({"code-block": {"line-numbers": "yes"}}))] =>
        fails(INVALID_VALUE, "code-block.line-numbers", FILE);
    line_numbers_setter_boolean_word_converts: Baseline, [Set(&["style.code-block.line-numbers=no"])] =>
        Expect::Typed(|prepared| assert_eq!(prepared.style().code_block.clone().expect("code-block").line_numbers, Some(false)));
    line_numbers_setter_text_fails: Baseline, [Set(&["style.code-block.line-numbers=maybe"])] =>
        fails(TYPE_MISMATCH, "code-block.line-numbers", "style.code-block.line-numbers=maybe");
}

// Compound border (boolean | weight | object): union controls and the
// invalid shapes of each arm.
cells! {
    border_weight_control: Baseline, [File(json!({"table": {"border": "thick"}}))] =>
        Expect::Typed(|prepared| assert_eq!(table_border(prepared).weight, BorderWeight::Thick));
    border_bool_control: Baseline, [Set(&["style.table.border=true"])] =>
        Expect::Typed(|prepared| assert_eq!(table_border(prepared).weight, BorderWeight::Thin));
    border_object_control: Baseline, [File(json!({"table": {"border": {"weight": "medium", "color": "red-500"}}}))] =>
        Expect::Typed(|prepared| {
            let border = table_border(prepared);
            assert_eq!(border.weight, BorderWeight::Medium);
            assert!(border.color.is_some());
        });
    border_setter_null_leaf_disappears: Baseline,
        [Doc(json!({"table": {"border": {"weight": "thick", "color": "red-500"}}})), Set(&["style.table.border.color=null", "style.table.border.left=true"])] =>
        Expect::Typed(|prepared| {
            assert_eq!(prepared.value()["table"]["border"], json!({"weight": "thick", "color": "red-500", "left": true}));
            assert!(table_border(prepared).color.is_some());
        });
    border_empty_object_supplies_no_file_leaf: Baseline, [File(json!({"table": {"border": {}}}))] => fails(EMPTY_FILE, "", FILE);
    border_file_unknown_member_warns_and_its_sibling_applies: Baseline, [File(json!({"table": {"border": {"weight": "thick", "glow": 1}}}))] =>
        Expect::Typed(|prepared| {
            assert_eq!(table_border(prepared).weight, BorderWeight::Thick);
            assert!(prepared.value()["table"]["border"].get("glow").is_none());
            assert!(
                prepared.diagnostics().iter().any(|diagnostic| diagnostic.code == Some("dm.style.unknown_key") && diagnostic.warning.path.ends_with("glow")),
                "{:?}", prepared.diagnostics()
            );
        });
    border_shared_by_code_blocks_excludes_an_unknown_member: Baseline, [File(json!({"code-block": {"border": {"left": true, "glow": 1}}}))] =>
        Expect::Typed(|prepared| {
            assert_eq!(prepared.value()["code-block"]["border"], json!({"left": true}));
            assert!(prepared.diagnostics().iter().any(|diagnostic| diagnostic.warning.path == "style.code-block.border.glow"));
        });
    border_setter_unknown_member_fails: Baseline, [Set(&["style.table.border.glow=1"])] =>
        fails(UNKNOWN_KEY, "table.border.glow", "style.table.border.glow=1");
    border_unknown_weight_fails: Baseline, [File(json!({"table": {"border": "wavy"}}))] => fails(CONSTRAINT, "table.border", FILE);
    border_empty_string_fails: Baseline, [File(json!({"table": {"border": ""}}))] => fails(CONSTRAINT, "table.border", FILE);
    border_wrong_type_fails: Baseline, [File(json!({"table": {"border": 5}}))] => fails(CONSTRAINT, "table.border", FILE);
    border_array_fails: Baseline, [File(json!({"table": {"border": ["thin"]}}))] => fails(CONSTRAINT, "table.border", FILE);
    border_member_weight_fails: Baseline, [File(json!({"table": {"border": {"weight": "huge"}}}))] => fails(CONSTRAINT, "table.border.weight", FILE);
    border_member_side_boolean_word_converts: Baseline, [Set(&["style.table.border.left=yes"])] =>
        Expect::Typed(|prepared| assert_eq!(prepared.value()["table"]["border"], json!({"left": true})));
    border_member_side_text_fails: Baseline, [Set(&["style.table.border.left=maybe"])] =>
        fails(TYPE_MISMATCH, "table.border.left", "style.table.border.left=maybe");
    border_document_unknown_member_warns_and_its_sibling_applies: Baseline, [Doc(json!({"table": {"border": {"weight": "thick", "glow": 1}}}))] =>
        Expect::Typed(|prepared| {
            assert_eq!(table_border(prepared).weight, BorderWeight::Thick);
            let warnings: Vec<_> = prepared.diagnostics().iter().filter(|diagnostic| diagnostic.code.is_some()).collect();
            let [warning] = warnings.as_slice() else { panic!("one warning: {:?}", prepared.diagnostics()) };
            assert_eq!((warning.code, warning.warning.path.as_str()), (Some("dm.style.unknown_key"), "style.table.border.glow"));
            assert_eq!(warning.origin.kind, StyleSourceKind::Document);
        });
}

// A schema-admitted custom array: retained without a rendering effect.
cells! {
    array_control_is_retained: Custom(ARRAY_SCHEMA), [File(json!({"tags": [1, 2]}))] => Expect::Typed(|prepared| {
        assert_eq!(prepared.retained(), &json!({"tags": [1, 2]}));
        assert!(prepared.style().page.is_none(), "no rendering effect");
    });
    array_absent_preserves_the_lower_layer: Custom(ARRAY_SCHEMA), [Doc(json!({"tags": [1]})), File(json!({"foo": "x"}))] =>
        Expect::Typed(|prepared| assert_eq!(prepared.retained()["tags"], json!([1])));
    array_null_preserves_the_lower_layer: Custom(ARRAY_SCHEMA), [Doc(json!({"tags": [1]})), File(json!({"tags": null, "foo": "x"}))] =>
        Expect::Typed(|prepared| assert_eq!(prepared.retained()["tags"], json!([1])));
    array_empty_replaces_an_inherited_array: Custom(ARRAY_SCHEMA), [Doc(json!({"tags": [1]})), File(json!({"tags": [], "foo": "x"}))] =>
        Expect::Typed(|prepared| assert_eq!(prepared.retained()["tags"], json!([])));
    array_empty_setter_replaces_an_inherited_array: Custom(ARRAY_SCHEMA), [Doc(json!({"tags": [1]})), Set(&["style.tags=[]"])] =>
        Expect::Typed(|prepared| assert_eq!(prepared.retained()["tags"], json!([])));
    array_empty_alone_is_an_empty_file: Custom(ARRAY_SCHEMA), [File(json!({"tags": []}))] => fails(EMPTY_FILE, "", FILE);
    array_wrong_whole_type_fails: Custom(ARRAY_SCHEMA), [File(json!({"tags": 5}))] => fails(TYPE_MISMATCH, "tags", FILE);
    array_one_invalid_element_fails: Custom(ARRAY_SCHEMA), [File(json!({"tags": [1, "x"]}))] => fails(TYPE_MISMATCH, "tags.1", FILE);
    array_all_invalid_elements_fail: Custom(ARRAY_SCHEMA), [File(json!({"tags": ["x", "y"]}))] => fails(TYPE_MISMATCH, "tags.0", FILE);
    array_setter_invalid_element_fails: Custom(ARRAY_SCHEMA), [Set(&["style.tags=[1, 'x']"])] => fails(TYPE_MISMATCH, "tags.1", "style.tags=[1, 'x']");
    array_setter_control: Custom(ARRAY_SCHEMA), [Set(&["style.tags=[3, 4]"])] =>
        Expect::Typed(|prepared| assert_eq!(prepared.retained()["tags"], json!([3, 4])));
}

// Required members: a null is absence, checked on the merged style.
cells! {
    required_from_a_document_satisfies_a_sparse_file: Custom(REQUIRED_SCHEMA), [Doc(json!({"page": {"color": "red-500"}})), File(json!({"page": {"theme": "nord"}}))] =>
        Expect::Typed(|prepared| assert_eq!(page(prepared).theme, Some(ThemePair::Nord)));
    required_file_null_is_missing: Custom(REQUIRED_SCHEMA), [File(json!({"page": {"color": null, "theme": "nord"}}))] =>
        fails(MISSING_REQUIRED, "page.color", "");
    required_setter_null_is_missing: Custom(REQUIRED_SCHEMA), [Set(&["style.page.color=null", "style.page.theme=nord"])] =>
        fails(MISSING_REQUIRED, "page.color", "");
}

// Unknown properties: an invalid sibling of an excluded key still fails.
cells! {
    unknown_file_key_beside_an_invalid_sibling_still_fails: Baseline, [File(json!({"bogus": 1, "max-width": "abc"}))] =>
        fails(CONSTRAINT, "page.max-width", FILE);
    unknown_setter_beside_a_valid_sibling_fails: Baseline, [Set(&["style.margin=2", "style.bogus=1"])] =>
        fails(UNKNOWN_KEY, "bogus", "style.bogus=1");
    inline_container_setter_duplicate_keys_fail: Baseline, [Set(&["style={ margin: 1, margin: 2 }"])] =>
        fails(INVALID_SETTER, "", "style={ margin: 1, margin: 2 }");
}
