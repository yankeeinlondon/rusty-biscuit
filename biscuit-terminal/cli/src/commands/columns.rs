use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::utils::layout::{Length, TargetValue};
use clap::Args as ClapArgs;

const COLUMNS_EXAMPLE_LEFT: &str = "<b>Release</b>";
const COLUMNS_EXAMPLE_RIGHT: &str = "Build passed, smoke tests are green, and docs were updated.";
const COLUMNS_EXAMPLE_CMD: &str = r#"bt columns --gap 6 --left 18 "<b>Release</b>" "Build passed, smoke tests are green, and docs were updated.""#;

/// Render two columns of text side by side
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

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for ColumnsArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
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

        let term = detect_terminal_honoring_force_color();
        let output = columns.render(&term);

        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })?;

        if self.example {
            print_example_command(COLUMNS_EXAMPLE_CMD);
        }

        Ok(())
    }
}
