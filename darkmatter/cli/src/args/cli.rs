use clap::Parser;
use clap_complete::engine::ArgValueCompleter;
use crate::args::{
    CliFill, CodeBlockArg, OutputFormat, PageAlignmentArg, PageBackgroundArg,
    complete_markdown_files, complete_theme_names, parse_cli_fill, parse_max_width,
    parse_theme_name, reject_width_flag,
};
use darkmatter::markdown::highlighting::ThemePair;
use renderable::style::PaintColor;
use std::path::PathBuf;

/// Command-line interface for the darkmatter markdown renderer.
///
/// Use `md --help` to see all available options.
#[derive(Parser)]
#[command(name = "md", about = "Darkmatter CLI", version)]
#[command(subcommand_precedence_over_arg = true, disable_help_subcommand = true)]
pub struct Cli {
    /// Input file path (reads from stdin if not provided, use "-" for explicit stdin)
    #[arg(add = ArgValueCompleter::new(complete_markdown_files))]
    pub input: Option<PathBuf>,

    /// Theme for prose content (kebab-case name)
    #[arg(long, value_parser = parse_theme_name, add = ArgValueCompleter::new(complete_theme_names))]
    pub theme: Option<ThemePair>,

    /// Theme for code blocks (overrides derived theme)
    #[arg(long, value_parser = parse_theme_name, add = ArgValueCompleter::new(complete_theme_names))]
    pub code_theme: Option<ThemePair>,

    /// Choose the code block theme variant relative to the page color mode:
    /// `inverse` (default, dark page -> light panel), `dark`, `light`, or `same`.
    #[arg(long, value_enum, value_name = "MODE", default_value_t = CodeBlockArg::Inverse, global = true)]
    pub code_block: CodeBlockArg,

    /// List available themes
    #[arg(long)]
    pub list_themes: bool,

    /// Output format for top-level render mode (when no subcommand given)
    #[arg(long, value_enum, default_value_t = OutputFormat::Auto)]
    pub output: OutputFormat,

    /// Open selected output in the default app using a temp file
    #[arg(long)]
    pub show: bool,

    /// Shorthand for `clean --save` with top-level [INPUT]
    #[arg(long)]
    pub save: bool,

    /// Render mermaid diagrams to terminal as images.
    /// Falls back to code blocks if terminal doesn't support images.
    #[arg(long)]
    pub mermaid: bool,

    // ── Layout flags (Phase 5) ──────────────────────────────────────────
    /// Margin on all sides (cells)
    #[arg(short = 'm', long, value_name = "N")]
    pub margin: Option<u16>,

    /// Horizontal margin (left + right)
    #[arg(long, value_name = "N")]
    pub mx: Option<u16>,

    /// Vertical margin (top + bottom)
    #[arg(long, value_name = "N")]
    pub my: Option<u16>,

    /// Top margin
    #[arg(long, visible_alias = "margin-top", value_name = "N")]
    pub mt: Option<u16>,

    /// Bottom margin
    #[arg(long, visible_alias = "margin-bottom", value_name = "N")]
    pub mb: Option<u16>,

    /// Left margin
    #[arg(long, visible_alias = "margin-left", value_name = "N")]
    pub ml: Option<u16>,

    /// Right margin
    #[arg(long, visible_alias = "margin-right", value_name = "N")]
    pub mr: Option<u16>,

    /// Padding on all sides (cells)
    #[arg(long, value_name = "N")]
    pub padding: Option<u16>,

    /// Horizontal padding (left + right)
    #[arg(long, value_name = "N")]
    pub px: Option<u16>,

    /// Vertical padding (top + bottom)
    #[arg(long, value_name = "N")]
    pub py: Option<u16>,

    /// Top padding
    #[arg(long, visible_alias = "padding-top", value_name = "N")]
    pub pt: Option<u16>,

    /// Bottom padding
    #[arg(long, visible_alias = "padding-bottom", value_name = "N")]
    pub pb: Option<u16>,

    /// Left padding
    #[arg(long, visible_alias = "padding-left", value_name = "N")]
    pub pl: Option<u16>,

    /// Right padding
    #[arg(long, visible_alias = "padding-right", value_name = "N")]
    pub pr: Option<u16>,

    /// Page background style
    #[arg(
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
    #[arg(long, value_name = "COLOR", value_parser = PaintColor::from_css_str)]
    pub page_bg_color: Option<PaintColor>,

    /// Max content width in columns (0 rejected)
    #[arg(long, value_name = "N", value_parser = parse_max_width)]
    pub max_width: Option<u16>,

    /// Intentionally unsupported width shorthand.
    ///
    /// The terminal-cell width is determined from the captured `Terminal`
    /// and the configured `--max-width`; a plain `--width` flag would
    /// shadow those for no reason. The flag is rejected with a clear
    /// error so callers know to use `--max-width` instead.
    #[arg(long, value_name = "N", value_parser = reject_width_flag)]
    pub width: Option<u16>,

