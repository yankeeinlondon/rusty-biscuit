use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::pad::PadLeft;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use clap::Args as ClapArgs;

/// Pad text on the left (right-align) to a minimum width
#[derive(ClapArgs, Debug, Clone)]
pub struct PadLeftArgs {
    #[arg(value_name = "WIDTH")]
    pub width: u32,

    #[arg(value_name = "TEXT")]
    pub text: Vec<String>,

    #[arg(long)]
    pub truncate: bool,
}

impl Run for PadLeftArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let content = self.text.join(" ");
        if content.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No content provided. Usage: bt padleft 20 \"hello\""
            ));
        }

        let prose = Prose::new(&content);
        let mut pad = PadLeft::new(prose, self.width);
        if self.truncate {
            pad = pad.truncate();
        }

        let term = detect_terminal_honoring_force_color();
        println!("{}", pad.render(&term));
        Ok(())
    }
}

/// Pad text on the right (left-align) to a minimum width
#[derive(ClapArgs, Debug, Clone)]
pub struct PadRightArgs {
    #[arg(value_name = "WIDTH")]
    pub width: u32,

    #[arg(value_name = "TEXT")]
    pub text: Vec<String>,

    #[arg(long)]
    pub truncate: bool,
}

impl Run for PadRightArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        use biscuit_terminal::components::pad::PadRight;

        let content = self.text.join(" ");
        if content.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No content provided. Usage: bt padright 20 \"hello\""
            ));
        }

        let prose = Prose::new(&content);
        let mut pad = PadRight::new(prose, self.width);
        if self.truncate {
            pad = pad.truncate();
        }

        let term = detect_terminal_honoring_force_color();
        println!("{}", pad.render(&term));
        Ok(())
    }
}
