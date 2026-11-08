use clap::Parser;
use clap_complete::engine::ArgValueCompleter;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use std::path::PathBuf;

/// The root help's heading for options every command shares.
const SHARED_HEADING: &str = "Shared controls";

/// The root help's heading for the render options it lists.
const RENDER_HEADING: &str = "Render";

/// `md --help`'s layout: the root is also `md render`, so it says so and
/// points at the full render help instead of listing every render option.
const ROOT_HELP_TEMPLATE: &str = "\
{before-help}{about-with-newline}
{usage-heading} {usage}

`md [ARGS] [OPTIONS]` is `md render [ARGS] [OPTIONS]`; see `md render --help`.

{all-args}{after-help}";

/// Command-line interface for the darkmatter markdown renderer.
///
/// Use `md --help` to see all available options.
#[derive(Parser)]
#[command(name = "md", about = "Darkmatter CLI", version)]
#[command(subcommand_precedence_over_arg = true, disable_help_subcommand = true)]
// Root help only: the root shows commands, shared controls, and `--style`; the
// flattened render options stay visible on `md render --help`, so they are
// hidden here rather than in `RenderOptions`.
#[command(help_template = ROOT_HELP_TEMPLATE)]
#[command(
    mut_arg("args", |arg| arg.hide(true)),
    mut_arg("literal_args", |arg| arg.hide(true)),
    mut_arg("style", |arg| arg.help_heading(RENDER_HEADING)),
    mut_arg("output", |arg| arg.hide(true)),
    mut_arg("show", |arg| arg.hide(true)),
    mut_arg("indent", |arg| arg.hide(true)),
    mut_arg("deny", |arg| arg.hide(true)),
    mut_arg("features", |arg| arg.hide(true)),
    mut_arg("mermaid", |arg| arg.hide(true)),
)]
pub struct Cli {
    /// Options used when no subcommand is supplied.
    #[command(flatten)]
    pub render: crate::args::RenderOptions,

    /// Width the output is wrapped to: N columns, N%, `auto` (`OUTPUT_COLUMNS`,
    /// else the terminal, else unlimited), `source` (keep authored line
    /// breaks), or `unlimited`. `md render`'s `style.max-width` caps content
    /// inside it.
    #[arg(
        long,
        value_name = "WIDTH",
        value_parser = crate::args::output::parse_width,
        global = true,
        help_heading = SHARED_HEADING,
        add = ArgValueCompleter::new(crate::args::complete_width_values)
    )]
    pub width: Option<WidthRequest>,

    /// Increase verbosity for styled user-facing output (-v summary, -vv detailed)
    #[arg(
        short = 'v',
        long = "verbose",
        action = clap::ArgAction::Count,
        global = true,
        help_heading = SHARED_HEADING
    )]
    pub verbose: u8,

    /// Search DIR for `@` references, before the default roots (repeatable;
    /// searched in the order given; give it before the subcommand). A
    /// relative DIR is relative to the launch directory; DIR must exist.
    // Not `global`: clap keeps only the deepest level's values of a global
    // `Append` argument, so `md --magic-root a compose --magic-root b` would
    // silently drop `a`.
    #[arg(
        long = "magic-root",
        value_name = "DIR",
        action = clap::ArgAction::Append,
        value_hint = clap::ValueHint::DirPath,
        help_heading = SHARED_HEADING
    )]
    pub magic_roots: Vec<PathBuf>,

    /// Enable developer debug logging (1=INFO, 2=DEBUG, 3=TRACE, 4=TRACE+locations).
    /// Alternatively, set RUST_LOG environment variable.
    #[arg(long = "debug", value_name = "LEVEL", global = true, hide = true, help_heading = SHARED_HEADING)]
    pub debug_level: Option<u8>,

    /// Generate shell completions for the specified shell
    #[arg(long, value_name = "SHELL", help_heading = SHARED_HEADING)]
    pub completions: Option<clap_complete::Shell>,

    /// Subcommand (read, clean, compose, toc, delta)
    #[command(subcommand)]
    pub command: Option<crate::args::Command>,
}

