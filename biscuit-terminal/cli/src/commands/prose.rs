use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Margin, WordWrap};
use clap::Args as ClapArgs;

/// Render prose text with inline styling tokens
#[derive(ClapArgs, Debug, Clone)]
pub struct ProseArgs {
    #[arg(value_name = "CONTENT")]
    pub content: Vec<String>,

    #[arg(long)]
    pub no_wrap: bool,

    #[arg(long)]
    pub force_color: bool,

    #[arg(long = "print-bytes")]
    pub print_bytes: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for ProseArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let text = self.content.join(" ");

        if text.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No content provided. Usage: bt prose \"Hello {{bold}}world{{reset}}!\""
            ));
        }

        let text = crate::types::unescape_shell_escapes(&text);

        let mut prose = biscuit_terminal::components::prose::Prose::new(&text);

        if self.no_wrap {
            prose = prose.with_word_wrap(WordWrap::None);
        } else {
            prose = prose.with_word_wrap(WordWrap::WrapProse(None, None));
        }

        if let Some(left) = self.layout.margin_left {
            prose = prose.with_left_margin(Margin::Chars(left));
        }
        if let Some(right) = self.layout.margin_right {
            prose = prose.with_right_margin(Margin::Chars(right));
        }
        if let Some(align) = self.layout.alignment {
            prose = prose.alignment(align);
        }

        let term = if self.force_color {
            Terminal::new_forced()
        } else {
            detect_terminal_honoring_force_color()
        };
        let output = prose.render(&term);

        let output = if std::env::var("NO_COLOR").is_ok() {
            strip_sgr_sequences(&output)
        } else {
            output
        };

        if self.print_bytes {
            eprintln!("--- prose debug ---");
            let mut hex = String::with_capacity(output.len() * 2);
            for byte in output.as_bytes() {
                use std::fmt::Write as _;
                let _ = write!(hex, "{byte:02x}");
            }
            eprintln!("{hex}");
        }

        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })
    }
}
