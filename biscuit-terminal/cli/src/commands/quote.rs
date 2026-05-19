use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::utils::layout::{Length, TargetValue};
use clap::Args as ClapArgs;
use std::rc::Rc;

const QUOTE_EXAMPLE: &str = "<b>Clarity</b> is kind when the work gets complex.";
const QUOTE_EXAMPLE_ATTRIBUTION: &str = "Engineering Notes";
const QUOTE_EXAMPLE_CMD: &str = r#"bt quote --attribution "Engineering Notes" "<b>Clarity</b> is kind when the work gets complex.""#;

/// Render styled text in a block quote
#[derive(ClapArgs, Debug, Clone)]
pub struct QuoteArgs {
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(value_name = "CONTENT", required_unless_present = "example")]
    pub content: Vec<String>,

    #[arg(long)]
    pub attribution: Option<String>,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for QuoteArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let text = if self.example {
            QUOTE_EXAMPLE.to_string()
        } else {
            self.content.join(" ")
        };

        if text.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No content provided. Usage: bt quote \"To be or not to be\" --attribution \"Shakespeare\""
            ));
        }

        let text = crate::types::unescape_shell_escapes(&text);
        let prose = Prose::new(&text);
        let attribution = self
            .attribution
            .as_deref()
            .or_else(|| self.example.then_some(QUOTE_EXAMPLE_ATTRIBUTION));

        let mut quote = BlockQuote::new(
            RenderableTerminalContent::Component(Rc::new(prose)),
            attribution,
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
        })?;

        if self.example {
            print_example_command(QUOTE_EXAMPLE_CMD);
        }

        Ok(())
    }
}
