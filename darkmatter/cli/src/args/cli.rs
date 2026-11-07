use clap::Parser;
use clap_complete::engine::ArgValueCompleter;
use crate::args::{
    CliFill, PageAlignmentArg, PageBackgroundArg,
    complete_theme_names, parse_cli_fill,
    parse_theme_name,
};
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use darkmatter::markdown::highlighting::ThemePair;
use renderable::style::PaintColor;
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
    mut_arg("max_width", |arg| arg.hide(true)),
    mut_arg("indent", |arg| arg.hide(true)),
    mut_arg("deny", |arg| arg.hide(true)),
    mut_arg("features", |arg| arg.hide(true)),
    mut_arg("mermaid", |arg| arg.hide(true)),
)]
pub struct Cli {
    /// Options used when no subcommand is supplied.
    #[command(flatten)]
    pub render: crate::args::RenderOptions,

    /// Theme for prose content (kebab-case name)
    #[arg(hide = true, long, value_parser = parse_theme_name, add = ArgValueCompleter::new(complete_theme_names))]
    pub theme: Option<ThemePair>,

    /// Theme for code blocks (overrides derived theme)
    #[arg(hide = true, long, value_parser = parse_theme_name, add = ArgValueCompleter::new(complete_theme_names))]
    pub code_theme: Option<ThemePair>,

    /// List available themes
    #[arg(hide = true, long)]
    pub list_themes: bool,

    /// Shorthand for `clean --save` with top-level [INPUT]
    #[arg(hide = true, long)]
    pub save: bool,

    // ── Layout flags (Phase 5) ──────────────────────────────────────────
    /// Margin on all sides (cells)
    #[arg(hide = true, short = 'm', long, value_name = "N")]
    pub margin: Option<u16>,

    /// Horizontal margin (left + right)
    #[arg(hide = true, long, value_name = "N")]
    pub mx: Option<u16>,

    /// Vertical margin (top + bottom)
    #[arg(hide = true, long, value_name = "N")]
    pub my: Option<u16>,

    /// Top margin
    #[arg(hide = true, long, visible_alias = "margin-top", value_name = "N")]
    pub mt: Option<u16>,

    /// Bottom margin
    #[arg(hide = true, long, visible_alias = "margin-bottom", value_name = "N")]
    pub mb: Option<u16>,

    /// Left margin
    #[arg(hide = true, long, visible_alias = "margin-left", value_name = "N")]
    pub ml: Option<u16>,

    /// Right margin
    #[arg(hide = true, long, visible_alias = "margin-right", value_name = "N")]
    pub mr: Option<u16>,

    /// Padding on all sides (cells)
    #[arg(hide = true, long, value_name = "N")]
    pub padding: Option<u16>,

    /// Horizontal padding (left + right)
    #[arg(hide = true, long, value_name = "N")]
    pub px: Option<u16>,

    /// Vertical padding (top + bottom)
    #[arg(hide = true, long, value_name = "N")]
    pub py: Option<u16>,

    /// Top padding
    #[arg(hide = true, long, visible_alias = "padding-top", value_name = "N")]
    pub pt: Option<u16>,

    /// Bottom padding
    #[arg(hide = true, long, visible_alias = "padding-bottom", value_name = "N")]
    pub pb: Option<u16>,

    /// Left padding
    #[arg(hide = true, long, visible_alias = "padding-left", value_name = "N")]
    pub pl: Option<u16>,

    /// Right padding
    #[arg(hide = true, long, visible_alias = "padding-right", value_name = "N")]
    pub pr: Option<u16>,

