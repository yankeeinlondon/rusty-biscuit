use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::pad::PadLeft;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use clap::Args as ClapArgs;

const PADLEFT_EXAMPLE_WIDTH: u32 = 18;
const PADLEFT_EXAMPLE_TEXT: &str = "subtotal $42";
const PADLEFT_EXAMPLE_CMD: &str = r#"bt padleft 18 "subtotal $42""#;

const PADRIGHT_EXAMPLE_WIDTH: u32 = 18;
const PADRIGHT_EXAMPLE_TEXT: &str = "status";
const PADRIGHT_EXAMPLE_CMD: &str = r#"bt padright 18 "status""#;

/// Pad text on the left (right-align) to a minimum width
#[derive(ClapArgs, Debug, Clone)]
pub struct PadLeftArgs {
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(value_name = "WIDTH", required_unless_present = "example")]
    pub width: Option<u32>,

    #[arg(value_name = "TEXT", required_unless_present = "example")]
    pub text: Vec<String>,

    #[arg(long)]
    pub truncate: bool,
}

impl Run for PadLeftArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let width = self.width.unwrap_or(PADLEFT_EXAMPLE_WIDTH);
        let content = if self.example {
            PADLEFT_EXAMPLE_TEXT.to_string()
        } else {
            self.text.join(" ")
        };
        if content.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No content provided. Usage: bt padleft 20 \"hello\""
            ));
        }

        let prose = Prose::new(&content);
        let mut pad = PadLeft::new(prose, width);
        if self.truncate {
            pad = pad.truncate();
        }

        let term = terminal_for_render(ctx.plain);
        println!("{}", pad.render(&term));
        if self.example {
            print_example_command(PADLEFT_EXAMPLE_CMD);
        }
        Ok(())
    }
}

/// Pad text on the right (left-align) to a minimum width
#[derive(ClapArgs, Debug, Clone)]
pub struct PadRightArgs {
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(value_name = "WIDTH", required_unless_present = "example")]
    pub width: Option<u32>,

    #[arg(value_name = "TEXT", required_unless_present = "example")]
    pub text: Vec<String>,

    #[arg(long)]
    pub truncate: bool,
}

impl Run for PadRightArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        use biscuit_terminal::components::pad::PadRight;

        let width = self.width.unwrap_or(PADRIGHT_EXAMPLE_WIDTH);
        let content = if self.example {
            PADRIGHT_EXAMPLE_TEXT.to_string()
        } else {
            self.text.join(" ")
        };
        if content.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No content provided. Usage: bt padright 20 \"hello\""
            ));
        }

        let prose = Prose::new(&content);
        let mut pad = PadRight::new(prose, width);
        if self.truncate {
            pad = pad.truncate();
        }

        let term = terminal_for_render(ctx.plain);
        println!("{}", pad.render(&term));
        if self.example {
            print_example_command(PADRIGHT_EXAMPLE_CMD);
        }
        Ok(())
    }
}