impl Cli {
    /// Whether this invocation prints a data output, which owns stderr too:
    /// nothing may be written there.
    pub fn requests_data_output(&self) -> bool {
        use crate::args::{Command, SchemaTarget, ValidateTarget};
        let target: Option<OutputTarget> = match &self.command {
            None => self.render.output,
            Some(Command::Render(options)) => options.output,
            Some(
                Command::Clean { output, .. }
                | Command::Compose { output, .. }
                | Command::Toc { output, .. }
                | Command::ListThemes { output }
                | Command::Delta { output, .. }
                | Command::Get { output, .. }
                | Command::Set { output, .. }
                | Command::Rm { output, .. }
                | Command::Edit { output, .. }
                | Command::Hash { output, .. }
                | Command::Graph { output, .. }
                | Command::CodeBlock { output, .. }
                | Command::Validate { target: ValidateTarget::Refs { output, .. } }
                | Command::Schema {
                    target:
                        SchemaTarget::Validate { output, .. } | SchemaTarget::Triggers { output, .. } | SchemaTarget::About { output, .. },
                },
            ) => Some(*output),
            Some(Command::Schema { target: SchemaTarget::Detect { output, .. } }) => Some(output.target()),
        };
        target.is_some_and(OutputTarget::is_data)
    }

    /// The explicit command, or `render` with the root's render options.
    ///
    /// Nothing is combined: a root render option before a subcommand is a
    /// usage error (`commands::validate_subcommand_usage`).
    pub fn command_or_render(&self) -> crate::args::Command {
        self.command.clone().unwrap_or_else(|| crate::args::Command::Render(self.render.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn compose_perf_flag_sets_true() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md", "--perf"]).unwrap();
        match cli.command {
            Some(crate::args::Command::Compose { perf, .. }) => assert!(perf),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_without_perf_defaults_false() {
        let cli = Cli::try_parse_from(["md", "compose"]).unwrap();
        match cli.command {
            Some(crate::args::Command::Compose { perf, .. }) => assert!(!perf),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn clean_fixed_width_flag_is_removed() {
        let error = Cli::try_parse_from(["md", "clean", "doc.md", "--fixed-width", "80"]).err().expect("removed");
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn clean_ignore_incidental_newlines_flag_is_removed() {
        let error = Cli::try_parse_from(["md", "clean", "doc.md", "--ignore-incidental-newlines"]).err().expect("removed");
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn clean_width_replaces_both_removed_flags() {
        let cli = Cli::try_parse_from(["md", "clean", "doc.md", "--width", "80"]).unwrap();
        assert_eq!(cli.width, Some(WidthRequest::Columns(80)));
        let cli = Cli::try_parse_from(["md", "clean", "doc.md", "--width", "source"]).unwrap();
        assert_eq!(cli.width, Some(WidthRequest::Source));
    }

    #[test]
    fn debug_flag_parses_level() {
        let cli = Cli::try_parse_from(["md", "--debug", "2", "doc.md"]).unwrap();
        assert_eq!(cli.debug_level, Some(2));
    }

    #[test]
    fn debug_flag_absent_is_none() {
        let cli = Cli::try_parse_from(["md", "doc.md"]).unwrap();
        assert_eq!(cli.debug_level, None);
    }

    #[test]
    fn compose_timeout_flag_parses() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md", "--timeout", "3"]).unwrap();
        match cli.command {
            Some(crate::args::Command::Compose { timeout, .. }) => assert_eq!(timeout, Some(3)),
            _ => panic!("Expected Compose command"),
        }
    }

    /// The resolved policy of a parsed `md compose`.
    fn compose_policy(cli: &Cli) -> Result<crate::args::policy::ResolvedPolicy, clap::Error> {
        match &cli.command {
            Some(crate::args::Command::Compose { allow, deny, disable, baseline_schema, .. }) => {
                crate::args::policy::resolve_for(
                    crate::args::policy::PolicyCommand::Compose,
                    allow,
                    deny,
                    disable,
                    baseline_schema.is_some().then_some("--baseline-schema <PATH>"),
                )
            }
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_allow_shell_timeout_parses() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md", "--allow", "shell-timeout"]).unwrap();
        assert!(compose_policy(&cli).unwrap().recovers("shell-timeout"));
    }

    #[test]
    fn compose_timeout_defaults_to_none() {
        let cli = Cli::try_parse_from(["md", "compose"]).unwrap();
        match cli.command {
            Some(crate::args::Command::Compose { timeout, .. }) => assert_eq!(timeout, None),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_recovers_nothing_by_default() {
        let cli = Cli::try_parse_from(["md", "compose"]).unwrap();
        let policy = compose_policy(&cli).unwrap();
        assert!(!policy.recovers("shell-timeout"));
        assert!(policy.blocking_policy().is_empty());
    }

    #[test]
    fn compose_baseline_schema_flag_parses() {
        let cli =
            Cli::try_parse_from(["md", "compose", "doc.md", "--baseline-schema", "schema.yaml"])
                .unwrap();
        match cli.command {
            Some(crate::args::Command::Compose {
                baseline_schema, ..
            }) => assert_eq!(baseline_schema, Some(PathBuf::from("schema.yaml"))),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_disable_baseline_schema_parses() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md", "--disable", "baseline-schema"]).unwrap();
        assert!(compose_policy(&cli).unwrap().disables("baseline-schema"));
    }

    #[test]
    fn compose_baseline_schema_flags_conflict() {
        let result = Cli::try_parse_from([
            "md",
            "compose",
            "doc.md",
            "--baseline-schema",
            "schema.yaml",
            "--disable",
            "baseline-schema",
        ])
        .map(|cli| compose_policy(&cli));
        let Ok(Err(err)) = result else {
            panic!("expected conflicting compose baseline flags to fail");
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn compose_args_captures_positional_tokens() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md", "key=value"]).unwrap();
        match cli.command {
            Some(crate::args::Command::Compose { args, .. }) => {
                assert_eq!(args, vec!["doc.md", "key=value"]);
            }
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_args_empty_when_no_positionals() {
        let cli = Cli::try_parse_from(["md", "compose"]).unwrap();
        match cli.command {
            Some(crate::args::Command::Compose { args, .. }) => {
                assert!(args.is_empty());
            }
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn schema_about_code_block_defaults_to_inverse() {
        let cli = Cli::try_parse_from(["md", "schema", "about"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(crate::args::Command::Schema {
                target: crate::args::SchemaTarget::About { code_block: crate::args::CodeBlockArg::Inverse, .. },
            })
        ));
    }

    #[test]
    fn schema_about_accepts_code_block_flag() {
        let cli =
            Cli::try_parse_from(["md", "schema", "about", "--code-block", "light"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(crate::args::Command::Schema {
                target: crate::args::SchemaTarget::About { code_block: crate::args::CodeBlockArg::Light, .. },
            })
        ));
    }

    #[test]
    fn code_block_is_an_unknown_argument_outside_schema_about() {
        for args in [&["md", "doc.md", "--code-block", "dark"][..], &["md", "render", "doc.md", "--code-block", "dark"][..]] {
            let error = Cli::try_parse_from(args).err().expect("--code-block belongs to `schema about`");
            assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument, "{args:?}");
        }
    }

    /// Every root presentation flag `--style` and the style setters replaced,
    /// with each visible alias, and `md render --max-width`, as written with a
    /// sample value.
    const REMOVED_RENDER_FLAGS: &[&[&str]] = &[
        &["--theme", "nord"],
        &["--code-theme", "nord"],
        &["-m", "4"],
        &["--margin", "4"],
        &["--mx", "2"],
        &["--my", "2"],
        &["--mt", "1"],
        &["--margin-top", "1"],
        &["--mb", "1"],
        &["--margin-bottom", "1"],
        &["--ml", "1"],
        &["--margin-left", "1"],
        &["--mr", "1"],
        &["--margin-right", "1"],
        &["--padding", "4"],
        &["--px", "2"],
        &["--py", "2"],
        &["--pt", "1"],
        &["--padding-top", "1"],
        &["--pb", "1"],
        &["--padding-bottom", "1"],
        &["--pl", "1"],
        &["--padding-left", "1"],
        &["--pr", "1"],
        &["--padding-right", "1"],
        &["--page-bg", "subtle"],
        &["--page-background", "subtle"],
        &["--page-bg-color", "#1e1e23"],
        &["--max-width", "40"],
        &["--line-numbers"],
        &["--line-numbers", "false"],
        &["--alignment", "center"],
        &["--align-images", "left"],
        &["--align-lists", "left"],
        &["--align-ul", "left"],
        &["--align-ol", "left"],
        &["--align-li", "left"],
        &["--align-block-quotes", "left"],
        &["--align-tables", "left"],
        &["--align-code-blocks", "left"],
        &["--fill", "full"],
        &["--fill-images", "max=40"],
        &["--fill-lists", "pad=4"],
        &["--fill-ul", "indent=4"],
        &["--fill-ol", "explicit=50"],
        &["--fill-li", "full"],
        &["--fill-block-quotes", "max=40"],
        &["--fill-tables", "max=40"],
        &["--fill-code-blocks", "max=40"],
    ];

    /// A removed spelling is clap's ordinary unknown argument, with no custom
    /// message, on both `md` and `md render`.
    #[test]
    fn removed_render_flags_are_unknown_arguments() {
        for flag in REMOVED_RENDER_FLAGS {
            for prefix in [&["md", "doc.md"][..], &["md", "render", "doc.md"][..]] {
                let args: Vec<&str> = prefix.iter().chain(flag.iter()).copied().collect();
                let error = Cli::try_parse_from(&args).err().unwrap_or_else(|| panic!("{args:?} should be rejected"));
                assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument, "{args:?}: {error}");
                assert_eq!(error.exit_code(), 2, "{args:?}");
                assert!(error.to_string().contains(&format!("unexpected argument '{}'", flag[0])), "{args:?}: {error}");
            }
        }
    }

    /// The root's own arguments are only the shared controls: a new root flag
    /// is a placement decision, so it must be added here deliberately. The
    /// flattened render options are guarded by `args::render::tests`.
    #[test]
    fn the_root_declares_only_shared_controls_besides_render_options() {
        use clap::{Args as _, CommandFactory as _};
        let render: std::collections::BTreeSet<String> = crate::args::RenderOptions::augment_args(clap::Command::new("x"))
            .get_arguments()
            .map(|arg| arg.get_id().to_string())
            .collect();
        let root: std::collections::BTreeSet<String> = Cli::command()
            .get_arguments()
            .map(|arg| arg.get_id().to_string())
            .filter(|id| !render.contains(id))
            .collect();
        let expected = ["completions", "debug_level", "magic_roots", "verbose", "width"];
        assert_eq!(root, expected.iter().map(ToString::to_string).collect(), "root arguments");
    }

    #[test]
    fn magic_root_is_repeatable_before_the_subcommand() {
        let cli = Cli::try_parse_from([
            "md", "--magic-root", "first", "--magic-root", "/second", "compose", "doc.md",
        ])
        .unwrap();
        assert_eq!(cli.magic_roots, [PathBuf::from("first"), PathBuf::from("/second")]);
    }

    #[test]
    fn magic_root_after_the_subcommand_is_rejected() {
        let error = Cli::try_parse_from(["md", "compose", "doc.md", "--magic-root", "dir"])
            .err()
            .expect("--magic-root is a top-level flag");
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn magic_root_defaults_to_none() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md"]).unwrap();
        assert!(cli.magic_roots.is_empty());
    }
}
