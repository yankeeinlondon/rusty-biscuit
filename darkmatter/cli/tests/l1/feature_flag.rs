//! `md --feature <NAME>`: the repository's `--feature` convention on `md`
//! (repeatable, comma-separated, case-insensitive; an unknown or empty name is
//! a usage error) and its lowering onto the HTML page.

use crate::common;

use clap::error::ErrorKind;
use clap::{Parser, ValueEnum};
use common::{CliProcessFixture, md_file};
use darkmatter::markdown::Markdown;
use darkmatter_cli::args::{Command, Feature};
use darkmatter_cli::artifact::html_page;
use darkmatter_cli::Cli;
use std::ffi::OsString;

const MERMAID_DOC: &str = "# Diagram\n\n```mermaid\nflowchart LR\n  A --> B\n```\n";

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("md").chain(args.iter().copied()))
}

fn features(args: &[&str]) -> Vec<Feature> {
    parse(args).expect("valid --feature arguments").render.features
}

/// Every name `md --feature` accepts, in declaration order.
const FEATURE_NAMES: [&str; 5] = [
    "embed-mermaid-font",
    "no-mermaid-fallbacks",
    "no-mermaid-partial-fallbacks",
    "no-mermaid-partial-warning",
    "no-mermaid-rendering",
];

#[test]
fn feature_names_are_the_declared_set() {
    let names: Vec<String> = Feature::value_variants()
        .iter()
        .map(|feature| feature.to_possible_value().unwrap().get_name().to_string())
        .collect();
    assert_eq!(names, FEATURE_NAMES);
}

#[test]
fn mermaid_feature_names_parse_comma_separated_repeated_and_in_any_case() {
    let comma = features(&[
        "--feature",
        "no-mermaid-fallbacks,NO-MERMAID-PARTIAL-WARNING",
        "--feature",
        "No-Mermaid-Partial-Fallbacks,no-mermaid-rendering",
        "doc.md",
    ]);
    let repeated = features(&[
        "--feature",
        "no-mermaid-fallbacks",
        "--feature",
        "no-mermaid-partial-warning",
        "--feature",
        "no-mermaid-partial-fallbacks",
        "--feature",
        "no-mermaid-rendering",
        "doc.md",
    ]);

    assert_eq!(comma, repeated);
    assert_eq!(
        comma,
        [
            Feature::NoMermaidFallbacks,
            Feature::NoMermaidPartialWarning,
            Feature::NoMermaidPartialFallbacks,
            Feature::NoMermaidRendering,
        ]
    );
}

#[test]
fn comma_separated_and_repeated_forms_are_equivalent() {
    let comma = features(&["--feature", "embed-mermaid-font,embed-mermaid-font", "doc.md"]);
    let repeated = features(&[
        "--feature",
        "embed-mermaid-font",
        "--feature",
        "embed-mermaid-font",
        "doc.md",
    ]);

    assert_eq!(comma, repeated);
    assert_eq!(comma, [Feature::EmbedMermaidFont, Feature::EmbedMermaidFont]);
}

#[test]
fn names_are_case_insensitive() {
    assert_eq!(
        features(&["--feature", "EMBED-Mermaid-Font", "doc.md"]),
        [Feature::EmbedMermaidFont]
    );
}

#[test]
fn feature_is_absent_by_default() {
    assert!(features(&["doc.md"]).is_empty());
}

#[test]
fn render_subcommand_accepts_feature() {
    let cli = parse(&["render", "doc.md", "--feature", "embed-mermaid-font"]).unwrap();
    let Some(Command::Render(options)) = cli.command else {
        panic!("expected the render subcommand");
    };
    assert_eq!(options.features, [Feature::EmbedMermaidFont]);

    // Before the subcommand it lands on the top level, where it is a usage
    // error rather than being combined with `render`'s list.
    let cli = parse(&["--feature", "embed-mermaid-font", "render", "doc.md"]).unwrap();
    assert_eq!(cli.render.features, [Feature::EmbedMermaidFont]);
    let error = darkmatter_cli::commands::validate_subcommand_usage(&cli)
        .expect_err("a root --feature before `render`")
        .downcast::<clap::Error>()
        .expect("a usage error");
    assert_eq!(error.exit_code(), 2);
}

#[test]
fn root_feature_before_the_render_subcommand_exits_with_usage_error() {
    let fixture = CliProcessFixture::named("root_feature_before_the_render_subcommand_exits_with_usage_error");
    let doc = md_file(MERMAID_DOC);
    let output = fixture
        .command()
        .args(["--feature", "embed-mermaid-font", "render"])
        .arg(doc.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("subcommands cannot be combined") && stderr.contains("--feature"), "{stderr}");
}

