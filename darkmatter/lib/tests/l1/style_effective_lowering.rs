//! Lowering an effective style, prepared from its layers, onto a page
//! (`darkmatter::style::apply_prepared_style`): horizontal padding sides and
//! rule padding, presentation defaults (themes, code-block mode, line
//! numbers), and the context each `page.stylesheet` resolves in.
//!
//! Every page is built at an explicit width with a pinned color depth from
//! `Terminal::new_optimistic`, which reports a dark surface; nothing detects
//! the host. Every context is anchored in a temporary directory and none
//! discovers a repository.

use std::collections::HashMap;
use std::path::Path;

use biscuit_file::FileResolutionContext;
use biscuit_terminal::output::OutputTarget;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use darkmatter::layout::{DarkmatterPage, PageComponent};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{CodeBlockMode, ThemePair, get_code_theme_for_prose};
use darkmatter::markdown::output::terminal::ColorDepth;
use darkmatter::style::schema::CommonStyle;
use darkmatter::style::bespoke::merge_common_style;
use darkmatter::style::{
    PageStylesheet, PreparedStyle, PresentationDefaults, StyleApplyError, StyleAssetResolver, StyleLayer,
    StylePreparationPolicy, StyleSetter, StyleSourceKind, apply_prepared_style, prepare_style_layers,
};
use renderable::layout::{Edges, Length, TargetValue};
use serde_json::{Value, json};
use simplified_schema::PropertyPath;

const WIDTH: usize = 50;

fn context(dir: &Path) -> FileResolutionContext {
    FileResolutionContext::new(dir.to_path_buf())
}

fn document(value: Value, dir: &Path) -> StyleLayer {
    StyleLayer::document(value, "doc.md", context(dir), None)
}

fn file(value: Value, argument: &str, dir: &Path) -> StyleLayer {
    StyleLayer::file(value, argument, context(dir), None)
}

fn setters(arguments: &[&str], dir: &Path) -> StyleLayer {
    let setters = arguments
        .iter()
        .map(|argument| {
            let (key, raw) = argument.split_once('=').expect("a setter has `=`");
            StyleSetter::new(*argument, PropertyPath::parse(key).expect("a setter path"), raw)
        })
        .collect();
    StyleLayer::setters(setters, context(dir))
}

fn prepare(layers: &[StyleLayer]) -> PreparedStyle {
    prepare_style_layers(layers, None, StylePreparationPolicy::render())
        .unwrap_or_else(|error| panic!("preparation failed: {:#?}", error.problems))
}

fn page() -> DarkmatterPage {
    DarkmatterPage::new(&Terminal::new_optimistic(WIDTH as u32)).with_color_depth(ColorDepth::TrueColor)
}

fn apply_with(
    prepared: &PreparedStyle,
    target: OutputTarget,
    defaults: &PresentationDefaults,
) -> Result<DarkmatterPage, StyleApplyError> {
    apply_prepared_style(page(), prepared, StyleAssetResolver::for_target(target), defaults)
}

fn apply(prepared: &PreparedStyle, target: OutputTarget) -> DarkmatterPage {
    apply_with(prepared, target, &PresentationDefaults::default()).expect("the style applies")
}

fn ch(n: u32) -> TargetValue<Length> {
    TargetValue::universal(Length::ch(n))
}

fn padding_of(page: &DarkmatterPage, component: PageComponent) -> Edges {
    page.component_policy(component).map(|policy| policy.layout.padding.clone()).unwrap_or_default()
}

fn edges(top: u32, right: u32, bottom: u32, left: u32) -> Edges {
    Edges { top: ch(top), right: ch(right), bottom: ch(bottom), left: ch(left) }
}

mod padding {
    use super::*;

    const TABLE: &str = "Intro\n\n| a | b |\n|---|---|\n| 1 | 2 |\n";

    #[test]
    fn side_padding_lowers_after_padding_into_the_component_policy() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(&[document(
            json!({ "table": { "padding": 1, "padding-left": 4 }, "ul": { "padding-right": "3ch" } }),
            dir.path(),
        )]);
        let page = apply(&prepared, OutputTarget::Terminal);

