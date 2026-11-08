//! Public preparation of sparse style layers (`darkmatter::style::layer`):
//! normalization, precedence, schema admission versus runtime support, and
//! the per-source handling of unknown keys and empty files.
//!
//! Every schema is built from an in-memory document in a temporary
//! directory, so nothing here reads the repository or the process directory.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::ThemePair;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use darkmatter::style::layer::{
    STYLE_CONFLICT_CODE, STYLE_EMPTY_FILE_CODE, STYLE_INVALID_ROOT_CODE, STYLE_INVALID_SETTER_CODE,
    STYLE_INVALID_VALUE_CODE,
};
use darkmatter::style::schema::hr::HrAlignment;
use darkmatter::style::warning::StyleWarningKind;
use darkmatter::style::{
    PreparedStyle, StyleLayer, StylePreparationError, StylePreparationPolicy, StyleSetter, StyleSourceKind,
    from_json_value, prepare_style_layers,
};
use darkmatter::style::error::StyleParseError;
use renderable::layout::{Alignment, Length};
use renderable::style::BorderWeight;
use serde_json::{Value, json};
use simplified_schema::PropertyPath;

const UNKNOWN_KEY: &str = "dm.schema.unknown_key";
const TYPE_MISMATCH: &str = "dm.schema.type_mismatch";
const MISSING_REQUIRED: &str = "dm.schema.missing_required";
const STYLE_UNKNOWN_KEY: &str = "dm.style.unknown_key";

/// A temporary directory anchoring every context of one test.
struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("temp dir"),
        }
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::new(self.dir.path().to_path_buf())
    }

    /// A context anchored at the subdirectory `name`.
    fn context_in(&self, name: &str) -> FileResolutionContext {
        let dir = self.dir.path().join(name);
        std::fs::create_dir_all(&dir).expect("subdirectory");
        FileResolutionContext::new(dir)
    }

    /// The effective schema of a document holding `frontmatter`, over the
    /// compiled baseline.
    fn schema(&self, frontmatter: &str) -> EffectiveSchema {
        let markdown: Markdown = format!("---\n{frontmatter}---\nBody\n").as_str().into();
        DarkmatterSchemas::new(self.context())
            .with_darkmatter_baseline_json_schema()
            .expect("the compiled baseline loads")
            .effective_for(&markdown)
            .expect("the schema resolves")
            .expect("the baseline applies")
    }

    /// The baseline alone.
    fn baseline(&self) -> EffectiveSchema {
        self.schema("title: Notes\n")
    }

    fn document(&self, value: Value) -> StyleLayer {
        StyleLayer::document(value, "doc.md", self.context(), None)
    }

    fn file(&self, value: Value) -> StyleLayer {
        StyleLayer::file(value, "style.yaml", self.context(), None)
    }

    fn setters(&self, arguments: &[&str]) -> StyleLayer {
        let setters = arguments
            .iter()
            .map(|argument| {
                let (key, raw) = argument.split_once('=').expect("a setter has `=`");
                StyleSetter::new(*argument, PropertyPath::parse(key).expect("a setter path"), raw)
            })
            .collect();
        StyleLayer::setters(setters, self.context())
    }
}

fn prepare(layers: &[StyleLayer], schema: &EffectiveSchema) -> Result<PreparedStyle, StylePreparationError> {
    prepare_style_layers(layers, Some(schema), StylePreparationPolicy::render())
}

fn prepared(layers: &[StyleLayer], schema: &EffectiveSchema) -> PreparedStyle {
    prepare(layers, schema).unwrap_or_else(|error| panic!("preparation failed: {:#?}", error.problems))
}

fn failure(layers: &[StyleLayer], schema: &EffectiveSchema) -> StylePreparationError {
    match prepare(layers, schema) {
        Ok(style) => panic!("preparation succeeded: {:#?}", style.value()),
        Err(error) => error,
    }
}

fn codes(error: &StylePreparationError) -> Vec<&'static str> {
    error.problems.iter().map(|problem| problem.code).collect()
}

/// `(margin-left, margin-right, margin-top, margin-bottom)`.
fn margins(style: &PreparedStyle) -> (Option<Length>, Option<Length>, Option<u16>, Option<u16>) {
    let page = style.style().page.clone().unwrap_or_default();
    (page.margin_left, page.margin_right, page.margin_top, page.margin_bottom)
}

