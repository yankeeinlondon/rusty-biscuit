use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Length, TargetValue, WordWrap};
use clap::Args as ClapArgs;
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;

/// Render prose text with inline styling tags
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

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with = "md")]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with = "html")]
    pub md: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for ProseArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let text = self.content.join(" ");

        if text.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No content provided. Usage: bt prose \"Hello <bold>world</bold>!\""
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
            prose = prose.with_left_margin(TargetValue::universal(Length::ch(left)));
        }
        if let Some(right) = self.layout.margin_right {
            prose = prose.with_right_margin(TargetValue::universal(Length::ch(right)));
        }
        if let Some(align) = self.layout.alignment {
            prose = prose.alignment(align);
        }

        // Cross-target output: HTML fragment or portable Markdown. Layout
        // and terminal capability detection do not apply to these targets.
        if self.html {
            println!("{}", prose.render_html_fragment().render());
            return Ok(());
        }
        if self.md {
            println!("{}", prose.render_markdown());
            return Ok(());
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
