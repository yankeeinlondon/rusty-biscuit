use crate::common;

use common::layout::*;
use common::{CliProcessFixture, MdCommandBuilder, md_file};

fn rendering_command(fixture: &CliProcessFixture) -> assert_cmd::Command {
    rendering_builder(fixture).build()
}

/// The same pinned rendering frame before `build()`, for a test that also has
/// an application input to declare.
fn rendering_builder(fixture: &CliProcessFixture) -> MdCommandBuilder<'_> {
    fixture.command_builder().plain_terminal(80, 24)
}
use common::style::{COMPONENT_BUCKETS, every_component, style_object, styled, styled_page};
use darkmatter::layout::{DarkmatterPage, PageComponent};
use renderable::layout::{Alignment, Edges, Length, TargetValue};

/// The page `md` builds for `raw` and `args`' setters on an 80-column terminal.
fn apply_style_for(raw: &str, args: &[&str]) -> DarkmatterPage {
    styled_page(raw, args, 80)
}

fn cells(edge: &TargetValue<Length>) -> u16 {
    tv_cells(edge)
}

/// The padding of `component`'s policy as `[top, right, bottom, left]` cells.
fn padding_cells(page: &DarkmatterPage, component: PageComponent) -> [u16; 4] {
    let padding: Edges = page.component_policy(component).map(|policy| policy.layout.padding.clone()).unwrap_or_default();
    [cells(&padding.top), cells(&padding.right), cells(&padding.bottom), cells(&padding.left)]
}

/// The buckets each fill replacement can name. `hyperlinks` takes no padding
/// and showed no change from any fill mode, and a rule has no `width: auto`
/// (`full` showed no change on it), so neither is named where it cannot be.
/// The page components whose width policy a broadcast `width`/`max-width`
/// sets. A hyperlink showed no change from any fill mode, so it is not
/// asserted; a rule keeps its width in its own style, which
/// [`hr_width_keys_size_the_rule_as_the_old_fill_did`] checks on the drawn rule.
fn fill_targets() -> impl Iterator<Item = PageComponent> {
    COMPONENT_BUCKETS
        .into_iter()
        .map(|(component, _)| component)
        .filter(|component| !matches!(component, PageComponent::Hyperlinks | PageComponent::Hr))
}

const FULL_BUCKETS: [&str; 8] = ["images", "block-quote", "table", "code-block", "ul", "ol", "li", "disclosure"];
const PADDED_BUCKETS: [&str; 9] = ["images", "block-quote", "table", "code-block", "ul", "ol", "li", "hr", "disclosure"];

fn component_of(name: &str) -> PageComponent {
    COMPONENT_BUCKETS.iter().find_map(|(component, each)| (*each == name).then_some(*component)).expect("a bucket name")
}

#[test]
fn global_alignment_setter_reaches_every_bucket_and_leaves_fill() {
    let page = apply_style_for("# Doc\n", &["doc.md", "style.alignment=center"]);
    for (component, _) in COMPONENT_BUCKETS {
        assert_eq!(alignment_for(&page, component), Alignment::Center, "{component:?}");
        assert_eq!(fill_for(&page, component), TestFill::Full, "{component:?} fill must stay untouched");
    }
}