mod normalization {
    use super::*;

    #[test]
    fn a_specific_side_beats_all_sides_in_either_key_order() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        for value in [json!({"margin": 2, "ml": 6}), json!({"ml": 6, "margin": 2})] {
            let style = prepared(&[fixture.file(value)], &schema);
            assert_eq!(margins(&style), (Some(Length::Ch(6)), Some(Length::Ch(2)), Some(2), Some(2)));
        }
    }

    #[test]
    fn setter_shorthands_resolve_by_specificity_within_their_layer() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        for arguments in [&["style.ml=6", "style.margin=2"][..], &["style={ margin: 2 }", "style.ml=6"]] {
            let style = prepared(&[fixture.setters(arguments)], &schema);
            assert_eq!(margins(&style), (Some(Length::Ch(6)), Some(Length::Ch(2)), Some(2), Some(2)), "{arguments:?}");
        }
    }

    #[test]
    fn a_later_broad_layer_beats_an_earlier_specific_one() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let document = fixture.document(json!({"ml": 6}));
        let style = prepared(&[document.clone(), fixture.setters(&["style.margin=2"])], &schema);
        assert_eq!(margins(&style), (Some(Length::Ch(2)), Some(Length::Ch(2)), Some(2), Some(2)));

        let style = prepared(&[fixture.file(json!({"page": {"ml": 6}})), fixture.file(json!({"margin": 2}))], &schema);
        assert_eq!(margins(&style).0, Some(Length::Ch(2)));
    }

    #[test]
    fn two_spellings_of_one_field_in_one_layer_fail() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let error = failure(&[fixture.setters(&["style.ml=2", "style.page.margin-left=3"])], &schema);
        assert_eq!(codes(&error), [STYLE_CONFLICT_CODE]);
        assert_eq!(error.problems[0].path, "page.margin-left");

        for value in [
            json!({"max-width": 50, "page": {"max-width": 60}}),
            json!({"ml": 1, "page": {"ml": 2}}),
            json!({"code": {"theme": "nord"}, "page": {"code": {"theme": "github"}}}),
        ] {
            let error = failure(&[fixture.file(value.clone())], &schema);
            assert_eq!(codes(&error), [STYLE_CONFLICT_CODE], "{value}");
        }
        // A broad spelling beside a more specific one is not a conflict.
        prepared(&[fixture.file(json!({"margin": 1, "page": {"margin-left": 3}}))], &schema);
    }

    #[test]
    fn repeating_a_setter_path_keeps_its_last_value_unread_until_then() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let style = prepared(&[fixture.setters(&["style.margin=abc", "style.margin=3"])], &schema);
        assert_eq!(margins(&style).2, Some(3));

        let style = prepared(&[fixture.setters(&["style={ margin: 2 }", "style={ max-width: 50 }"])], &schema);
        let page = style.style().page.clone().expect("page");
        assert_eq!((page.margin_top, page.max_width), (None, Some(Length::Ch(50))));
    }

    #[test]
    fn the_document_parser_honors_top_level_shorthands() {
        let (style, warnings) = from_json_value(&json!({"margin": 2, "ml": 6})).expect("parses");
        assert!(warnings.iter().all(|warning| !warning.is_schema_issue()), "{warnings:?}");
        let page = style.page.expect("page");
        assert_eq!((page.margin_left, page.margin_right, page.margin_top), (Some(Length::Ch(6)), Some(Length::Ch(2)), Some(2)));

        match from_json_value(&json!({"ml": 1, "page": {"margin-left": 2}})) {
            Err(StyleParseError::Conflict { path, .. }) => assert_eq!(path, "style.page.margin-left"),
            other => panic!("expected a conflict, got {other:?}"),
        }
    }
}

mod alignment {
    use super::*;

