use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::utils::layout::Margin;
use clap::Args as ClapArgs;

/// Render two columns of text side by side
#[derive(ClapArgs, Debug, Clone)]
pub struct ColumnsArgs {
    #[arg(value_name = "LEFT", id = "left_content")]
    pub left: String,

    #[arg(value_name = "RIGHT")]
    pub right: String,

    #[arg(long, default_value_t = 3)]
    pub gap: u32,

    #[arg(long = "left", value_name = "WIDTH")]
    pub left_width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for ColumnsArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let left_text = crate::types::unescape_shell_escapes(&self.left);
        let right_text = crate::types::unescape_shell_escapes(&self.right);

        let mut columns = TwoColumn::new(left_text, right_text).with_gap(self.gap);

        if let Some(spec) = &self.left_width {
            columns = columns.with_left_width(parse_column_width(&spec.to_string())?);
        }

        if let Some(left) = self.layout.margin_left {
            columns = columns.left_margin(Margin::Chars(left));
        }
        if let Some(right) = self.layout.margin_right {
            columns = columns.right_margin(Margin::Chars(right));
        }
        if let Some(align) = self.layout.alignment {
            columns = columns.alignment(align);
        }

        let term = detect_terminal_honoring_force_color();
        let output = columns.render(&term);

        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })
    }
}
