use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::utils::layout::{Length, TargetValue};
use clap::Args as ClapArgs;
use std::rc::Rc;

/// Render styled text in a block quote
#[derive(ClapArgs, Debug, Clone)]
pub struct QuoteArgs {
    #[arg(value_name = "CONTENT")]
    pub content: Vec<String>,

    #[arg(long)]
    pub attribution: Option<String>,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for QuoteArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let text = self.content.join(" ");

        if text.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No content provided. Usage: bt quote \"To be or not to be\" --attribution \"Shakespeare\""
            ));
        }

        let text = crate::types::unescape_shell_escapes(&text);
        let prose = Prose::new(&text);

        let mut quote = BlockQuote::new(
            RenderableTerminalContent::Component(Rc::new(prose)),
            self.attribution.as_deref(),
        );

        if let Some(left) = self.layout.margin_left {
            quote = quote.left_margin(TargetValue::universal(Length::ch(left)));
        }
        if let Some(right) = self.layout.margin_right {
            quote = quote.right_margin(TargetValue::universal(Length::ch(right)));
        }
        if let Some(align) = self.layout.alignment {
            quote = quote.alignment(align);
        }

        let term = detect_terminal_honoring_force_color();
        let output = quote.render(&term);

        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })
    }
}