    #[test]
    fn alignment_reaches_the_page_and_every_component() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let style = prepared(&[fixture.setters(&["style.alignment=center"])], &schema).into_style();
        let center = Some(Alignment::Center);
        assert_eq!(style.page.expect("page").alignment, center);
        assert_eq!(style.images.expect("images").common.alignment, center);
        assert_eq!(style.block_quote.expect("block-quote").common.alignment, center);
        assert_eq!(style.table.expect("table").common.alignment, center);
        assert_eq!(style.code_block.expect("code-block").common.alignment, center);
        assert_eq!(style.ul.expect("ul").common.alignment, center);
        assert_eq!(style.ol.expect("ol").common.alignment, center);
        assert_eq!(style.li.expect("li").common.alignment, center);
        assert_eq!(style.hyperlinks.expect("hyperlinks").common.alignment, center);
        assert_eq!(style.hr.expect("hr").alignment, Some(HrAlignment::Center));
        assert_eq!(style.disclosure.expect("disclosure").common.alignment, center);
    }

    #[test]
    fn a_component_alignment_in_the_same_layer_wins() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let style = prepared(&[fixture.setters(&["style.alignment=center", "style.table.alignment=right"])], &schema).into_style();
        assert_eq!(style.table.expect("table").common.alignment, Some(Alignment::Right));
        assert_eq!(style.ul.expect("ul").common.alignment, Some(Alignment::Center));

        // The document keeps its table left; a later source's broad
        // alignment overrides it.
        let document = fixture.document(json!({"page": {"alignment": "center"}, "table": {"alignment": "left"}}));
        let style = prepared(std::slice::from_ref(&document), &schema).into_style();
        assert_eq!(style.table.expect("table").common.alignment, Some(Alignment::Left));
        let style = prepared(&[document, fixture.setters(&["style.alignment=right"])], &schema).into_style();
        assert_eq!(style.table.expect("table").common.alignment, Some(Alignment::Right));
    }

    #[test]
    fn a_rule_receives_a_side_from_a_broadcast_and_keeps_a_direct_full() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let style = prepared(&[fixture.file(json!({"alignment": "right", "hr": {"alignment": "full"}}))], &schema).into_style();
        assert_eq!(style.hr.expect("hr").alignment, Some(HrAlignment::Full));
        let style = prepared(&[fixture.setters(&["style.alignment=left"])], &schema).into_style();
        assert_eq!(style.hr.expect("hr").alignment, Some(HrAlignment::Left));
    }
}

mod merging {
    use super::*;

    #[test]
    fn a_nested_border_setter_selects_the_object_form_after_a_scalar() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let style = prepared(&[fixture.setters(&["style.table.border=thin", "style.table.border.left=false"])], &schema);
        assert_eq!(style.value()["table"]["border"], json!({"left": false}));

