//! The render positionals through the real parser: which tokens reach
//! `args` and `literal_args`, how the classifier splits them, and which root
//! render arguments a subcommand rejects.

use super::*;
use crate::args::{Cli, Command};
use clap::{CommandFactory as _, Parser as _};
use std::collections::BTreeMap;

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("md").chain(args.iter().copied()))
        .unwrap_or_else(|error| panic!("{args:?} should parse: {error}"))
}

fn os(tokens: &[&str]) -> Vec<OsString> {
    tokens.iter().map(OsString::from).collect()
}

/// The render options `md` dispatches for `args`.
fn render_options(args: &[&str]) -> RenderOptions {
    match parse(args).command_or_render() {
        Command::Render(options) => options,
        other => panic!("{args:?} should render, got {other:?}"),
    }
}

fn classified(args: &[&str]) -> RenderInput {
    let options = render_options(args);
    classify_render_args(&options.args, &options.literal_args).expect("one input at most")
}

fn setter(argument: &str, key: &str, value: &str) -> RawStyleSetter {
    RawStyleSetter { argument: argument.into(), key: key.into(), value: value.into() }
}

#[test]
fn a_bare_file_is_the_input() {
    let options = render_options(&["doc.md"]);
    assert_eq!(options.args, os(&["doc.md"]));
    assert!(options.literal_args.is_empty());
    assert_eq!(classified(&["doc.md"]), RenderInput { input: Some("doc.md".into()), setters: vec![] });
}

#[test]
fn a_command_name_after_the_terminator_is_a_file() {
    let cli = parse(&["--", "render"]);
    assert!(cli.command.is_none(), "`md -- render` renders a file named `render`");
    assert!(cli.render.args.is_empty());
    assert_eq!(cli.render.literal_args, os(&["render"]));
    assert_eq!(classified(&["--", "render"]).input, Some("render".into()));

    let options = render_options(&["render", "--", "toc"]);
    assert!(options.args.is_empty());
    assert_eq!(options.literal_args, os(&["toc"]));
    assert_eq!(classified(&["render", "--", "toc"]).input, Some("toc".into()));
}

#[test]
fn setters_sit_on_either_side_of_the_input() {
    let input = classified(&["style.margin=2", "doc.md", "style.ml=6"]);
    assert_eq!(input.input, Some("doc.md".into()));
    assert_eq!(
        input.setters,
        [setter("style.margin=2", "style.margin", "2"), setter("style.ml=6", "style.ml", "6")]
    );
    assert_eq!(classified(&["render", "style.ml=6", "doc.md"]), classified(&["render", "doc.md", "style.ml=6"]));
}

#[test]
fn a_setter_looking_file_is_an_input_with_dot_slash_or_after_the_terminator() {
    for args in [&["render", "./style.draft=old.md"][..], &["./style.draft=old.md"][..]] {
        let input = classified(args);
        assert_eq!(input.input, Some("./style.draft=old.md".into()), "{args:?}");
        assert!(input.setters.is_empty(), "{args:?}");
    }
    for args in [&["render", "--", "style.draft=old.md"][..], &["--", "style.draft=old.md"][..]] {
        let options = render_options(args);
        assert_eq!(options.literal_args, os(&["style.draft=old.md"]), "{args:?}");
        let input = classify_render_args(&options.args, &options.literal_args).unwrap();
        assert_eq!(input.input, Some("style.draft=old.md".into()), "{args:?}");
        assert!(input.setters.is_empty(), "{args:?}");
    }
}

#[test]
fn a_whole_style_value_is_one_setter() {
    let input = classified(&["render", "doc.md", "style={ max-width: 50 }"]);
    assert_eq!(input.input, Some("doc.md".into()));
    assert_eq!(input.setters, [setter("style={ max-width: 50 }", "style", "{ max-width: 50 }")]);
}

