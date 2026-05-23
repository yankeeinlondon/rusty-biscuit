//! Integration: parse the user's example fixture
//! (`darkmatter/example-docs/rendering/style-prop.md`) and assert every
//! field in the spec's acceptance criteria.

use std::fs;
use std::path::PathBuf;

use darkmatter::markdown::Markdown;
use darkmatter::style::warning::StyleWarningKind;
use darkmatter::style::{StyleFrontmatter, from_frontmatter, into_strict};
use renderable::layout::{Alignment, Length};

/// Locate the fixture relative to `CARGO_MANIFEST_DIR` so the test is
/// independent of where it's invoked from.
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("example-docs")
        .join("rendering")
        .join("style-prop.md")
}

#[test]
fn fixture_parses_to_expected_style_frontmatter() {
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");

    let (style, warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    // Acceptance criteria: page.
    let page = style.page.as_ref().expect("page bucket");
    assert_eq!(page.left_margin, Some(Length::Ch(2)));
    assert_eq!(page.right_margin, Some(Length::Ch(4)));
    assert_eq!(page.top_margin, Some(1));
    assert_eq!(page.bottom_margin, Some(0));

    // Acceptance criteria: table.
    let table = style.table.as_ref().expect("table bucket");
    assert_eq!(table.common.alignment, Some(Alignment::Right));
    assert_eq!(table.common.max_width, Some(Length::Percent(50.0)));

    // Acceptance criteria: ol.
    let ol = style.ol.as_ref().expect("ol bucket");
    assert_eq!(ol.common.alignment, Some(Alignment::Right));

    // Acceptance criteria: ul.
    let ul = style.ul.as_ref().expect("ul bucket");
    assert_eq!(ul.common.alignment, Some(Alignment::Left));
    assert_eq!(ul.left_margin, Some(Length::Ch(4)));
    assert_eq!(ul.common.max_width, Some(Length::Ch(40)));

    // Other buckets must remain None.
    assert!(style.hyperlinks.is_none());
    assert!(style.images.is_none());
    assert!(style.hr.is_none());
    assert!(style.li.is_none());
    assert!(style.block_quote.is_none());

    // All warnings must be KnownButInactive (the fixture is schema-clean).
    let schema_issues: Vec<_> = warnings
        .iter()
        .filter(|w| w.is_schema_issue())
        .collect();
    assert!(
        schema_issues.is_empty(),
        "fixture should be schema-clean, got: {:?}",
        schema_issues
    );

    let inactive_count = warnings
        .iter()
        .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
        .count();
    assert!(inactive_count > 0, "expected KnownButInactive warnings");
}

#[test]
fn fixture_passes_strict_validation() {
    // Strict mode succeeds because every warning is KnownButInactive.
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    let md = Markdown::try_from_content(&raw).expect("parse markdown");
    let parsed = from_frontmatter(md.frontmatter()).expect("parse style");
    let _: StyleFrontmatter = into_strict(parsed).expect("strict should pass");
}