        let style = prepared(
            &[fixture.document(json!({"table": {"border": "thin"}})), fixture.file(json!({"table": {"border": {"left": true}}}))],
            &schema,
        );
        assert_eq!(style.value()["table"]["border"], json!({"left": true}));
        let style = prepared(
            &[fixture.document(json!({"table": {"border": {"color": "red-500"}}})), fixture.setters(&["style.table.border=thick"])],
            &schema,
        );
        assert_eq!(style.value()["table"]["border"], json!("thick"));
    }

    #[test]
    fn a_sparse_border_keeps_an_earlier_weight_and_color() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let document = fixture.document(json!({"table": {"border": {"weight": "thick", "color": "red-500"}}}));
        let style = prepared(&[document, fixture.setters(&["style.table.border.left=false"])], &schema);
        assert_eq!(style.value()["table"]["border"], json!({"weight": "thick", "color": "red-500", "left": false}));
        let table = style.into_style().table.expect("table");
        let border = table.common.border.expect("border");
        assert_eq!(border.as_border().weight, BorderWeight::Thick);
        assert!(border.as_border().color.is_some(), "the document's color survives");
    }

    #[test]
    fn null_is_absence_while_zero_and_false_are_values() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let document = fixture.document(json!({"page": {"margin-left": 4, "margin-top": 3}, "code-block": {"line-numbers": true}}));
        let file = fixture.file(json!({"page": {"margin-left": null, "margin-top": 0}, "code-block": {"line-numbers": false}}));
        let style = prepared(&[document, file, fixture.setters(&["style.page.margin-left=null"])], &schema);
        assert_eq!(margins(&style).0, Some(Length::Ch(4)));
        assert_eq!(margins(&style).2, Some(0));
        assert_eq!(style.style().code_block.as_ref().expect("code-block").line_numbers, Some(false));
    }

    #[test]
    fn width_and_max_width_conflict_in_a_layer_and_replace_across_layers() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let error = failure(&[fixture.file(json!({"table": {"width": 50, "max-width": 40}}))], &schema);
        assert_eq!(codes(&error), [STYLE_CONFLICT_CODE]);

        let document = fixture.document(json!({"table": {"max-width": 40}}));
        let style = prepared(&[document, fixture.setters(&["style.table.width=50"])], &schema);
        let table = style.into_style().table.expect("table");
        assert!(table.common.width.is_some());
        assert_eq!(table.common.max_width, None);
    }

    #[test]
    fn a_later_component_padding_replaces_inherited_sides() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let document = fixture.document(json!({"table": {"padding-left": 4}}));
        let style = prepared(&[document, fixture.setters(&["style.table.padding=0"])], &schema);
        let table = style.into_style().table.expect("table");
        assert!(table.common.padding.is_some());
        assert_eq!(table.common.padding_left, None);
    }

    #[test]
    fn a_winning_stylesheet_keeps_its_own_file_context() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let (first, second) = (fixture.context_in("a"), fixture.context_in("b"));
        let layers = [
            fixture.document(json!({"page": {"color": "slate-100"}})),
            StyleLayer::file(json!({"page": {"stylesheet": "./theme.css"}}), "a/style.yaml", first.clone(), None),
            StyleLayer::file(json!({"page": {"bg-color": "slate-900"}}), "b/style.yaml", second, None),
        ];
        let style = prepared(&layers, &schema);
        let origin = style.origin("page.stylesheet").expect("a stylesheet origin");
        assert_eq!((origin.kind, origin.source.as_str()), (StyleSourceKind::File, "a/style.yaml"));
        assert_eq!(origin.context, first);
        assert_eq!(style.origin("page.color").expect("color origin").kind, StyleSourceKind::Document);
    }
}

mod admission {
    use super::*;

    #[test]
    fn required_style_fields_may_come_from_two_layers() {
        let fixture = Fixture::new();
        let schema = fixture.schema(
            "$schema:\n    title: string(required)\n    style:\n        page:\n            color: string(required)\n            theme: string(required)\n",
        );
        let file = fixture.file(json!({"page": {"color": "red-500"}}));
        // The document's missing `title` does not block a render.
        let style = prepared(&[file.clone(), fixture.setters(&["style.page.theme=github"])], &schema);
        assert_eq!(style.style().page.as_ref().expect("page").theme, Some(ThemePair::Github));

        let error = failure(&[file], &schema);
        assert_eq!(codes(&error), [MISSING_REQUIRED]);
        assert_eq!(error.problems[0].path, "page.theme");
    }

    #[test]
    fn an_admitted_custom_key_is_retained_without_a_warning_or_effect() {
        let fixture = Fixture::new();
        let schema = fixture.schema("$schema:\n    style:\n        foo: string\n        \"<string>\": unknown\n");
        let style = prepared(&[fixture.file(json!({"foo": "bar", "margin": 2})), fixture.setters(&["style.mx=3"])], &schema);
        assert!(style.diagnostics().iter().all(|diagnostic| diagnostic.code.is_none()), "{:?}", style.diagnostics());
        assert_eq!(style.retained(), &json!({"foo": "bar"}));
        assert_eq!(margins(&style), (Some(Length::Ch(3)), Some(Length::Ch(3)), Some(2), Some(2)));
    }

    #[test]
    fn a_document_style_declaration_replaces_the_built_in_one() {
        let fixture = Fixture::new();
        let schema = fixture.schema("$schema:\n    style:\n        foo: string\n");
        let error = failure(&[fixture.setters(&["style.margin=4"])], &schema);
        assert_eq!(codes(&error), [UNKNOWN_KEY]);
        let problem = &error.problems[0];
        assert_eq!(problem.path, "page.margin");
        assert_eq!(problem.origin.as_ref().expect("an origin").source, "style.margin=4");
    }