#[test]
fn broadcast_fill_object_reaches_every_bucket_and_leaves_alignment() {
    let page = apply_style_for("# Doc\n", &["doc.md", &every_component(r#"{ "max-width": 60 }"#)]);
    for component in fill_targets() {
        assert_eq!(fill_for(&page, component), TestFill::Max(Length::ch(60)), "{component:?}");
    }
    for (component, _) in COMPONENT_BUCKETS {
        assert_eq!(alignment_for(&page, component), Alignment::default(), "{component:?} alignment must stay untouched");
    }
}

#[test]
fn component_alignment_setter_reaches_one_bucket() {
    let page = apply_style_for("# Doc\n", &["doc.md", "style.table.alignment=right"]);
    assert_eq!(alignment_for(&page, PageComponent::Tables), Alignment::Right);
    for component in [PageComponent::Images, PageComponent::BlockQuotes, PageComponent::CodeBlocks] {
        assert_eq!(alignment_for(&page, component), Alignment::default(), "{component:?}");
    }
    for (component, _) in COMPONENT_BUCKETS {
        assert_eq!(fill_for(&page, component), TestFill::Full, "{component:?}");
    }
}

#[test]
fn component_fill_setter_reaches_one_bucket() {
    let page = apply_style_for("# Doc\n", &["doc.md", "style.images.max-width=40"]);
    assert_eq!(fill_for(&page, PageComponent::Images), TestFill::Max(Length::ch(40)));
    for component in [PageComponent::Tables, PageComponent::BlockQuotes, PageComponent::CodeBlocks] {
        assert_eq!(fill_for(&page, component), TestFill::Full, "{component:?}");
    }
    for (component, _) in COMPONENT_BUCKETS {
        assert_eq!(alignment_for(&page, component), Alignment::default(), "{component:?}");
    }
}

// ── The five old fill modes, broadcast through explicit bucket objects ──

/// A document whose every component inherits a cap and vertical padding, so
/// each replacement has something to clear.
fn inherited_fill_document(buckets: &[&str]) -> String {
    let mut doc = String::from("---\nstyle:\n");
    for bucket in buckets {
        doc.push_str(&format!("    {bucket}:\n        max-width: 50%\n"));
    }
    doc.push_str("---\n\n# Doc\n");
    doc
}

#[test]
fn fill_full_replacement_clears_an_inherited_cap_in_every_bucket() {
    // The old `--fill full`: `{ width: "auto", padding: 0 }` per bucket.
    let doc = inherited_fill_document(&FULL_BUCKETS);
    let page = apply_style_for(&doc, &["doc.md", &style_object(&FULL_BUCKETS, r#"{ width: "auto", padding: 0 }"#)]);
    for name in FULL_BUCKETS {
        let component = component_of(name);
        assert_eq!(fill_for(&page, component), TestFill::Full, "{name}");
    }
}

#[test]
fn fill_max_replacement_caps_every_bucket() {
    // The old `--fill max=40`: `max-width: 40` per bucket.
    let page = apply_style_for("# Doc\n", &["doc.md", &every_component(r#"{ "max-width": 40 }"#)]);
    for component in fill_targets() {
        assert_eq!(fill_for(&page, component), TestFill::Max(Length::ch(40)), "{component:?}");
    }
}

#[test]
fn fill_explicit_replacement_fixes_every_bucket_width() {
    // The old `--fill explicit=50`: `width: 50` per bucket. Setting `width`
    // drops an inherited `max-width`.
    let all: Vec<&str> = COMPONENT_BUCKETS.iter().map(|(_, name)| *name).collect();
    let doc = inherited_fill_document(&all);
    let page = apply_style_for(&doc, &["doc.md", &every_component(r#"{ width: 50 }"#)]);
    for component in fill_targets() {
        assert_eq!(fill_for(&page, component), TestFill::Explicit(Length::ch(50)), "{component:?}");
    }
}

/// A document whose padded buckets carry `padding: 2` on every side.
fn padded_document() -> String {
    let mut doc = String::from("---\nstyle:\n");
    for bucket in PADDED_BUCKETS {
        doc.push_str(&format!("    {bucket}:\n        padding: 2\n"));
    }
    doc.push_str("---\n\n# Doc\n");
    doc
}

#[test]
fn fill_pad_replacement_resets_the_whole_padding_in_every_bucket() {
    // The old `--fill pad=4`: the padding reset, then both horizontal sides.
    // The inherited vertical padding is gone, as the old fill replaced it.
    let setter = style_object(&PADDED_BUCKETS, r#"{ padding: 0, "padding-left": 4, "padding-right": 4 }"#);
    let page = apply_style_for(&padded_document(), &["doc.md", &setter]);
    for name in PADDED_BUCKETS {
        let component = component_of(name);
        assert_eq!(padding_cells(&page, component), [0, 4, 0, 4], "{name}: [top, right, bottom, left]");
        assert_eq!(fill_for(&page, component), TestFill::Pad(Length::ch(4)), "{name}");
    }
}

#[test]
fn fill_pad_sides_without_the_reset_keep_inherited_vertical_padding() {
    // Why the replacement resets first: the sides alone change only their edges.
    let setter = style_object(&["table"], r#"{ "padding-left": 4, "padding-right": 4 }"#);
    let page = apply_style_for(&padded_document(), &["doc.md", &setter]);
    assert_eq!(padding_cells(&page, PageComponent::Tables), [2, 4, 2, 4]);
}

#[test]
fn fill_indent_replacement_names_its_side_in_every_bucket() {
    // The old `--fill indent=4` chose the side from the alignment; the
    // replacement names it: left for a left-aligned bucket, right for a
    // right-aligned one, both for a centered one. Each resets the padding.
    let left = style_object(&PADDED_BUCKETS, r#"{ padding: 0, "padding-left": 4 }"#);
    let page = apply_style_for(&padded_document(), &["doc.md", &left]);
    for name in PADDED_BUCKETS {
        let component = component_of(name);
        assert_eq!(padding_cells(&page, component), [0, 0, 0, 4], "{name}: left indent");
        assert_eq!(fill_for(&page, component), TestFill::Indent(Length::ch(4)), "{name}");
    }

    let right = style_object(&PADDED_BUCKETS, r#"{ alignment: "right", padding: 0, "padding-right": 4 }"#);
    let page = apply_style_for(&padded_document(), &["doc.md", &right]);
    for name in PADDED_BUCKETS {
        assert_eq!(padding_cells(&page, component_of(name)), [0, 4, 0, 0], "{name}: right indent");
    }

    let center = style_object(&PADDED_BUCKETS, r#"{ alignment: "center", padding: 0, "padding-left": 4, "padding-right": 4 }"#);
    let page = apply_style_for(&padded_document(), &["doc.md", &center]);
    for name in PADDED_BUCKETS {
        assert_eq!(padding_cells(&page, component_of(name)), [0, 4, 0, 4], "{name}: centered indent");
    }
}

/// The rule line of `md`'s terminal rendering of a rule between two
/// paragraphs at 50 columns, under `args`: its leading blank cells and its
/// drawn width.
fn rule_placement(args: &[&str]) -> (usize, usize) {
    use darkmatter::testing::strip_ansi_codes;
    let (page, md) = styled("Above\n\n---\n\nBelow\n", args, 50);
    let plain = strip_ansi_codes(&page.render(&md).expect("terminal render"));
    let line = plain
        .lines()
        .find(|line| !line.trim().is_empty() && !line.contains("Above") && !line.contains("Below"))
        .unwrap_or_else(|| panic!("a rule line in:\n{plain}"))
        .trim_end();
    let indent = line.chars().take_while(|ch| *ch == ' ').count();
    (indent, line.chars().count() - indent)
}

#[test]
fn images_padding_moves_a_lone_image_as_the_old_fill_did() {
    // `--fill-images pad=10` and `indent=10` moved a lone image 10 cells in
    // and wrote its padding into the HTML; `images` takes the same padding
    // keys as every other bucket.
    let fixture = CliProcessFixture::named("images_padding_moves_a_lone_image_as_the_old_fill_did");
    let tmp = md_file("# Img\n\n![alt text here](./missing.png)\n");
    let run = |args: &[&str]| {
        let output = rendering_command(&fixture).arg(tmp.path()).args(["--width", "60"]).args(args).output().unwrap();
        assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };
    let indent = r#"style={ images: { padding: 0, "padding-left": 10 } }"#;
    let plain = run(&["--output", "plain", indent]);
    assert!(plain.lines().any(|line| line == "          IMAGE[alt text here]"), "{plain}");

    let pad = r#"style={ images: { padding: 0, "padding-left": 10, "padding-right": 10 } }"#;
    let html = run(&["--output", "html", pad]);
    assert!(html.contains("padding-left:10ch;padding-right:10ch"), "{html}");
}

#[test]
fn hr_width_keys_size_the_rule_as_the_old_fill_did() {
    // `--fill max=N` and `--fill explicit=N` reached a rule as `hr.max-width`
    // and `hr.width`, which the broadcast objects name.
    assert_eq!(rule_placement(&["doc.md", &every_component(r#"{ "max-width": 40 }"#)]).1, 40);
    assert_eq!(rule_placement(&["doc.md", &every_component(r#"{ width: 30 }"#)]).1, 30);
}

#[test]
fn hr_padding_moves_the_rule_as_the_old_fill_did() {
    let (base_indent, base_width) = rule_placement(&["doc.md"]);
    assert_eq!((base_indent, base_width), (0, 50), "the default rule spans the content width");
    // `--fill indent=4` moved the rule 4 cells in (46 wide); `--fill pad=4`
    // gave 4 cells in, 42 wide.
    assert_eq!(rule_placement(&["doc.md", "style.hr.padding-left=4"]), (4, 46));
    assert_eq!(
        rule_placement(&["doc.md", r#"style={ hr: { padding: 0, "padding-left": 4, "padding-right": 4 } }"#]),
        (4, 42)
    );
}

#[test]
fn frontmatter_table_alignment_reaches_page_when_no_cli_flag() {
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       alignment: left\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        alignment_for(&page, PageComponent::Tables),
        Alignment::Left,
        "frontmatter table.alignment must reach the page when no CLI claim",
    );
}

#[test]
fn table_alignment_setter_overrides_frontmatter_table_alignment() {
    // `style.table.alignment=right` over frontmatter `style.table.alignment:
    // left`: the setter wins that field, and the document's unrelated
    // `table.max-width` survives the merge.
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       alignment: left\n\
\x20       max-width: 50%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md", "style.table.alignment=right"]);
    assert_eq!(
        alignment_for(&page, PageComponent::Tables),
        Alignment::Right,
        "style.table.alignment=right must override frontmatter table.alignment: left",
    );
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::Percent(50.0)),
        "the document's table.max-width must survive a setter that names only alignment",
    );
}

#[test]
fn broadcast_fill_object_overrides_frontmatter_table_max_width() {
    // The old `--fill max=60` over frontmatter `style.table.max-width: 50%`:
    // the broadcast object names `table` with the rest, so it wins that field,
    // and the document's `table.alignment` survives.
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       alignment: left\n\
\x20       max-width: 50%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md", &every_component(r#"{ "max-width": 60 }"#)]);
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::ch(60)),
        "the broadcast object must win the table's max-width",
    );
    assert_eq!(alignment_for(&page, PageComponent::Tables), Alignment::Left);
}

#[test]
fn frontmatter_table_max_width_reaches_page_when_no_cli_flag() {
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       max-width: 50%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::Percent(50.0)),
        "frontmatter table.max-width must reach the page when no CLI claim",
    );
}

#[test]
fn frontmatter_images_alignment_and_fill_reach_page() {
    let raw = "---\n\
style:\n\
\x20   images:\n\
\x20       alignment: center\n\
\x20       max-width: 40ch\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        alignment_for(&page, PageComponent::Images),
        Alignment::Center,
    );
    assert_eq!(
        fill_for(&page, PageComponent::Images),
        TestFill::Max(Length::ch(40)),
    );
}

#[test]
fn frontmatter_block_quote_max_width_reaches_page() {
    let raw = "---\n\
style:\n\
\x20   block-quote:\n\
\x20       max-width: 75%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        fill_for(&page, PageComponent::BlockQuotes),
        TestFill::Max(Length::Percent(75.0)),
    );
}

#[test]
fn style_fixture_renders_with_align_tables_override() {
    let fixture = CliProcessFixture::named("style_fixture_renders_with_align_tables_override");
    // End-to-end sanity check: the canonical fixture renders successfully
    // when the user overrides table alignment from the CLI.
    let output = rendering_command(&fixture)
        .arg(style_prop_fixture())
        .arg("style.table.alignment=center")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md style-prop.md style.table.alignment=center must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_fixture_renders_html_with_fill_override() {
    let fixture = CliProcessFixture::named("style_fixture_renders_html_with_fill_override");
    // End-to-end sanity check: --output html runs the same component-style
    // path as the terminal pipeline.
    let output = rendering_builder(&fixture)
        .application_input("MD_DRY_RUN", "1")
        .build()
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .arg(every_component(r#"{ "max-width": 60 }"#))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html with a broadcast max-width object must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let html = String::from_utf8(output.stdout).expect("html stdout must be utf-8");
    assert!(
        html.contains("<table") && html.contains("max-width:60ch") && !html.contains("max-width:50%"),
        "the object's table max-width must replace the fixture's 50%. html:\n{html}",
    );
}

#[test]
fn style_frontmatter_html_emits_component_layout_css() {
    let fixture = CliProcessFixture::named("style_frontmatter_html_emits_component_layout_css");
    // Sub-spec #3 acceptance (review-3 finding #3): `md --output html` on a
    // document carrying `style.table.*`, `style.images.*`, and
    // `style.block-quote.*` must emit the matching component layout CSS
    // selectors and declarations, not just succeed silently.
    //
    // Mirrors `darkmatter/lib/src/layout/page.rs::browser_render_with_component_*_css`
    // but drives the assertion through the public CLI surface so the
    // frontmatter → page-style → HTML pipeline is exercised end-to-end.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   table:\n\
        \x20       alignment: center\n\
        \x20       max-width: 60ch\n\
        \x20   images:\n\
        \x20       alignment: right\n\
        \x20       max-width: 40ch\n\
        \x20   block-quote:\n\
        \x20       alignment: right\n\
        \x20       max-width: 50ch\n\
        ---\n\n\
        # Doc\n\n\
        | A | B |\n\
        | - | - |\n\
        | 1 | 2 |\n\n\
        ![alt](./x.png)\n\n\
        > quote\n",
    );

    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let html = String::from_utf8(output.stdout).expect("html stdout must be utf-8");

    // Component layout is now emitted as inline `style` attributes by the
    // renderable browser fold (build_component_css was deleted in the cutover).
    // Table: center alignment + max-width: 60ch → margin-left:auto;margin-right:auto.
    assert!(
        html.contains("<table")
            && html.contains("max-width:60ch")
            && html.contains("margin-left:auto")
            && html.contains("margin-right:auto"),
        "expected centered table with inline max-width and auto margins in HTML. html:\n{html}",
    );
    // Block-quote: right alignment + max-width: 50ch → margin-left:auto.
    assert!(
        html.contains("<blockquote")
            && html.contains("max-width:50ch")
            && html.contains("margin-left:auto"),
        "expected right-aligned blockquote with inline max-width and auto margin in HTML. html:\n{html}",
    );
    // Image: max-width and alignment are applied to the wrapping paragraph via
    // the lone-image layout path (alignment without max-width does not emit
    // margin styles in the current fold).
    assert!(
        html.contains("<img") && html.contains("src=\"./x.png\""),
        "expected image element in HTML. html:\n{html}",
    );
}

#[test]
fn style_prop_fixture_html_emits_table_layout_css() {
    let fixture = CliProcessFixture::named("style_prop_fixture_html_emits_table_layout_css");
    // Acceptance from sub-spec #3: `md --output html style-prop.md` must emit
    // the expected table layout CSS (right alignment + 50% max-width that
    // lowers to the page-content base, i.e. 50ch when the page builds at
    // its 120-col default for HTML).
    let output = rendering_command(&fixture)
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let html = String::from_utf8(output.stdout).expect("html stdout must be utf-8");

    // Component layout is now emitted as inline `style` attributes by the
    // renderable browser fold (build_component_css was deleted in the cutover).
    // Right alignment + max-width: 50% → margin-left:auto on the table element.
    assert!(
        html.contains("<table") && html.contains("margin-left:auto"),
        "expected right-aligned table with inline auto margin in HTML. html:\n{html}",
    );
    // The fixture sets `max-width: 50%`; the fold preserves the percent on HTML.
    assert!(
        html.contains("max-width:50%"),
        "expected `max-width:50%` declaration in HTML. html:\n{html}",
    );
}

// =============================================================================
//          PHASE 5 REGRESSION TESTS (Sub-Spec #3)
// =============================================================================
//
// These tests cover the Phase 5 acceptance criteria:
//
//   1. Setters on a document without a `style:` block resolve to the same page
//      as the same values written in the document's own `style:` (no silent
//      state drift between the two layers).
//   2. The canonical `style-prop.md` fixture resolves to the structural
//      page state the spec promises (table right-aligned, capped at 50%).
//   3. Component fill from `style.block-quote.max-width` flows into the
//      page builder and matches the structural shape used by the terminal
//      blockquote renderer.

#[test]
fn setters_on_a_document_without_style_match_the_same_frontmatter() {
    let raw = "# No Style Doc\n\nBody.\n";
    let broadcast = every_component(r#"{ "max-width": 50 }"#);
    let from_setters = styled_page(raw, &["doc.md", "style.margin=3", "style.max-width=70", "style.alignment=center", &broadcast], 120);

    let mut frontmatter = String::from("---\nstyle:\n    margin: 3\n    max-width: 70\n    alignment: center\n");
    for (_, name) in COMPONENT_BUCKETS {
        frontmatter.push_str(&format!("    {name}:\n        max-width: 50\n"));
    }
    frontmatter.push_str("---\n");
    let from_document = styled_page(&format!("{frontmatter}{raw}"), &["doc.md"], 120);

    assert_eq!(from_setters.page_margin(), from_document.page_margin(), "margins");
    assert_eq!(tv_cells(&from_setters.page_margin().left), 3);
    assert_eq!(from_setters.page_padding(), from_document.page_padding(), "padding");
    assert_eq!(from_setters.max_width(), Some(70));
    assert_eq!(from_setters.max_width(), from_document.max_width(), "max-width");
    for component in PageComponent::ALL {
        assert_eq!(alignment_for(&from_setters, component), Alignment::Center, "{component:?}");
        assert_eq!(alignment_for(&from_setters, component), alignment_for(&from_document, component), "{component:?}");
        assert_eq!(fill_for(&from_setters, component), fill_for(&from_document, component), "{component:?}");
    }
    for component in fill_targets() {
        assert_eq!(fill_for(&from_setters, component), TestFill::Max(Length::ch(50)), "{component:?}");
    }
}

#[test]
fn style_prop_fixture_resolves_to_expected_table_layout() {
    // Phase 5 acceptance: the canonical `style-prop.md` fixture must produce
    // a page where the table is right-aligned and capped at 50% max-width via
    // the new component-style apply path.
    let raw = std::fs::read_to_string(style_prop_fixture()).unwrap();
    let page = apply_style_for(&raw, &["doc.md"]);

    assert_eq!(
        alignment_for(&page, PageComponent::Tables),
        Alignment::Right,
        "fixture must resolve to a right-aligned table",
    );
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::Percent(50.0)),
        "fixture must cap the table at 50% max-width",
    );
}

#[test]
fn style_prop_fixture_resolves_to_expected_page_margins() {
    // Phase 5 acceptance: page-level margins from the fixture survive the
    // full CLI -> page-style -> component-style pipeline.
    let raw = std::fs::read_to_string(style_prop_fixture()).unwrap();
    let page = apply_style_for(&raw, &["doc.md"]);

    let m = page.page_margin();
    assert_eq!(
        tv_cells(&m.left),
        2,
        "fixture margin-left: 2ch must reach the page"
    );
    assert_eq!(
        tv_cells(&m.right),
        4,
        "fixture margin-right: 4ch must reach the page"
    );
    assert_eq!(
        tv_cells(&m.top),
        1,
        "fixture margin-top: 1 must reach the page"
    );
    assert_eq!(
        tv_cells(&m.bottom),
        0,
        "fixture margin-bottom: 0 must reach the page"
    );
}

#[test]
fn block_quote_max_width_caps_terminal_render_wrap_width() {
    // Phase 5 acceptance: `style.block-quote.max-width` reaches the page and
    // caps visible wrap width when the terminal renders a top-level
    // blockquote. We use a 100-col terminal so 50% resolves cleanly, then
    // assert that no rendered (ANSI-stripped) blockquote line exceeds the
    // resolved fill width.
    use darkmatter::testing::strip_ansi_codes;

    let raw = "---\n\
style:\n\
\x20   block-quote:\n\
\x20       max-width: 50%\n\
---\n\n\
> This is a long quoted paragraph intended to wrap onto multiple visible \
rows once the blockquote fill caps the render width well below the page width. \
Add filler text to guarantee the wrap point is reached even when the \
terminal is reasonably wide.\n";

    let (page, md) = styled(raw, &["doc.md"], 100);

    // Structural guard: the apply pipeline put the fill where the renderer
    // will look for it.
    assert_eq!(
        fill_for(&page, PageComponent::BlockQuotes),
        TestFill::Max(Length::Percent(50.0)),
        "block-quote.max-width must reach the page fill slot",
    );

    let rendered = page.render(&md).expect("render to terminal");
    let plain = strip_ansi_codes(&rendered);

    // The blockquote should wrap onto at least two visible lines.
    let quote_lines: Vec<String> = plain
        .lines()
        .filter(|l| {
            let trimmed = l.trim_start();
            // Common blockquote indicators across themes (`│`, `▌`, `▐`).
            trimmed.starts_with('│') || trimmed.starts_with('▌') || trimmed.starts_with('▐')
        })
        .map(|l| l.trim_end().to_string())
        .collect();

    assert!(
        quote_lines.len() >= 2,
        "blockquote should wrap onto >=2 visible lines under max-width: 50%. plain:\n{plain}",
    );

    // The renderer chooses 50% of the available content width; with a 100-col
    // terminal and no other margins/padding that's at most 50 cells of *text*
    // beyond the indicator + leading space. Allow generous slack for indent +
    // alignment padding by upper-bounding total visible width at 60.
    let max_len = quote_lines.iter().map(|l| l.chars().count()).max().unwrap();
    assert!(
        max_len <= 60,
        "blockquote visible width should be capped under max-width: 50% on a 100-col terminal, got max={max_len}. plain:\n{plain}",
    );
}