    /// Page background style
    #[arg(
        hide = true,
        long,
        visible_alias = "page-background",
        value_enum,
        value_name = "STYLE"
    )]
    pub page_bg: Option<PageBackgroundArg>,

    /// Explicit page background color (e.g. `#1e1e23` or `30,30,35`).
    ///
    /// Overrides the computed `PageBackground` color when set. Accepts a
    /// hex string (`#RGB` / `#RRGGBB`) or a comma-separated `R,G,B` triple
    /// in the 0-255 range. Tailwind palette names (`red-500`, `slate-50`,
    /// etc.) and CSS special keywords (`transparent`, `inherit`,
    /// `currentColor`) are also accepted.
    #[arg(hide = true, long, value_name = "COLOR", value_parser = PaintColor::from_css_str)]
    pub page_bg_color: Option<PaintColor>,

    /// Width the output is wrapped to: N columns, N%, `auto` (`OUTPUT_COLUMNS`,
    /// else the terminal, else unlimited), `source` (keep authored line
    /// breaks), or `unlimited`. `md render`'s `--max-width` caps content inside it.
    #[arg(
        long,
        value_name = "WIDTH",
        value_parser = crate::args::output::parse_width,
        global = true,
        help_heading = SHARED_HEADING,
        add = ArgValueCompleter::new(crate::args::complete_width_values)
    )]
    pub width: Option<WidthRequest>,

    /// Include line numbers in code blocks
    ///
    /// Accepts `--line-numbers` (defaults to `true`) or
    /// `--line-numbers <true|false>` for explicit control.
    #[arg(
        hide = true,
        long,
        value_name = "BOOL",
        num_args = 0..=1,
        default_missing_value = "true",
        require_equals = false,
    )]
    pub line_numbers: Option<bool>,

    /// Default alignment for all components
    #[arg(hide = true, long, value_enum, value_name = "ALIGN")]
    pub alignment: Option<PageAlignmentArg>,

    /// Image alignment
    #[arg(hide = true, long, value_enum, value_name = "ALIGN")]
    pub align_images: Option<PageAlignmentArg>,

    /// List alignment
    #[arg(hide = true, long, value_enum, value_name = "ALIGN")]
    pub align_lists: Option<PageAlignmentArg>,

    /// Unordered list alignment
    #[arg(hide = true, long, value_enum, value_name = "ALIGN")]
    pub align_ul: Option<PageAlignmentArg>,

    /// Ordered list alignment
    #[arg(hide = true, long, value_enum, value_name = "ALIGN")]
    pub align_ol: Option<PageAlignmentArg>,

    /// List item alignment
    #[arg(hide = true, long, value_enum, value_name = "ALIGN")]
    pub align_li: Option<PageAlignmentArg>,

    /// Block quote alignment
    #[arg(hide = true, long, value_enum, value_name = "ALIGN")]
    pub align_block_quotes: Option<PageAlignmentArg>,

    /// Table alignment
    #[arg(hide = true, long, value_enum, value_name = "ALIGN")]
    pub align_tables: Option<PageAlignmentArg>,

    /// Code block alignment
    #[arg(hide = true, long, value_enum, value_name = "ALIGN")]
    pub align_code_blocks: Option<PageAlignmentArg>,

    /// Default fill for all components
    #[arg(hide = true, long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill: Option<CliFill>,

    /// Image fill
    #[arg(hide = true, long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_images: Option<CliFill>,

    /// List fill
    #[arg(hide = true, long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_lists: Option<CliFill>,

    /// Unordered list fill
    #[arg(hide = true, long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_ul: Option<CliFill>,

    /// Ordered list fill
    #[arg(hide = true, long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_ol: Option<CliFill>,

    /// List item fill
    #[arg(hide = true, long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_li: Option<CliFill>,

    /// Block quote fill
    #[arg(hide = true, long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_block_quotes: Option<CliFill>,

    /// Table fill
    #[arg(hide = true, long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_tables: Option<CliFill>,

    /// Code block fill
    #[arg(hide = true, long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_code_blocks: Option<CliFill>,

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
    fn cli_margin_flags_parse_correctly() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--margin", "4", "--mt", "1", "--mx", "2"])
            .unwrap();
        assert_eq!(cli.margin, Some(4));
        assert_eq!(cli.mt, Some(1));
        assert_eq!(cli.mx, Some(2));
    }

    #[test]
    fn cli_padding_flags_parse_correctly() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--padding", "2", "--px", "1"]).unwrap();
        assert_eq!(cli.padding, Some(2));
        assert_eq!(cli.px, Some(1));
    }

    #[test]
    fn cli_page_bg_flag_parses() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--page-bg", "subtle"]).unwrap();
        assert!(cli.page_bg.is_some());
    }

    #[test]
    fn cli_alignment_flags_parse() {
        let cli = Cli::try_parse_from([
            "md",
            "doc.md",
            "--alignment",
            "center",
            "--align-images",
            "left",
        ])
        .unwrap();
        assert!(cli.alignment.is_some());
        assert!(cli.align_images.is_some());
    }

    #[test]
    fn cli_fill_flags_parse() {
        let cli = Cli::try_parse_from([
            "md",
            "doc.md",
            "--fill",
            "pad=4",
            "--fill-code-blocks",
            "max=40",
        ])
        .unwrap();
        assert!(cli.fill.is_some());
        assert!(cli.fill_code_blocks.is_some());
    }

    #[test]
    fn cli_line_numbers_bare_flag_parses_as_true() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--line-numbers"]).unwrap();
        assert_eq!(cli.line_numbers, Some(true));
    }

    #[test]
    fn cli_line_numbers_true_parses() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--line-numbers", "true"]).unwrap();
        assert_eq!(cli.line_numbers, Some(true));
    }

    #[test]
    fn cli_line_numbers_false_parses() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--line-numbers", "false"]).unwrap();
        assert_eq!(cli.line_numbers, Some(false));
    }

    #[test]
    fn cli_line_numbers_omitted_is_none() {
        let cli = Cli::try_parse_from(["md", "doc.md"]).unwrap();
        assert_eq!(cli.line_numbers, None);
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
