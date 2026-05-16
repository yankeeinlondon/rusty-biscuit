use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{TerminalRenderable, RenderableTerminalContent};
use biscuit_terminal::utils::layout::Margin;
use clap::Args as ClapArgs;
use std::rc::Rc;

/// Render a bulleted list with hanging indents
#[derive(ClapArgs, Debug, Clone)]
pub struct ListArgs {
    #[arg(value_name = "ITEMS", required = true)]
    pub items: Vec<String>,

    #[arg(long, short = 'b', default_value = "• ")]
    pub bullet: String,

    #[arg(long)]
    pub no_hanging_indent: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for ListArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        if self.items.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No items provided. Usage: bt list \"First item\" \"Second item\" \"Third item\""
            ));
        }

        let prose_items: Vec<RenderableTerminalContent> = self
            .items
            .iter()
            .map(|item| {
                let text = crate::types::unescape_shell_escapes(item);
                let prose = Prose::new(&text);
                RenderableTerminalContent::Component(Rc::new(prose))
            })
            .collect();

        let mut list = UnorderedList::from(prose_items).with_bullet(&self.bullet);

        if self.no_hanging_indent {
            list = list.without_hanging_indent();
        }

        if let Some(left) = self.layout.margin_left {
            list = list.left_margin(Margin::Chars(left));
        }
        if let Some(right) = self.layout.margin_right {
            list = list.right_margin(Margin::Chars(right));
        }
        if let Some(align) = self.layout.alignment {
            list = list.alignment(align);
        }

        let term = detect_terminal_honoring_force_color();
        let output = list.render(&term);

        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })
    }
}
