use crate::args::LayoutArgs;
use crate::commands::color_parse::parse_color;
use crate::commands::shared::{
    apply_renderable_layout, emit_vertical_margins, print_example_command,
    render_markdown_with_layout_frontmatter, terminal_for_render,
};
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::text_block::TextBlock;
use biscuit_terminal::utils::color::Color;
use biscuit_terminal::utils::styling::UnderliningRequest;
use clap::Args as ClapArgs;
use renderable::markdown::MarkdownRenderable;

const TEXT_BLOCK_EXAMPLE: &str = "Release candidate passed";
const TEXT_BLOCK_EXAMPLE_CMD: &str =
    r#"bt text-block "Release candidate passed" --fg green --bold --underline"#;

/// Render a uniformly-styled text block through the canonical render tree.
///
/// `bt text-block` instantiates the actual [`TextBlock`] component and
/// exercises its migration path through the render tree. The default target
/// is the terminal; switch to a different render target with one of the
/// mutually-exclusive `--html`, `--md`, or `--md-plus` flags.
///
/// This subcommand is intentionally separate from `bt block` — `bt block`
/// is a generic render-tree style exerciser and includes box concepts
/// (border, fill) that `TextBlock` does not expose. `bt text-block` is the
/// component-parity surface.
#[derive(ClapArgs, Debug, Clone)]
pub struct TextBlockArgs {
    /// Render an example and show the command used.
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Text content; multiple values are joined with spaces.
    #[arg(value_name = "TEXT", required_unless_present = "example")]
    pub text: Vec<String>,

    /// Bold text.
    #[arg(long)]
    pub bold: bool,

    /// Dim text.
    #[arg(long, conflicts_with = "bold")]
    pub dim: bool,

    /// Italic text.
    #[arg(long)]
    pub italic: bool,

    /// Strikethrough text.
    #[arg(long, visible_alias = "strike")]
    pub strikethrough: bool,

    /// Straight underline.
    #[arg(long, conflicts_with_all = [
        "double_underline", "curly_underline", "dotted_underline", "dashed_underline",
    ])]
    pub underline: bool,

    /// Double underline.
    #[arg(long = "double-underline", conflicts_with_all = [
        "underline", "curly_underline", "dotted_underline", "dashed_underline",
    ])]
    pub double_underline: bool,

    /// Curly (wavy) underline.
    #[arg(long = "curly-underline", conflicts_with_all = [
        "underline", "double_underline", "dotted_underline", "dashed_underline",
    ])]
    pub curly_underline: bool,

    /// Dotted underline.
    #[arg(long = "dotted-underline", conflicts_with_all = [
        "underline", "double_underline", "curly_underline", "dashed_underline",
    ])]
    pub dotted_underline: bool,

    /// Dashed underline.
    #[arg(long = "dashed-underline", conflicts_with_all = [
        "underline", "double_underline", "curly_underline", "dotted_underline",
    ])]
    pub dashed_underline: bool,

    /// Blinking text.
    #[arg(long)]
    pub blink: bool,

    /// Foreground (text) color (named or `#rrggbb`).
    #[arg(long)]
    pub fg: Option<String>,

    /// Background color (named or `#rrggbb`).
    #[arg(long)]
    pub bg: Option<String>,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus instead of the terminal.
    ///
    /// Currently produces the same plain text as `--md` for styled
    /// `TextBlock` content. Styled MarkdownPlus is a renderer-wide feature
    /// deferred outside this migration (see spec RT-TEXTBLOCK-002, DENIED).
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,

    /// Shared layout arguments.
    ///
    /// `--margin-left`, `--margin-right`, and `--alignment` are lowered
    /// through the canonical render tree and therefore affect every target
    /// (Terminal, HTML, Markdown via frontmatter). `--margin-top` and
    /// `--margin-bottom` only affect terminal output: the cross-target
    /// branches `return Ok(())` before [`emit_vertical_margins`] runs.
    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl TextBlockArgs {
    /// Picks the [`UnderliningRequest`] variant from the mutually-exclusive
    /// underline flags. The component currently does not expose underline
    /// color, so each variant carries `None`.
    fn underline_request(&self) -> UnderliningRequest {
        if self.underline {
            UnderliningRequest::Straight(None)
        } else if self.double_underline {
            UnderliningRequest::Double(None)
        } else if self.curly_underline {
            UnderliningRequest::Curly(None)
        } else if self.dotted_underline {
            UnderliningRequest::Dotted(None)
        } else if self.dashed_underline {
            UnderliningRequest::Dashed(None)
        } else {
            UnderliningRequest::None
        }
    }

    /// Builds the configured [`TextBlock`] from the parsed flags.
    fn build_text_block(&self, text: &str) -> color_eyre::Result<TextBlock> {
        let mut block = TextBlock::new(text);
        if self.bold {
            block.using_bold_text();
        }
        if self.dim {
            block.using_dim_text();
        }
        if self.italic {
            block.using_italics();
        }
        if self.strikethrough {
            block.use_strikethrough_on_content();
        }
        if self.blink {
            block.make_content_blink();
        }
        let underline = self.underline_request();
        if !matches!(underline, UnderliningRequest::None) {
            block.with_underline(underline);
        }
        if let Some(fg) = &self.fg {
            let c: Color = parse_color(fg)?;
            block.with_foreground_color(c);
        }
        if let Some(bg) = &self.bg {
            let c: Color = parse_color(bg)?;
            block.with_background_color(c);
        }
        apply_renderable_layout(&mut block, &self.layout);
        Ok(block)
    }
}

impl Run for TextBlockArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let text = if self.example {
            TEXT_BLOCK_EXAMPLE.to_string()
        } else {
            self.text.join(" ")
        };

        // For `--example`, force a representative style set so the rendered
        // output matches the documented command.
        //
        // `--dim` wins over the example's implicit `bold = true` so the
        // in-memory state never advertises two mutually-exclusive font weights
        // (the clap-level `conflicts_with` only fires when both come from the
        // command line; the example knob bypasses that gate).
        let mut effective = self.clone();
        if self.example {
            effective.fg.get_or_insert_with(|| "green".to_string());
            if !self.dim {
                effective.bold = true;
            }
            if !effective.underline
                && !effective.double_underline
                && !effective.curly_underline
                && !effective.dotted_underline
                && !effective.dashed_underline
            {
                effective.underline = true;
            }
        }

        let block = effective.build_text_block(&text)?;

        if self.html {
            println!("{}", block.render_html_fragment().render());
            if self.example {
                print_example_command(TEXT_BLOCK_EXAMPLE_CMD);
            }
            return Ok(());
        }
        if self.md {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&block.render_markdown(), &self.layout)
            );
            if self.example {
                print_example_command(TEXT_BLOCK_EXAMPLE_CMD);
            }
            return Ok(());
        }
        if self.md_plus {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(
                    &block.render_markdown_plus(),
                    &self.layout
                )
            );
            if self.example {
                print_example_command(TEXT_BLOCK_EXAMPLE_CMD);
            }
            return Ok(());
        }

        // Terminal (default).
        let term = terminal_for_render(ctx.plain);
        let output = block.render(&term);
        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })?;

        if self.example {
            print_example_command(TEXT_BLOCK_EXAMPLE_CMD);
        }
        Ok(())
    }
}