        assert_eq!(padding_of(&page, PageComponent::Tables), edges(1, 1, 1, 4));
        let ul = padding_of(&page, PageComponent::Ul);
        assert_eq!(ul.right, ch(3));
        assert_eq!((ul.top, ul.bottom, ul.left), (Edges::default().top, Edges::default().bottom, Edges::default().left));
    }

    #[test]
    fn terminal_and_browser_render_the_lowered_sides() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(&[document(json!({ "table": { "padding": 0, "padding-left": 4 } }), dir.path())]);
        let md = Markdown::from(TABLE);

        let terminal = apply(&prepared, OutputTarget::Terminal);
        let rendered = strip_escape_codes(terminal.render(&md).expect("terminal render"));
        let table_rows: Vec<&str> = rendered.lines().filter(|line| line.contains('│') || line.contains('┌')).collect();
        assert!(!table_rows.is_empty(), "{rendered}");
        assert!(table_rows.iter().all(|row| row.starts_with("    ") && !row.starts_with("     ")), "{rendered}");

        let browser = apply(&prepared, OutputTarget::Html);
        let html = browser.render_to_browser(&md).expect("browser render");
        let table = html.split("<table").nth(1).expect("a table element");
        assert!(table.contains("padding-left:4ch") && table.contains("padding-top:0"), "{table}");
    }

    #[test]
    fn padding_then_side_in_a_later_layer_resets_inherited_vertical_padding() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(&[
            document(json!({ "table": { "padding": 2 } }), dir.path()),
            file(json!({ "table": { "padding": 0, "padding-left": 4 } }), "fill.yaml", dir.path()),
        ]);
        assert_eq!(padding_of(&apply(&prepared, OutputTarget::Terminal), PageComponent::Tables), edges(0, 0, 0, 4));
    }

    #[test]
    fn a_side_only_override_keeps_inherited_vertical_padding() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(&[
            document(json!({ "table": { "padding": 2 } }), dir.path()),
            file(json!({ "table": { "padding-left": 4 } }), "indent.yaml", dir.path()),
        ]);
        assert_eq!(padding_of(&apply(&prepared, OutputTarget::Terminal), PageComponent::Tables), edges(2, 2, 2, 4));
    }

    /// The rule line of a terminal render: the one made of rule glyphs.
    fn rule_line(rendered: &str) -> String {
        rendered
            .lines()
            .find(|line| line.chars().any(|c| !c.is_whitespace()) && line.trim().chars().all(|c| c == '╌'))
            .unwrap_or_else(|| panic!("no rule line in {rendered:?}"))
            .to_string()
    }

    #[test]
    fn hr_padding_left_moves_the_rule_four_cells_in() {
        let dir = tempfile::tempdir().unwrap();
        let md = Markdown::from("Above\n\n---\n\nBelow\n");

        let plain = rule_line(&strip_escape_codes(apply(&prepare(&[]), OutputTarget::Terminal).render(&md).unwrap()));
        assert_eq!(plain.chars().count(), WIDTH);
        assert!(!plain.starts_with(' '), "{plain:?}");

        let indented = prepare(&[setters(&["style.hr.padding-left=4"], dir.path())]);
        let line = rule_line(&strip_escape_codes(apply(&indented, OutputTarget::Terminal).render(&md).unwrap()));
        assert_eq!(line.len() - line.trim_start().len(), 4, "{line:?}");
        assert_eq!(line.trim().chars().count(), WIDTH - 4, "{line:?}");

        let padded = prepare(&[document(json!({ "hr": { "padding": 0, "padding-left": 4, "padding-right": 4 } }), dir.path())]);
        let line = rule_line(&strip_escape_codes(apply(&padded, OutputTarget::Terminal).render(&md).unwrap()));
        assert_eq!(line.trim().chars().count(), 42, "`--fill pad=4` measured 42 wide at 50: {line:?}");
    }

    #[test]
    fn css_overlay_writes_padding_then_sides_as_longhands() {
        let style: CommonStyle = serde_json::from_value(json!({ "padding": 1, "padding-left": 4 })).unwrap();
        let css = style.to_css_overlay().expect("padding is a declaration").to_css();
        for declaration in ["padding-top: 1ch", "padding-right: 1ch", "padding-bottom: 1ch", "padding-left: 4ch"] {
            assert!(css.contains(declaration), "{declaration} in {css}");
        }

        let side_only: CommonStyle = serde_json::from_value(json!({ "padding-right": "2ch" })).unwrap();
        let css = side_only.to_css_overlay().expect("a side is a declaration").to_css();
        assert!(css.contains("padding-right: 2ch") && !css.contains("padding-left"), "{css}");
    }

    #[test]
    fn local_style_merge_applies_sides_over_padding() {
        let common = |value: Value| -> CommonStyle { serde_json::from_value(value).unwrap() };

        // A local `padding` replaces the base's sides.
        let merged = merge_common_style(&common(json!({ "padding-left": 4 })), &common(json!({ "padding": 0 })));
        assert_eq!(merged.padding_left, None);
        assert!(merged.padding.is_some());

        // A local side keeps the base's `padding` for the other edges.
        let merged = merge_common_style(&common(json!({ "padding": 2 })), &common(json!({ "padding-left": 4 })));
        let css = merged.to_css_overlay().unwrap().to_css();
        assert!(css.contains("padding-top: 2ch") && css.contains("padding-left: 4ch"), "{css}");
    }

    #[test]
    fn hyperlink_and_image_local_styles_keep_their_sides() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(&[document(
            json!({
                "hyperlinks": { "local-style": { "padding-left": 4 } },
                "images": { "local-style": { "padding": 1, "padding-right": 3 } },
            }),
            dir.path(),
        )]);
        let page = apply(&prepared, OutputTarget::Html);

        let link = page.local_hyperlink_style().expect("a local hyperlink style");
        assert_eq!(link.padding_left, Some(Length::Ch(4)));
        let image_css = page.local_image_style().and_then(CommonStyle::to_css_overlay).expect("image css").to_css();
        assert!(image_css.contains("padding-top: 1ch") && image_css.contains("padding-right: 3ch"), "{image_css}");
    }
}