    /// Include line numbers in code blocks
    ///
    /// Accepts `--line-numbers` (defaults to `true`) or
    /// `--line-numbers <true|false>` for explicit control.
    #[arg(
        long,
        value_name = "BOOL",
        num_args = 0..=1,
        default_missing_value = "true",
        require_equals = false,
    )]
    pub line_numbers: Option<bool>,

    /// Default alignment for all components
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub alignment: Option<PageAlignmentArg>,

    /// Image alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_images: Option<PageAlignmentArg>,

    /// List alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_lists: Option<PageAlignmentArg>,

    /// Unordered list alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_ul: Option<PageAlignmentArg>,

    /// Ordered list alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_ol: Option<PageAlignmentArg>,

    /// List item alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_li: Option<PageAlignmentArg>,

    /// Block quote alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_block_quotes: Option<PageAlignmentArg>,

    /// Table alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_tables: Option<PageAlignmentArg>,

    /// Code block alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_code_blocks: Option<PageAlignmentArg>,

    /// Default fill for all components
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill: Option<CliFill>,

    /// Image fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_images: Option<CliFill>,

    /// List fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_lists: Option<CliFill>,

    /// Unordered list fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_ul: Option<CliFill>,

    /// Ordered list fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_ol: Option<CliFill>,

    /// List item fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_li: Option<CliFill>,

    /// Block quote fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_block_quotes: Option<CliFill>,

    /// Table fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_tables: Option<CliFill>,

    /// Code block fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_code_blocks: Option<CliFill>,

    /// Promote schema-validation warnings (unknown / deprecated keys) to errors.
    #[arg(long)]
    pub strict_style: bool,

    /// Increase verbosity for styled user-facing output (-v summary, -vv detailed)
    #[arg(
        short = 'v',
        long = "verbose",
        action = clap::ArgAction::Count,
        global = true
    )]
    pub verbose: u8,

    /// Enable developer debug logging (1=INFO, 2=DEBUG, 3=TRACE, 4=TRACE+locations).
    /// Alternatively, set RUST_LOG environment variable.
    #[arg(long = "debug", value_name = "LEVEL", global = true, hide = true)]
    pub debug_level: Option<u8>,

    /// Generate shell completions for the specified shell
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<clap_complete::Shell>,

    /// Subcommand (read, clean, compose, toc, delta)
    #[command(subcommand)]
    pub command: Option<crate::args::Command>,
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
    fn clean_fixed_width_flag_parses() {
        let cli = Cli::try_parse_from(["md", "clean", "doc.md", "--fixed-width", "80"]).unwrap();
        match cli.command {
            Some(crate::args::Command::Clean { fixed_width, .. }) => {
                assert_eq!(fixed_width, Some(80));
            }
            _ => panic!("Expected Clean command"),
        }
    }

    #[test]
    fn clean_ignore_incidental_newlines_flag_parses() {
        let cli =
            Cli::try_parse_from(["md", "clean", "doc.md", "--ignore-incidental-newlines"])
                .unwrap();
        match cli.command {
            Some(crate::args::Command::Clean {
                ignore_incidental_newlines,
                ..
            }) => assert!(ignore_incidental_newlines),
            _ => panic!("Expected Clean command"),
        }
    }

    #[test]
    fn clean_fixed_width_conflicts_with_ignore_incidental_newlines() {
        let result = Cli::try_parse_from([
            "md",
            "clean",
            "doc.md",
            "--fixed-width",
            "80",
            "--ignore-incidental-newlines",
        ]);
        let Err(err) = result else {
            panic!("expected conflicting clean flags to fail");
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
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

    #[test]
    fn compose_allow_shell_timeout_flag_parses() {
        let cli =
            Cli::try_parse_from(["md", "compose", "doc.md", "--allow-shell-timeout"]).unwrap();
        match cli.command {
            Some(crate::args::Command::Compose {
                allow_shell_timeout,
                ..
            }) => assert!(allow_shell_timeout),
            _ => panic!("Expected Compose command"),
        }
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
    fn compose_allow_shell_timeout_defaults_false() {
        let cli = Cli::try_parse_from(["md", "compose"]).unwrap();
        match cli.command {
            Some(crate::args::Command::Compose {
                allow_shell_timeout,
                ..
            }) => assert!(!allow_shell_timeout),
            _ => panic!("Expected Compose command"),
        }
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
    fn compose_no_baseline_schema_flag_parses() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md", "--no-baseline-schema"])
            .unwrap();
        match cli.command {
            Some(crate::args::Command::Compose {
                no_baseline_schema,
                ..
            }) => assert!(no_baseline_schema),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_baseline_schema_flags_conflict() {
        let result = Cli::try_parse_from([
            "md",
            "compose",
            "doc.md",
            "--baseline-schema",
            "schema.yaml",
            "--no-baseline-schema",
        ]);
        let Err(err) = result else {
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
    fn schema_about_parses_as_schema_target() {
        let cli = Cli::try_parse_from(["md", "schema", "about"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(crate::args::Command::Schema {
                target: crate::args::SchemaTarget::About,
            })
        ));
        assert_eq!(cli.code_block, CodeBlockArg::Inverse);
    }

    #[test]
    fn schema_about_accepts_code_block_flag() {
        let cli =
            Cli::try_parse_from(["md", "schema", "about", "--code-block", "light"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(crate::args::Command::Schema {
                target: crate::args::SchemaTarget::About,
            })
        ));
        assert_eq!(cli.code_block, CodeBlockArg::Light);
    }

    #[test]
    fn render_code_block_flag_defaults_to_inverse() {
        let cli = Cli::try_parse_from(["md", "doc.md"]).unwrap();
        assert_eq!(cli.code_block, CodeBlockArg::Inverse);
    }

    #[test]
    fn render_code_block_flag_parses_dark() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--code-block", "dark"]).unwrap();
        assert_eq!(cli.code_block, CodeBlockArg::Dark);
    }
}