    #[test]
    fn a_widened_known_field_still_meets_its_runtime_contract() {
        let fixture = Fixture::new();
        let schema = fixture.schema("$schema:\n    style:\n        page:\n            max-width: string\n");
        let error = failure(&[fixture.setters(&["style.page.max-width=abc"])], &schema);
        assert_eq!(codes(&error), [STYLE_INVALID_VALUE_CODE]);
        assert_eq!(error.problems[0].path, "page.max-width");
    }

    #[test]
    fn setter_unknown_keys_and_wrong_types_fail_with_schema_codes() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let error = failure(&[fixture.setters(&["style.magin=4", "style.margin=abc"])], &schema);
        assert_eq!(codes(&error), [UNKNOWN_KEY, TYPE_MISMATCH]);
        let (unknown, mismatch) = (&error.problems[0], &error.problems[1]);
        assert_eq!(unknown.origin.as_ref().expect("origin").source, "style.magin=4");
        assert_eq!(unknown.origin.as_ref().expect("origin").span, None, "a setter has no document position");
        assert_eq!((mismatch.path.as_str(), mismatch.origin.as_ref().expect("origin").source.as_str()), ("page.margin", "style.margin=abc"));
    }

    #[test]
    fn an_open_object_takes_only_a_whole_value() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let error = failure(&[fixture.setters(&["style.page.meta.author=Ken"])], &schema);
        assert_eq!(codes(&error), [STYLE_INVALID_SETTER_CODE]);
        let style = prepared(&[fixture.setters(&["style.page.meta={ author: Ken }"])], &schema);
        assert_eq!(style.value()["page"]["meta"], json!({"author": "Ken"}));
    }

    #[test]
    fn a_cli_root_must_be_a_mapping() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        assert_eq!(codes(&failure(&[fixture.file(json!(5))], &schema)), [STYLE_INVALID_ROOT_CODE]);
        assert_eq!(codes(&failure(&[fixture.file(Value::Null)], &schema)), [STYLE_INVALID_ROOT_CODE]);
        // A setter's root is an ordinary schema value, named by its key.
        let setter = failure(&[fixture.setters(&["style=5"])], &schema);
        assert_eq!(codes(&setter), [TYPE_MISMATCH]);
        let message = setter.problems[0].to_string();
        assert!(message.contains("`style` takes a mapping"), "{message}");
        assert!(message.contains("from the setter `style=5`"), "{message}");
    }

    #[test]
    fn without_a_schema_the_descriptor_decides() {
        let fixture = Fixture::new();
        let style = prepare_style_layers(&[fixture.setters(&["style.margin=2"])], None, StylePreparationPolicy::render())
            .expect("prepares");
        assert_eq!(margins(&style).2, Some(2));
        let error = prepare_style_layers(&[fixture.setters(&["style.magin=2"])], None, StylePreparationPolicy::render())
            .expect_err("an unknown setter fails");
        assert_eq!(codes(&error), [UNKNOWN_KEY]);
    }
}

mod sources {
    use super::*;

    #[test]
    fn document_unknown_keys_warn_for_render_and_fail_for_compose() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let document = fixture.document(json!({"bogus": 1, "page": {"max_width": 50}}));
        let style = prepared(std::slice::from_ref(&document), &schema);
        let codes_seen: Vec<Option<&str>> = style.diagnostics().iter().map(|diagnostic| diagnostic.code).collect();
        assert!(codes_seen.contains(&Some(STYLE_UNKNOWN_KEY)), "{codes_seen:?}");
        assert!(codes_seen.contains(&Some("dm.style.deprecated")), "{codes_seen:?}");
        assert_eq!(style.style().page.as_ref().expect("page").max_width, Some(Length::Ch(50)));

