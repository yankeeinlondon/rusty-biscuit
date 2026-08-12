use crate::args::LayoutArgs;
use crate::commands::shared::{
    apply_renderable_layout, emit_vertical_margins, print_example_command,
    render_markdown_with_layout_frontmatter, terminal_for_render,
};
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::utils::layout::Alignment;
use clap::Args as ClapArgs;
use renderable::markdown::MarkdownRenderable;

const SECTION_EXAMPLE_TITLE: &str = "Deployment Guide";
const SECTION_EXAMPLE_BODY: &[&str] =
    &["Follow these steps to deploy.", "Verify the build passes."];
const SECTION_EXAMPLE_CMD: &str = r#"bt section "Deployment Guide" --level 2 --content "Follow these steps to deploy." --content "Verify the build passes.""#;

/// Render a Markdown-style section (heading + body) through the canonical
/// render tree.
///
/// The default target is the terminal. Switch to a different render target
/// with one of the mutually-exclusive `--html`, `--md`, or `--md-plus` flags.
#[derive(ClapArgs, Debug, Clone)]
pub struct SectionArgs {
    /// Render an example and show the command used.
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Section heading text.
    #[arg(value_name = "TITLE", required_unless_present = "example")]
    pub title: Option<String>,

    /// Heading level (1-6).
    #[arg(long, short = 'l', default_value_t = 2)]
    pub level: u8,

    /// Body content items. Repeat for multiple items.
    #[arg(long, short = 'c')]
    pub content: Vec<String>,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus (inline HTML allowed) instead of the terminal.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,

    /// Shared layout arguments.
    ///
    /// `--margin-left`, `--margin-right`, and `--alignment` are lowered through
    /// the canonical render tree and therefore affect every target (Terminal,
    /// HTML, Markdown via frontmatter). `--margin-top` and `--margin-bottom`
    /// only affect terminal output: the cross-target branches (`--html`,
    /// `--md`, `--md-plus`) `return Ok(())` before
    /// [`emit_vertical_margins`][crate::commands::shared::emit_vertical_margins]
    /// runs, so vertical-margin flags are silently ignored on those targets.
    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for SectionArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let title = if self.example {
            SECTION_EXAMPLE_TITLE.to_string()
        } else {
            self.title.clone().ok_or_else(|| {
                color_eyre::eyre::eyre!(
                    "Title is required. Usage: bt section \"My Title\" --content \"Body text\""
                )
            })?
        };

        let level = heading_level_from_u8(self.level)?;

        let content: Vec<String> = if self.example && self.content.is_empty() {
            SECTION_EXAMPLE_BODY
                .iter()
                .map(|s| (*s).to_string())
                .collect()
        } else {
            self.content.clone()
        };

        let mut section = Section::new(level, &title);
        for item in &content {
            section.push(item.as_str());
        }

        apply_renderable_layout(&mut section, &self.layout);

        if self.html {
            println!(
                "{}",
                render_html_with_layout(&section.render_html_fragment().render(), &self.layout)
            );
            if self.example {
                print_example_command(SECTION_EXAMPLE_CMD);
            }
            return Ok(());
        }
        if self.md {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&section.render_markdown(), &self.layout)
            );
            if self.example {
                print_example_command(SECTION_EXAMPLE_CMD);
            }
            return Ok(());
        }
        if self.md_plus {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(
                    &section.render_markdown_plus(),
                    &self.layout
                )
            );
            if self.example {
                print_example_command(SECTION_EXAMPLE_CMD);
            }
            return Ok(());
        }

        // Terminal (default).
        let term = terminal_for_render(ctx.plain);
        let output = section.render(&term);
        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })?;

        if self.example {
            print_example_command(SECTION_EXAMPLE_CMD);
        }
        Ok(())
    }
}

/// Maps a `1..=6` integer onto the matching [`HeadingLevel`] variant.
fn heading_level_from_u8(level: u8) -> color_eyre::Result<HeadingLevel> {
    match level {
        1 => Ok(HeadingLevel::h1),
        2 => Ok(HeadingLevel::h2),
        3 => Ok(HeadingLevel::h3),
        4 => Ok(HeadingLevel::h4),
        5 => Ok(HeadingLevel::h5),
        6 => Ok(HeadingLevel::h6),
        other => Err(color_eyre::eyre::eyre!(
            "Heading level must be 1-6, got {other}"
        )),
    }
}

/// Wraps the rendered HTML fragment in a `<div>` only when the CLI was given
/// layout properties the tree-path CSS lowering cannot express on the
/// component's own root element.
///
/// The tree renderer lowers `Layout` to CSS on the `<section>` itself. Margins
/// (all four sides) and `max_width`-driven alignment are emitted there
/// directly. The only `LayoutArgs` property without a `<section>`-level peer
/// is `--alignment` without a `max_width`, which needs a `text-align`
/// declaration on a surrounding block.
fn render_html_with_layout(fragment: &str, layout: &LayoutArgs) -> String {
    let Some(style) = wrapper_only_css(layout) else {
        return fragment.to_string();
    };
    format!("<div style=\"{style}\">{fragment}</div>")
}

/// Returns the CSS declarations that the tree-path layout lowering cannot
/// express on the component's own element. Today that is `text-align` from
/// `--alignment`. Margins are always expressible by the tree path and
/// therefore deliberately omitted here to avoid double-application.
fn wrapper_only_css(layout: &LayoutArgs) -> Option<String> {
    let alignment = layout.alignment?;
    let text_align = match alignment {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
    };
    Some(format!("text-align: {text_align}"))
}