mod presentation {
    use super::*;

    const CODE: &str = "```rust\nfn main() {}\n```\n";

    fn env(pairs: &[(&str, &str)]) -> PresentationDefaults {
        let env: HashMap<String, String> = pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
        PresentationDefaults::from_env(&env)
    }

    fn themes(page: &DarkmatterPage) -> (ThemePair, ThemePair) {
        let options = page.terminal_options();
        (options.prose_theme, page.page_code_theme().copied().unwrap_or(options.code_theme))
    }

    fn applied(style: Value, defaults: &PresentationDefaults) -> DarkmatterPage {
        let dir = tempfile::tempdir().unwrap();
        apply_with(&prepare(&[document(style, dir.path())]), OutputTarget::Terminal, defaults).expect("the style applies")
    }

    #[test]
    fn absent_values_resolve_to_one_half_and_inverse() {
        let page = applied(json!({}), &PresentationDefaults::default());
        assert_eq!(themes(&page), (ThemePair::OneHalf, get_code_theme_for_prose(ThemePair::OneHalf)));
        assert_eq!(page.code_block_mode(), CodeBlockMode::Inverse);
        assert!(!page.line_numbers());
    }

    #[test]
    fn present_environment_values_read_like_from_str_or_default() {
        let page = applied(json!({}), &env(&[("THEME", "dracula"), ("CODE_THEME", "not-a-theme")]));
        assert_eq!(themes(&page), (ThemePair::Dracula, ThemePair::OneHalf));
    }

    #[test]
    fn style_themes_beat_environment_defaults() {
        let defaults = env(&[("THEME", "github"), ("CODE_THEME", "monokai")]);
        let page = applied(json!({ "page": { "theme": "nord", "code": { "theme": "gruvbox" } } }), &defaults);
        assert_eq!(themes(&page), (ThemePair::Nord, ThemePair::Gruvbox));
    }

    #[test]
    fn an_absent_code_default_derives_from_the_effective_prose_theme() {
        let page = applied(json!({ "page": { "theme": "solarized" } }), &env(&[("THEME", "github")]));
        assert_eq!(themes(&page), (ThemePair::Solarized, get_code_theme_for_prose(ThemePair::Solarized)));
    }