        let error = prepare_style_layers(std::slice::from_ref(&document), Some(&schema), StylePreparationPolicy::compose())
            .expect_err("compose rejects an unknown key");
        assert_eq!(codes(&error), [STYLE_UNKNOWN_KEY]);
    }

    #[test]
    fn an_admitted_document_key_does_not_warn() {
        let fixture = Fixture::new();
        let schema = fixture.schema("$schema:\n    style:\n        \"<string>\": unknown\n");
        let style = prepared(&[fixture.document(json!({"bogus": 1}))], &schema);
        assert!(style.diagnostics().is_empty(), "{:?}", style.diagnostics());
        assert_eq!(style.retained(), &json!({"bogus": 1}));
    }

    #[test]
    fn a_file_unknown_key_is_excluded_with_a_warning_and_its_sibling_applies() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let file = fixture.file(json!({"bad-mojo": true, "max-width": 50}));
        let style = prepared(std::slice::from_ref(&file), &schema);
        assert_eq!(style.style().page.as_ref().expect("page").max_width, Some(Length::Ch(50)));
        let [warning] = style.diagnostics() else {
            panic!("one warning expected: {:?}", style.diagnostics());
        };
        assert_eq!((warning.code, warning.warning.kind.clone()), (Some(STYLE_UNKNOWN_KEY), StyleWarningKind::UnknownKey));
        assert_eq!((warning.origin.source.as_str(), warning.warning.path.as_str()), ("style.yaml", "style.bad-mojo"));

        let denied = prepare_style_layers(&[file], Some(&schema), StylePreparationPolicy::render().with_deny_style(true))
            .expect_err("--deny style makes it fatal");
        assert_eq!(codes(&denied), [STYLE_UNKNOWN_KEY]);
    }

    #[test]
    fn a_file_must_supply_a_non_null_admitted_value() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        for value in [json!({}), json!({"page": {}}), json!({"page": {"color": null}}), json!({"table": {"bad": 1}})] {
            assert_eq!(codes(&failure(&[fixture.file(value.clone())], &schema)), [STYLE_EMPTY_FILE_CODE], "{value}");
        }
        // A cleanup rule and a target-inactive key count; neither renders.
        let style = prepared(&[fixture.file(json!({"markdown": {"unwrap": true}}))], &schema);
        assert_eq!(style.retained(), &json!({"markdown": {"unwrap": true}}));
        prepared(&[fixture.file(json!({"page": {"theme": "github"}}))], &schema);
        // An empty string is a value, checked by its field.
        assert_eq!(codes(&failure(&[fixture.file(json!({"page": {"color": ""}}))], &schema)), [STYLE_INVALID_VALUE_CODE]);
        // Inline `style={}` is a valid no-op.
        let style = prepared(&[fixture.setters(&["style={}"])], &schema);
        assert_eq!(style.value(), &json!({}));
    }

    #[test]
    fn an_admitted_custom_file_value_counts_as_supplied() {
        let fixture = Fixture::new();
        let schema = fixture.schema("$schema:\n    style:\n        foo: string\n");
        let style = prepared(&[fixture.file(json!({"foo": "bar"}))], &schema);
        assert_eq!(style.retained(), &json!({"foo": "bar"}));
    }

    #[test]
    fn every_file_is_checked_even_when_a_later_one_replaces_it() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let error = failure(&[fixture.file(json!({"max-width": "abc"})), fixture.file(json!({"max-width": 50}))], &schema);
        assert_eq!(error.problems.len(), 1, "{:?}", error.problems);
        assert_eq!(error.problems[0].path, "page.max-width");
    }
}

/// Typo suggestions name only keys the layer's effective schema admits; with
/// no schema, the descriptor's.
mod suggestions {
    use super::*;

    /// `style: { accent: string }`, replacing the built-in style definition.
    const ACCENT_SCHEMA: &str = "$schema:\n    style:\n        accent: string\n";

    /// The one problem message of a failing setter.
    fn setter_message(fixture: &Fixture, schema: Option<&EffectiveSchema>, argument: &str) -> String {
        let error = prepare_style_layers(&[fixture.setters(&[argument])], schema, StylePreparationPolicy::render())
            .expect_err("the setter fails");
        let [problem] = error.problems.as_slice() else {
            panic!("one problem expected: {:#?}", error.problems);
        };
        assert_eq!(problem.code, UNKNOWN_KEY, "{problem:?}");
        problem.message.clone()
    }

    /// The suggestion of a file's or document's one unknown-key warning.
    fn warning_suggestion(layer: StyleLayer, schema: &EffectiveSchema) -> Option<String> {
        let style = prepared(&[layer], schema);
        let [warning] = style.diagnostics() else {
            panic!("one warning expected: {:?}", style.diagnostics());
        };
        assert_eq!(warning.code, Some(STYLE_UNKNOWN_KEY), "{warning:?}");
        warning.warning.suggestion.clone()
    }

