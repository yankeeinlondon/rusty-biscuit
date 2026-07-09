use crate::commands::color_parse::parse_color;
use crate::commands::shared::{print_example_command, terminal_for_render};
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use clap::Args as ClapArgs;
use renderable::markdown::MarkdownRenderable;

const PROGRESS_EXAMPLE_PERCENT: u8 = 72;
const PROGRESS_EXAMPLE_CMD: &str =
    r#"bt progress 72 --label "Indexing" --width 28 --fill-color green --bracket-color cyan"#;

/// Render a progress bar through the canonical render tree.
///
/// The default target is the terminal. Switch to a different render target
/// with one of the mutually-exclusive `--html`, `--md`, or `--md-plus` flags.
#[derive(ClapArgs, Debug, Clone)]
pub struct ProgressArgs {
    /// Render an example and show the command used.
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Completion percentage (0-100).
    #[arg(value_name = "PERCENT", required_unless_present = "example")]
    pub percent: Option<u8>,

    /// Label shown before the bar.
    #[arg(long)]
    pub label: Option<String>,

    /// Width of the bar portion in characters (or CSS `ch` for browser/MarkdownPlus).
    #[arg(long)]
    pub width: Option<u32>,

    /// Color of the filled portion of the track (named or #rrggbb).
    #[arg(long = "fill-color")]
    pub fill_color: Option<String>,

    /// Color of the empty portion of the track (named or #rrggbb).
    #[arg(long = "empty-color")]
    pub empty_color: Option<String>,

    /// Color of the bracket glyphs (named or #rrggbb). Retained for terminal
    /// and hint-preserving outputs; the browser output uses it only when
    /// rendering bracket affordances.
    #[arg(long = "bracket-color")]
    pub bracket_color: Option<String>,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    ///
    /// Portable Markdown has no progress widget, so the output is just
    /// `"{label} {percentage}%"` (or `"{percentage}%"` when no label is set).
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus (inline HTML allowed) instead of the terminal.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,
}

impl Run for ProgressArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let percent = self.percent.unwrap_or(PROGRESS_EXAMPLE_PERCENT);
        if percent > 100 {
            return Err(color_eyre::eyre::eyre!(
                "Percentage must be 0-100. Usage: bt progress 60 --label Loading"
            ));
        }

        let mut progress = Progress::new(f32::from(percent) / 100.0);
        let label = self
            .label
            .or_else(|| self.example.then(|| "Indexing".to_string()));
        if let Some(label) = label {
            progress = progress.with_label(label);
        }
        let width = self.width.or_else(|| self.example.then_some(28));
        if let Some(width) = width {
            progress = progress.with_bar_width(width);
        }
        let fill_color = self
            .fill_color
            .or_else(|| self.example.then(|| "green".to_string()));
        if let Some(color) = &fill_color {
            progress = progress.with_filled_color(parse_color(color)?);
        }
        if let Some(color) = &self.empty_color {
            progress = progress.with_empty_color(parse_color(color)?);
        }
        let bracket_color = self
            .bracket_color
            .or_else(|| self.example.then(|| "cyan".to_string()));
        if let Some(color) = &bracket_color {
            progress = progress.with_bracket_color(parse_color(color)?);
        }

        // Dispatch by selected target. Mutual exclusion is enforced by clap.
        if self.html {
            println!("{}", progress.render_html_fragment().render());
        } else if self.md {
            println!("{}", progress.render_markdown());
        } else if self.md_plus {
            println!("{}", progress.render_markdown_plus());
        } else {
            let term = terminal_for_render(ctx.plain);
            println!("{}", progress.render(&term));
        }

        if self.example {
            print_example_command(PROGRESS_EXAMPLE_CMD);
        }
        Ok(())
    }
}