#[test]
fn a_value_is_split_at_the_first_equals_only() {
    let input = classified(&["doc.md", r#"style.page.meta={a: "x=y"}"#]);
    assert_eq!(input.setters, [setter(r#"style.page.meta={a: "x=y"}"#, "style.page.meta", r#"{a: "x=y"}"#)]);
}

#[test]
fn an_empty_value_is_still_a_setter() {
    let input = classified(&["doc.md", "style="]);
    assert_eq!(input.setters, [setter("style=", "style", "")]);
    assert_eq!(input.input, Some("doc.md".into()));
}

#[test]
fn only_style_or_a_style_dot_prefix_makes_a_setter() {
    for token in ["stylesheet=x", "styles.margin=1", "stlye.ul.x=4", "style", "style.margin", "x.style=1"] {
        let input = classify_render_args(&os(&[token]), &[]).unwrap();
        assert_eq!(input.input, Some(PathBuf::from(token)), "{token}");
        assert!(input.setters.is_empty(), "{token}");
    }
}

#[test]
fn setter_hint_suggests_the_style_spelling() {
    assert_eq!(setter_hint(OsStr::new("stlye.ul.x=4")).as_deref(), Some("style.ul.x=4"));
    assert_eq!(setter_hint(OsStr::new("page.meta=a=b")).as_deref(), Some("style.meta=a=b"));
    for token in ["doc.md", "x=1", "plain", "a=b.c"] {
        assert_eq!(setter_hint(OsStr::new(token)), None, "{token}");
    }
}

#[test]
fn two_inputs_are_a_usage_error() {
    for args in [&["a.md", "b.md"][..], &["render", "a.md", "--", "b.md"][..], &["--", "a.md", "b.md"][..]] {
        let options = render_options(args);
        let error = classify_render_args(&options.args, &options.literal_args).expect_err("two inputs");
        assert!(matches!(&error, RenderArgsError::MultipleInputs(inputs) if inputs.len() == 2), "{args:?}");
        assert_eq!(error.usage_error().exit_code(), 2, "{args:?}");
        let prepared = crate::args::PreparedRenderOptions::prepare(&options).expect_err("two inputs");
        assert_eq!(prepared.exit_code(), 2, "{args:?}");
    }
}

/// A token the OS gave as invalid Unicode is an input, kept byte for byte.
#[cfg(unix)]
#[test]
fn a_non_unicode_token_is_kept_native() {
    use std::os::unix::ffi::OsStrExt as _;
    let native = OsStr::from_bytes(b"doc-\xff.md");
    let setter_like = OsStr::from_bytes(b"style.margin=\xff");
    let cli = Cli::try_parse_from([OsStr::new("md"), native]).expect("a native token parses");
    assert_eq!(cli.render.args, [native.to_os_string()]);
    let input = classify_render_args(&cli.render.args, &cli.render.literal_args).unwrap();
    assert_eq!(input.input.as_deref().map(|path| path.as_os_str()), Some(native));

    // A non-Unicode word is never a setter, even with a setter's prefix.
    let input = classify_render_args(&[setter_like.to_os_string()], &[]).unwrap();
    assert_eq!(input.input.as_deref().map(|path| path.as_os_str()), Some(setter_like));
    assert!(input.setters.is_empty());
    assert_eq!(setter_hint(setter_like), None);
}

/// The Windows counterpart: an unpaired surrogate is not Unicode.
#[cfg(windows)]
#[test]
fn a_non_unicode_token_is_kept_native() {
    use std::os::windows::ffi::OsStringExt as _;
    let native = OsString::from_wide(&[0x64, 0xD800, 0x2E, 0x6D, 0x64]);
    let cli = Cli::try_parse_from([OsString::from("md"), native.clone()]).expect("a native token parses");
    assert_eq!(cli.render.args, [native.clone()]);
    let input = classify_render_args(&cli.render.args, &cli.render.literal_args).unwrap();
    assert_eq!(input.input.map(PathBuf::into_os_string), Some(native));
}

#[test]
fn a_non_ascii_unicode_token_is_kept_intact() {
    let input = classified(&["dóc ünïcode.md", "style.page.title=café"]);
    assert_eq!(input.input, Some("dóc ünïcode.md".into()));
    assert_eq!(input.setters, [setter("style.page.title=café", "style.page.title", "café")]);
}

/// A root representative for each render argument: the tokens that set it
/// before a subcommand, or `None` where no spelling can (nothing after `--`
/// is a subcommand).
fn root_representatives() -> BTreeMap<&'static str, Option<&'static [&'static str]>> {
    BTreeMap::from([
        ("args", Some(&["style.margin=4"][..])),
        ("literal_args", None),
        ("style", Some(&["--style", "s.yaml"][..])),
        ("output", Some(&["--output", "html"][..])),
        ("show", Some(&["--show"][..])),
        ("indent", Some(&["--indent", "2"][..])),
        ("deny", Some(&["--deny", "style"][..])),
        ("features", Some(&["--feature", "embed-mermaid-font"][..])),
        ("mermaid", Some(&["--mermaid"][..])),
    ])
}

fn usage_conflict(args: &[&str]) -> clap::Error {
    let cli = parse(args);
    assert!(cli.command.is_some(), "{args:?} should reach a subcommand");
    crate::commands::validate_subcommand_usage(&cli)
        .expect_err("a root render argument before a subcommand")
        .downcast::<clap::Error>()
        .expect("a usage error")
}

#[test]
fn every_root_render_argument_before_a_subcommand_is_a_usage_error() {
    let representatives = root_representatives();
    let definition = RenderOptions::augment_args(clap::Command::new("x"));
    let ids: Vec<&str> = definition.get_arguments().map(|arg| arg.get_id().as_str()).collect();
    assert_eq!(
        ids.iter().copied().collect::<std::collections::BTreeSet<_>>(),
        representatives.keys().copied().collect(),
        "every render argument needs a root representative, and only those"
    );

    for id in ids {
        match representatives[id] {
            Some(tokens) => {
                for command in [&["toc", "doc.md"][..], &["render", "doc.md"][..]] {
                    let args: Vec<&str> = tokens.iter().chain(command).copied().collect();
                    let error = usage_conflict(&args);
                    assert_eq!(error.exit_code(), 2, "{args:?}");
                    assert!(error.to_string().contains("subcommands cannot be combined"), "{args:?}: {error}");
                }
            }
            None => {
                let cli = parse(&["--", "x", "toc", "doc.md"]);
                assert!(cli.command.is_none(), "{id}: tokens after `--` never select a subcommand");
                assert_eq!(cli.render.literal_args, os(&["x", "toc", "doc.md"]));
            }
        }
    }
}

#[test]
fn spelled_root_render_arguments_are_rejected_before_a_subcommand() {
    for args in [
        &["--feature", "embed-mermaid-font", "render", "doc.md"][..],
        &["style.margin=4", "toc", "doc.md"][..],
        &["style={}", "toc", "doc.md"][..],
        &["--style", "s.yaml", "render", "doc.md"][..],
        &["--mermaid", "render", "doc.md"][..],
        &["doc.md", "toc", "other.md"][..],
    ] {
        assert_eq!(usage_conflict(args).exit_code(), 2, "{args:?}");
    }
}

#[test]
fn shared_controls_before_a_subcommand_are_accepted() {
    let cli = parse(&["--width", "80", "-v", "--debug", "1", "--magic-root", "dir", "toc", "doc.md"]);
    assert!(cli.render.has_explicit_arguments().is_empty());
    crate::commands::validate_subcommand_usage(&cli).expect("shared controls are not render arguments");
}

#[test]
fn render_options_have_no_defaults() {
    let options = render_options(&[]);
    assert!(options.has_explicit_arguments().is_empty(), "{options:?}");
    let definition = RenderOptions::augment_args(clap::Command::new("x"));
    for arg in definition.get_arguments() {
        assert!(arg.get_default_values().is_empty(), "{} has a default", arg.get_id());
    }
}

#[test]
fn the_render_subcommand_carries_its_own_options() {
    let options = render_options(&["render", "doc.md", "--style", "a.yaml", "--style", "b.toml", "--mermaid", "--feature", "no-mermaid-rendering"]);
    assert_eq!(options.style, os(&["a.yaml", "b.toml"]));
    assert!(options.mermaid);
    assert_eq!(options.features, [Feature::NoMermaidRendering]);

    let prepared = crate::args::PreparedRenderOptions::prepare(&options).unwrap();
    assert_eq!(prepared.input, Some("doc.md".into()));
    assert_eq!(prepared.styles, os(&["a.yaml", "b.toml"]));
    assert!(prepared.mermaid);
    assert_eq!(prepared.features, [Feature::NoMermaidRendering]);
}

#[test]
fn render_help_lists_every_render_option_and_root_help_does_not() {
    let mut root = Cli::command();
    let root_help = root.render_help().to_string();
    let render_help = root
        .find_subcommand_mut("render")
        .expect("render subcommand")
        .render_help()
        .to_string();
    for flag in ["--style", "--output", "--show", "--indent", "--deny", "--feature", "--mermaid", "[ARGS]"] {
        assert!(render_help.contains(flag), "`md render --help` lacks {flag}:\n{render_help}");
    }
    for hidden in ["--output", "--show", "--indent", "--deny", "--feature", "--mermaid"] {
        assert!(!root_help.contains(hidden), "`md --help` lists {hidden}:\n{root_help}");
    }
    // Removed, so in neither help.
    for removed in ["--max-width", "--margin", "--theme", "--code-theme", "--fill", "--align-", "--line-numbers", "--page-bg"] {
        assert!(!root_help.contains(removed), "`md --help` lists {removed}:\n{root_help}");
        assert!(!render_help.contains(removed), "`md render --help` lists {removed}:\n{render_help}");
    }
    for shown in ["--style", "--width", "--verbose", "--magic-root", "Commands:", "Shared controls:", "see `md render --help`"] {
        assert!(root_help.contains(shown), "`md --help` lacks {shown}:\n{root_help}");
    }
    assert!(!render_help.contains("see `md render --help`"), "the alias sentence is root-only:\n{render_help}");
}
