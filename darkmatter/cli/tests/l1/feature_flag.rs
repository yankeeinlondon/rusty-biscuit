//! `md --feature <NAME>`: the repository's `--feature` convention on `md`
//! (repeatable, comma-separated, case-insensitive; an unknown or empty name is
//! a usage error) and its lowering onto the HTML page.

use crate::common;

use clap::error::ErrorKind;
use clap::{Parser, ValueEnum};
use common::{CliProcessFixture, md_file};
use darkmatter::markdown::Markdown;
use darkmatter_cli::args::{Command, Feature};
use darkmatter_cli::artifact::{html_artifact, html_page};
use darkmatter_cli::Cli;
use std::ffi::OsString;

const MERMAID_DOC: &str = "# Diagram\n\n```mermaid\nflowchart LR\n  A --> B\n```\n";

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("md").chain(args.iter().copied()))
}

fn features(args: &[&str]) -> Vec<Feature> {
    parse(args).expect("valid --feature arguments").features
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
    let Some(Command::Render { features, .. }) = cli.command else {
        panic!("expected the render subcommand");
    };
    assert_eq!(features, [Feature::EmbedMermaidFont]);

    // Before the subcommand it lands on the top level, which `render` merges.
    let cli = parse(&["--feature", "embed-mermaid-font", "render", "doc.md"]).unwrap();
    assert_eq!(cli.features, [Feature::EmbedMermaidFont]);
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

#[test]
fn embed_mermaid_font_feature_sets_the_html_page_option() {
    let md = Markdown::try_from_content(MERMAID_DOC).unwrap();
    let with = parse(&["--feature", "embed-mermaid-font", "doc.md"]).unwrap();
    let without = parse(&["doc.md"]).unwrap();

    assert!(html_page(&md, &with, None, false, &with.features).unwrap().embed_mermaid_font());
    assert!(!html_page(&md, &without, None, false, &without.features).unwrap().embed_mermaid_font());
}

#[test]
fn no_frontmatter_key_turns_on_embedding() {
    let doc = format!(
        "---\nembed_mermaid_font: true\nembed-mermaid-font: true\nstyle:\n  embed_mermaid_font: true\n  embed-mermaid-font: true\n---\n{MERMAID_DOC}"
    );
    let with_keys = Markdown::try_from_content(&doc).unwrap();
    let plain = Markdown::try_from_content(MERMAID_DOC).unwrap();
    let cli = parse(&["doc.md"]).unwrap();

    assert!(!html_page(&with_keys, &cli, None, false, &cli.features).unwrap().embed_mermaid_font());
    assert_eq!(
        html_artifact(&with_keys, &cli, None, false, &cli.features).unwrap().content,
        html_artifact(&plain, &cli, None, false, &cli.features).unwrap().content
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