    #[test]
    fn line_numbers_false_reaches_the_page_and_both_folds() {
        let md = Markdown::from(CODE);
        let dir = tempfile::tempdir().unwrap();
        let off = prepare(&[document(json!({ "code-block": { "line-numbers": false } }), dir.path())]);
        let on = prepare(&[setters(&["style.code-block.line-numbers=true"], dir.path())]);

        // `false` overrides a page that started with line numbers on.
        let page = apply_prepared_style(
            page().with_line_numbers(true),
            &off,
            StyleAssetResolver::for_target(OutputTarget::Html),
            &PresentationDefaults::default(),
        )
        .unwrap();
        assert!(!page.line_numbers());
        assert!(!page.render_to_browser(&md).unwrap().contains("ln-gutter"));
        assert!(!strip_escape_codes(page.render(&md).unwrap()).contains("1 │"));

        let page = apply(&on, OutputTarget::Html);
        assert!(page.line_numbers());
        assert!(page.render_to_browser(&md).unwrap().contains("ln-gutter"));
        assert!(strip_escape_codes(page.render(&md).unwrap()).contains("1 │"));
    }

    #[test]
    fn code_mode_reaches_the_terminal_and_browser_folds() {
        let md = Markdown::from(CODE);
        let dir = tempfile::tempdir().unwrap();
        let mode = |name: &str| apply(&prepare(&[document(json!({ "page": { "code": { "mode": name } } }), dir.path())]), OutputTarget::Html);
        let (dark, light) = (mode("dark"), mode("light"));

        assert_eq!((dark.code_block_mode(), light.code_block_mode()), (CodeBlockMode::Dark, CodeBlockMode::Light));
        assert_ne!(dark.render(&md).unwrap(), light.render(&md).unwrap());
        assert_ne!(dark.render_to_browser(&md).unwrap(), light.render_to_browser(&md).unwrap());
        // On the dark surface `Terminal::new_optimistic` reports, the default
        // inverse mode is the light variant.
        let inverse = apply(&prepare(&[]), OutputTarget::Html);
        assert_eq!(inverse.render(&md).unwrap(), light.render(&md).unwrap());
    }
}

mod stylesheet_origins {
    use super::*;

    const SHEET: &str = "page.stylesheet";

