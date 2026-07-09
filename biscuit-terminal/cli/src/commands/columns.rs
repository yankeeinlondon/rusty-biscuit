use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::utils::layout::{Length, TargetValue};
use clap::Args as ClapArgs;
use renderable::markdown::MarkdownRenderable;

const COLUMNS_EXAMPLE_LEFT: &str = "<b>Release</b>";
const COLUMNS_EXAMPLE_RIGHT: &str = "Build passed, smoke tests are green, and docs were updated.";
const COLUMNS_EXAMPLE_CMD: &str = r#"bt columns --gap 6 --left 18 "<b>Release</b>" "Build passed, smoke tests are green, and docs were updated.""#;

/// Render two columns of text side by side through the canonical render tree.
///
/// The default target is the terminal. Switch to a different render target
/// with one of the mutually-exclusive `--html`, `--md`, or `--md-plus` flags.
#[derive(ClapArgs, Debug, Clone)]
pub struct ColumnsArgs {
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(
        value_name = "LEFT",
        id = "left_content",
        required_unless_present = "example"
    )]
    pub left: Option<String>,

    #[arg(value_name = "RIGHT", required_unless_present = "example")]
    pub right: Option<String>,

    #[arg(long, default_value_t = 3)]
    pub gap: u32,

    #[arg(long = "left", value_name = "WIDTH")]
    pub left_width: Option<crate::types::WidthSpec>,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    ///
    /// Portable Markdown has no side-by-side layout, so the columns collapse
    /// to sequential blocks (left, blank line, right). Width and gap are
    /// dropped.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus (inline HTML allowed) instead of the terminal.
    ///
    /// MarkdownPlus preserves the two-column layout as a `<div class="columns">`
    /// flex container with width and gap encoded as inline CSS.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for ColumnsArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let left = self
            .left
            .unwrap_or_else(|| COLUMNS_EXAMPLE_LEFT.to_string());
        let right = self
            .right
            .unwrap_or_else(|| COLUMNS_EXAMPLE_RIGHT.to_string());
        let left_text = crate::types::unescape_shell_escapes(&left);
        let right_text = crate::types::unescape_shell_escapes(&right);

        let gap = if self.example && self.gap == 3 {
            6
        } else {
            self.gap
        };
        let mut columns = TwoColumn::new(left_text, right_text).with_gap(gap);

        let example_left_width = self.example.then_some(crate::types::WidthSpec::Chars(18));
        let left_width = self.left_width.as_ref().or(example_left_width.as_ref());
        if let Some(spec) = left_width {
            columns = columns.with_left_width(parse_column_width(&spec.to_string())?);
        }

        if let Some(left) = self.layout.margin_left {
            columns = columns.left_margin(TargetValue::universal(Length::ch(left)));
        }
        if let Some(right) = self.layout.margin_right {
            columns = columns.right_margin(TargetValue::universal(Length::ch(right)));
        }
        if let Some(align) = self.layout.alignment {
            columns = columns.alignment(align);
        }

        // Dispatch by selected target. Mutual exclusion is enforced by clap.
        if self.html {
            println!("{}", columns.render_html_fragment().render());
        } else if self.md {
            println!("{}", columns.render_markdown());
        } else if self.md_plus {
            println!("{}", columns.render_markdown_plus());
        } else {
            let term = terminal_for_render(ctx.plain);
            let output = columns.render(&term);
            emit_vertical_margins(&self.layout, || {
                println!("{}", output);
                Ok(())
            })?;
        }

        if self.example {
            print_example_command(COLUMNS_EXAMPLE_CMD);
        }

        Ok(())
    }
}