    #[test]
    fn a_custom_schema_setter_typo_suggests_the_custom_key() {
        let fixture = Fixture::new();
        let schema = fixture.schema(ACCENT_SCHEMA);
        let style = prepared(&[fixture.setters(&["style.accent=blue"])], &schema);
        assert_eq!(style.retained(), &json!({"accent": "blue"}));
        let message = setter_message(&fixture, Some(&schema), "style.acent=blue");
        assert!(message.ends_with("; did you mean `style.accent`?"), "{message}");
    }

    #[test]
    fn a_custom_schema_setter_typo_never_suggests_a_rejected_built_in_key() {
        let fixture = Fixture::new();
        let schema = fixture.schema(ACCENT_SCHEMA);
        let message = setter_message(&fixture, Some(&schema), "style.magin=4");
        assert!(!message.contains("did you mean"), "{message}");
        // A rename hint names a built-in key this schema rejects as well.
        let message = setter_message(&fixture, Some(&schema), "style.left-margin=4");
        assert!(!message.contains("did you mean"), "{message}");
    }

    #[test]
    fn a_nested_custom_object_setter_typo_suggests_its_sibling() {
        let fixture = Fixture::new();
        let schema = fixture.schema("$schema:\n    style:\n        box:\n            accent: string\n");
        let message = setter_message(&fixture, Some(&schema), "style.box.acent=blue");
        assert!(message.ends_with("; did you mean `style.box.accent`?"), "{message}");
    }

    #[test]
    fn a_pattern_opened_style_admits_a_typo_without_a_suggestion_error() {
        let fixture = Fixture::new();
        let schema = fixture.schema("$schema:\n    style:\n        \"<string>\": unknown\n");
        let style = prepared(&[fixture.setters(&["style.acent=blue"])], &schema);
        assert_eq!(style.retained(), &json!({"acent": "blue"}));
    }

    #[test]
    fn a_baseline_setter_typo_suggests_the_built_in_sibling() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let message = setter_message(&fixture, Some(&schema), "style.magin=4");
        assert!(message.ends_with("; did you mean `style.margin`?"), "{message}");
        let message = setter_message(&fixture, Some(&schema), "style.table.alignmnt=x");
        assert!(message.ends_with("; did you mean `style.table.alignment`?"), "{message}");
        let message = setter_message(&fixture, Some(&schema), "style.page.left-margin=4");
        assert!(message.ends_with("; did you mean `style.page.margin-left`?"), "{message}");
    }

    #[test]
    fn a_schemaless_setter_typo_suggests_the_descriptor_sibling() {
        let fixture = Fixture::new();
        let message = setter_message(&fixture, None, "style.magin=4");
        assert!(message.ends_with("; did you mean `style.margin`?"), "{message}");
        let message = setter_message(&fixture, None, "style.table.alignmnt=x");
        assert!(message.ends_with("; did you mean `style.table.alignment`?"), "{message}");
    }

    #[test]
    fn file_and_document_rename_suggestions_are_unchanged_under_the_baseline() {
        let fixture = Fixture::new();
        let schema = fixture.baseline();
        let file = fixture.file(json!({"page": {"left-margin": 2, "max-width": 50}}));
        assert_eq!(warning_suggestion(file, &schema).as_deref(), Some("page.margin-left"));
        let document = fixture.document(json!({"page": {"left-margin": 2}}));
        assert_eq!(warning_suggestion(document, &schema).as_deref(), Some("page.margin-left"));
        // A file or document typo keeps the rename-only rule: no nearest key.
        let file = fixture.file(json!({"magin": 2, "max-width": 50}));
        assert_eq!(warning_suggestion(file, &schema), None);
    }

    #[test]
    fn a_file_rename_suggestion_never_names_a_key_the_custom_schema_rejects() {
        let fixture = Fixture::new();
        let schema = fixture.schema(ACCENT_SCHEMA);
        let file = fixture.file(json!({"left-margin": 2, "accent": "blue"}));
        assert_eq!(warning_suggestion(file, &schema), None);
        let document = fixture.document(json!({"left-margin": 2}));
        assert_eq!(warning_suggestion(document, &schema), None);
    }
}
