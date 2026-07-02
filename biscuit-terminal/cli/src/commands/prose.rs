use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Alignment, Length, TargetValue, WordWrap};
use clap::Args as ClapArgs;
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;

const PROSE_EXAMPLE: &str = "<b>Deploy status:</b> <green>healthy</green> after _3 checks_";
const PROSE_EXAMPLE_CMD: &str =
    r#"bt prose "<b>Deploy status:</b> <green>healthy</green> after _3 checks_""#;

/// Render prose text with inline styling tags
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

        // Cross-target output: HTML fragment or portable Markdown. Terminal
        // capability detection does not apply to these targets; layout flags
        // are preserved as target-native metadata/CSS.
        if self.html {
            println!(
                "{}",
                render_html_with_layout(&prose.render_html_fragment().render(), &self.layout)
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

        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })?;

        if self.example {
            print_example_command(PROSE_EXAMPLE_CMD);
        }

        Ok(())
    }
}

/// Wraps a Prose HTML fragment in the layout `<div>`.
///
/// The CLI is the **single source of layout** for `bt prose --html`:
/// [`Prose::render_html_fragment`](biscuit_terminal::components::prose::Prose)
/// deliberately emits a layout-free inline `<span class="prose">`, so this
/// wrapper owns margins/alignment without double-applying them. This split is
/// interim — see the doc note on `Prose::render_html_fragment` and
/// `renderable/features/2026-06-04-style-based-alignment`: once layout lowers
/// to CSS via the `renderable` style primitives, revisit whether the fragment
/// should carry its own layout and this wrapper retire.
fn render_html_with_layout(fragment: &str, layout: &LayoutArgs) -> String {
    let Some(style) = layout_css_style(layout) else {
        return fragment.to_string();
    };
    format!("<div style=\"{style}\">{fragment}</div>")
}

fn layout_css_style(layout: &LayoutArgs) -> Option<String> {
    let mut declarations = Vec::new();

    if let Some(left) = layout.margin_left {
        declarations.push(format!("margin-left: {left}ch"));
    }
    if let Some(right) = layout.margin_right {
        declarations.push(format!("margin-right: {right}ch"));
    }
    if let Some(top) = layout.margin_top {
        declarations.push(format!("margin-top: {top}lh"));
    }
    if let Some(bottom) = layout.margin_bottom {
        declarations.push(format!("margin-bottom: {bottom}lh"));
    }
    if let Some(alignment) = layout.alignment {
        let text_align = match alignment {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
        };
        declarations.push(format!("text-align: {text_align}"));
    }

    (!declarations.is_empty()).then(|| declarations.join("; "))
}
