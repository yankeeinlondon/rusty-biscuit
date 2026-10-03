use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Length, TargetValue, WordWrap};
use clap::Args as ClapArgs;
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;

const PROSE_EXAMPLE: &str = "<b>Deploy status:</b> <green>healthy</green> after _3 checks_";
const PROSE_EXAMPLE_CMD: &str =
    r#"bt prose "<b>Deploy status:</b> <green>healthy</green> after _3 checks_""#;

/// Render prose text with inline styling tags
///
/// CONTENT is block prose: a blank line starts a new paragraph, a single
/// newline is a soft break (it reflows as a space), and a `\` before a newline
/// is a hard break. Backtick code spans render as inline code, and fenced
/// code blocks as code blocks. Layout flags apply on every target: margins as
/// spaces and blank lines on the terminal and as CSS with `--html`;
/// `--md`/`--md-plus` carry the left and right margins as `style:`
/// frontmatter.
#[derive(ClapArgs, Debug, Clone)]
pub struct ProseArgs {
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(value_name = "CONTENT", required_unless_present = "example")]
    pub content: Vec<String>,

    #[arg(long)]
    pub no_wrap: bool,

    #[arg(long)]
    pub force_color: bool,

    #[arg(long = "print-bytes")]
    pub print_bytes: bool,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus instead of the terminal.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for ProseArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let text = if self.example {
            PROSE_EXAMPLE.to_string()
        } else {
            self.content.join(" ")
        };

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
        // Vertical margins are rows on the terminal and `lh` in HTML;
        // Markdown has no vertical spacing and drops them.
        if let Some(top) = self.layout.margin_top {
            prose.layout_mut().margin.top = TargetValue::universal(Length::ch(top));
        }
        if let Some(bottom) = self.layout.margin_bottom {
            prose.layout_mut().margin.bottom = TargetValue::universal(Length::ch(bottom));
        }

        // Cross-target output: HTML fragment or portable Markdown. Terminal
        // capability detection does not apply to these targets. `Prose`
        // renders its own layout as CSS (alignment aside, see
        // `render_html_with_alignment`); Markdown carries the horizontal
        // margins as `style:` frontmatter.
        if self.html {
            println!(
                "{}",
                render_html_with_alignment(&prose.render_html_fragment().render(), &self.layout)
            );
            return Ok(());
        }
        if self.md {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&prose.render_markdown(), &self.layout)
            );
            return Ok(());
        }
        if self.md_plus {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(
                    &prose.render_markdown_plus(),
                    &self.layout
                )
            );
            return Ok(());
        }

        let term = if ctx.plain {
            terminal_for_render(true)
        } else if self.force_color {
            Terminal::new_forced()
        } else {
            terminal_for_render(false)
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

        println!("{}", output);

        if self.example {
            print_example_command(PROSE_EXAMPLE_CMD);
        }

        Ok(())
    }
}