    /// A temporary directory holding `theme.css` whose rule names `marker`.
    fn css_dir(marker: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("theme.css"), format!(".{marker} {{ color: red; }}\n")).unwrap();
        dir
    }

    fn inline_css(page: &DarkmatterPage) -> &str {
        match page.stylesheet() {
            Some(PageStylesheet::Inline { css, .. }) => css,
            other => panic!("expected an inline stylesheet, got {other:?}"),
        }
    }

    #[test]
    fn a_document_value_resolves_in_the_document_context() {
        let (doc_dir, launch_dir) = (css_dir("from-document"), css_dir("from-launch"));
        let prepared = prepare(&[
            document(json!({ "page": { "stylesheet": "theme.css" } }), doc_dir.path()),
            setters(&["style.table.alignment=right"], launch_dir.path()),
        ]);
        assert_eq!(prepared.origin(SHEET).map(|origin| origin.kind), Some(StyleSourceKind::Document));
        assert!(inline_css(&apply(&prepared, OutputTarget::Html)).contains(".from-document"));
    }

    #[test]
    fn a_setter_value_resolves_in_the_launch_context() {
        let (doc_dir, launch_dir) = (css_dir("from-document"), css_dir("from-launch"));
        let prepared = prepare(&[
            document(json!({ "page": { "stylesheet": "theme.css" } }), doc_dir.path()),
            setters(&["style.page.stylesheet=./theme.css"], launch_dir.path()),
        ]);
        assert_eq!(prepared.origin(SHEET).map(|origin| origin.kind), Some(StyleSourceKind::Setters));
        assert!(inline_css(&apply(&prepared, OutputTarget::Html)).contains(".from-launch"));
    }

    #[test]
    fn a_file_value_resolves_in_its_own_file_context() {
        let (doc_dir, file_dir) = (css_dir("from-document"), css_dir("from-file"));
        let prepared = prepare(&[
            document(json!({ "page": { "stylesheet": "theme.css" } }), doc_dir.path()),
            file(json!({ "page": { "stylesheet": "theme.css" } }), "shared/style.yaml", file_dir.path()),
        ]);
        assert_eq!(prepared.origin(SHEET).map(|origin| origin.kind), Some(StyleSourceKind::File));
        assert!(inline_css(&apply(&prepared, OutputTarget::Html)).contains(".from-file"));
    }

    #[test]
    fn a_later_unrelated_file_keeps_the_earlier_files_context() {
        let first = css_dir("from-first-file");
        // The later file's directory has no `theme.css`: resolving there fails.
        let later = tempfile::tempdir().unwrap();
        let prepared = prepare(&[
            file(json!({ "page": { "stylesheet": "theme.css" } }), "first.yaml", first.path()),
            file(json!({ "table": { "alignment": "right" } }), "later.yaml", later.path()),
        ]);
        assert_eq!(prepared.origin(SHEET).map(|origin| origin.source.as_str()), Some("first.yaml"));
        assert!(inline_css(&apply(&prepared, OutputTarget::Html)).contains(".from-first-file"));
    }

    #[test]
    fn terminal_plain_and_source_targets_never_read_local_css() {
        let dir = tempfile::tempdir().unwrap();
        // Missing: any read would fail with `StylesheetNotFound`.
        let prepared = prepare(&[document(json!({ "page": { "stylesheet": "missing.css" } }), dir.path())]);
        for target in [OutputTarget::Terminal, OutputTarget::Plain, OutputTarget::Markdown, OutputTarget::Json] {
            let page = apply(&prepared, target);
            assert!(page.stylesheet().is_none(), "{target:?}");
        }
    }

    #[test]
    fn html_and_markdown_plus_read_local_css_into_the_page() {
        let dir = css_dir("read-for-browser");
        let prepared = prepare(&[document(json!({ "page": { "stylesheet": "theme.css" } }), dir.path())]);
        let md = Markdown::from("# Title\n");
        for target in [OutputTarget::Html, OutputTarget::MarkdownPlus] {
            let page = apply(&prepared, target);
            assert!(inline_css(&page).contains(".read-for-browser"), "{target:?}");
            assert!(page.render_to_browser_document(&md).unwrap().contains(".read-for-browser"), "{target:?}");
        }
    }

    #[test]
    fn an_http_stylesheet_stays_a_link() {
        let dir = tempfile::tempdir().unwrap();
        let href = "https://styles.invalid/app.css";
        let prepared = prepare(&[document(json!({ "page": { "stylesheet": href } }), dir.path())]);
        let page = apply(&prepared, OutputTarget::Html);
        assert_eq!(page.stylesheet(), Some(&PageStylesheet::Remote { href: href.to_string() }));
        let html = page.render_to_browser_document(&Markdown::from("# Title\n")).unwrap();
        assert!(html.contains(&format!(r#"<link rel="stylesheet" href="{href}""#)), "{html}");
    }

    #[test]
    fn a_file_url_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(&[document(json!({ "page": { "stylesheet": "file:///etc/hosts" } }), dir.path())]);
        for target in [OutputTarget::Html, OutputTarget::Terminal] {
            let error = apply_with(&prepared, target, &PresentationDefaults::default()).unwrap_err();
            assert_eq!(error, StyleApplyError::UnsupportedStylesheetScheme { scheme: "file://".into() }, "{target:?}");
        }
    }

    #[test]
    fn missing_local_css_is_a_typed_error() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(&[document(json!({ "page": { "stylesheet": "./missing.css" } }), dir.path())]);
        let error = apply_with(&prepared, OutputTarget::Html, &PresentationDefaults::default()).unwrap_err();
        assert!(matches!(&error, StyleApplyError::StylesheetNotFound { path } if path.ends_with("missing.css")), "{error:?}");
    }

    #[test]
    fn an_unresolvable_reference_carries_its_file_reference_error() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(&[document(json!({ "page": { "stylesheet": "{{STYLE_ROOT_UNSET}}/a.css" } }), dir.path())]);
        let error = apply_with(&prepared, OutputTarget::Html, &PresentationDefaults::default()).unwrap_err();
        assert!(matches!(error, StyleApplyError::StylesheetReference { .. }), "{error:?}");
        assert!(
            matches!(error.file_reference_error(), Some(biscuit_file::FileReferenceError::MissingEnvironmentVariable { .. })),
            "{error:?}"
        );
    }
}
