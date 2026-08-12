use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::render_tree::{
    BrowserTreeComponent, TerminalRenderOptions, render_terminal_node,
};
use biscuit_terminal::utils::layout::{Length, TargetValue};
use clap::Args as ClapArgs;
use renderable::browser::BrowserRenderable;
use renderable::tree::render::{MarkdownRenderOptions, render_markdown_node};
use renderable::tree::{RenderNode, RenderStrictness, TreeRenderable};
use std::rc::Rc;

const QUOTE_EXAMPLE: &str = "<b>Clarity</b> is kind when the work gets complex.";
const QUOTE_EXAMPLE_ATTRIBUTION: &str = "Engineering Notes";
const QUOTE_EXAMPLE_CMD: &str = r#"bt quote --attribution "Engineering Notes" "<b>Clarity</b> is kind when the work gets complex.""#;
const QUOTE_EXAMPLE_MD_CMD: &str = r#"bt quote --md --attribution "Engineering Notes" "<b>Clarity</b> is kind when the work gets complex.""#;
const QUOTE_EXAMPLE_HTML_CMD: &str = r#"bt quote --html --attribution "Engineering Notes" "<b>Clarity</b> is kind when the work gets complex.""#;

/// Render styled text in a block quote.
///
/// By default the quote renders for the terminal through the canonical
/// `renderable` tree renderer. Pass `--md` to emit portable Markdown or
/// `--html` to emit an HTML fragment instead. The two output-target flags
/// are mutually exclusive.
#[derive(ClapArgs, Debug, Clone)]
pub struct QuoteArgs {
    /// Render an example and show the command used.
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Quote content (positional, joined with spaces).
    #[arg(value_name = "CONTENT", required_unless_present = "example")]
    pub content: Vec<String>,

    /// Optional attribution (rendered as `— ATTRIBUTION`).
    #[arg(long)]
    pub attribution: Option<String>,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with = "html")]
    pub md: bool,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with = "md")]
    pub html: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for QuoteArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
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

        if self.md {
            let root = RenderNode::root(vec![quote.render_tree()]);
            let rendered = render_markdown_node(&root, &MarkdownRenderOptions::default())
                .map_err(|e| color_eyre::eyre::eyre!("markdown render failed: {e}"))?;
            println!("{}", rendered.output);
            if self.example {
                print_example_command(QUOTE_EXAMPLE_MD_CMD);
            }
            return Ok(());
        }

        if self.html {
            let html = BrowserTreeComponent::new(quote)
                .render_html_fragment()
                .render();
            println!("{html}");
            if self.example {
                print_example_command(QUOTE_EXAMPLE_HTML_CMD);
            }
            return Ok(());
        }

        // Default: render for the terminal through the canonical render tree.
        let term = terminal_for_render(ctx.plain);
        let root = RenderNode::root(vec![quote.render_tree()]);
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let rendered = render_terminal_node(&root, &opts)
            .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;

        emit_vertical_margins(&self.layout, || {
            println!("{}", rendered.output);
            Ok(())
        })?;

        if self.example {
            print_example_command(QUOTE_EXAMPLE_CMD);
        }

        Ok(())
    }
}