#[test]
fn unknown_name_is_a_usage_error_listing_valid_names() {
    let error = parse(&["--feature", "embed-mermaid-font,nope", "doc.md"])
        .err()
        .expect("unknown feature name must be rejected");

    assert_eq!(error.kind(), ErrorKind::InvalidValue);
    assert_eq!(error.exit_code(), 2);
    let message = error.to_string();
    assert!(message.contains("'nope'"), "{message}");
    for name in FEATURE_NAMES {
        assert!(message.contains(name), "{name}: {message}");
    }
}

#[test]
fn empty_name_is_a_usage_error_listing_valid_names() {
    for value in ["embed-mermaid-font,,embed-mermaid-font", "", ",embed-mermaid-font"] {
        let error = parse(&["--feature", value, "doc.md"])
            .err()
            .unwrap_or_else(|| panic!("empty feature name in {value:?} must be rejected"));

        assert_eq!(error.exit_code(), 2, "{value:?}");
        let message = error.to_string();
        for name in FEATURE_NAMES {
            assert!(message.contains(name), "{value:?} {name}: {message}");
        }
    }
}

#[test]
fn completion_offers_feature_names() {
    for args in [
        &["md", "--feature", ""][..],
        &["md", "render", "doc.md", "--feature", ""][..],
    ] {
        let mut command = <Cli as clap::CommandFactory>::command();
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        let index = args.len() - 1;
        let candidates = clap_complete::engine::complete(&mut command, args, index, None).unwrap();
        let values: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.get_value().to_string_lossy().into_owned())
            .collect();

        assert_eq!(values, FEATURE_NAMES);
    }
}

/// Page inputs with no document file: the stylesheet context is a scratch
/// directory and no theme variable is set.
fn page_inputs() -> darkmatter_cli::render::PageInputs {
    darkmatter_cli::render::PageInputs::new(
        biscuit_file::FileResolutionContext::new(std::env::temp_dir()),
        darkmatter::style::PresentationDefaults::default(),
    )
}

#[test]
fn embed_mermaid_font_feature_sets_the_html_page_option() {
    let md = Markdown::try_from_content(MERMAID_DOC).unwrap();
    let with = parse(&["--feature", "embed-mermaid-font", "doc.md"]).unwrap();
    let without = parse(&["doc.md"]).unwrap();

    assert!(html_page(&md, &with, &page_inputs(), false, &with.render.features).unwrap().embed_mermaid_font());
    assert!(!html_page(&md, &without, &page_inputs(), false, &without.render.features).unwrap().embed_mermaid_font());
}

#[test]
fn no_frontmatter_key_turns_on_embedding() {
    let doc = format!(
        "---\nembed_mermaid_font: true\nembed-mermaid-font: true\nstyle:\n  embed_mermaid_font: true\n  embed-mermaid-font: true\n---\n{MERMAID_DOC}"
    );
    let with_keys = Markdown::try_from_content(&doc).unwrap();
    let plain = Markdown::try_from_content(MERMAID_DOC).unwrap();
    let cli = parse(&["doc.md"]).unwrap();

    assert!(!html_page(&with_keys, &cli, &page_inputs(), false, &cli.render.features).unwrap().embed_mermaid_font());
    assert_eq!(
        html_page(&with_keys, &cli, &page_inputs(), false, &cli.render.features).unwrap().render_to_browser_document(&with_keys).unwrap(),
        html_page(&plain, &cli, &page_inputs(), false, &cli.render.features).unwrap().render_to_browser_document(&plain).unwrap()
    );
}

#[test]
fn default_mermaid_html_is_byte_identical_with_the_feature() {
    let fixture = CliProcessFixture::named("default_mermaid_html_is_byte_identical_with_the_feature");
    let doc = md_file(MERMAID_DOC);
    let render = |extra: &[&str]| {
        let output = fixture
            .command()
            .args(["--output", "html"])
            .args(extra)
            .arg(doc.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };

    let without = render(&[]);
    let with = render(&["--feature", "embed-mermaid-font"]);

    assert!(without.contains("class=\"mermaid\""), "{without}");
    assert!(!with.contains("@font-face"));
    assert_eq!(with, without);
}

#[test]
fn unknown_feature_exits_with_usage_error() {
    let fixture = CliProcessFixture::named("unknown_feature_exits_with_usage_error");
    let output = fixture
        .command()
        .args(["--output", "html", "--feature", "nope", "-"])
        .write_stdin("# T\n")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    for name in FEATURE_NAMES {
        assert!(stderr.contains(name), "{name}: {stderr}");
    }
}
