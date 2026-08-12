use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::list::{OrderedList, UnorderedList};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::utils::layout::{Length, TargetValue};
use clap::Args as ClapArgs;
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;
use std::rc::Rc;

const LIST_EXAMPLE: &[&str] = &[
    "<b>Plan</b> the change",
    "<green>Run</green> focused tests",
    "Ship the smallest useful fix",
];
const LIST_EXAMPLE_CMD: &str = r#"bt list "<b>Plan</b> the change" "<green>Run</green> focused tests" "Ship the smallest useful fix""#;

const ORDERED_LIST_EXAMPLE: &[&str] = &[
    "Install dependencies",
    "Run the test suite",
    "Deploy to staging",
];
const ORDERED_LIST_EXAMPLE_CMD: &str =
    r#"bt list --ordered "Install dependencies" "Run the test suite" "Deploy to staging""#;

/// Render a bulleted or numbered list with hanging indents.
///
/// Supports terminal output (the default), portable Markdown (`--md`),
/// MarkdownPlus (`--md-plus`), and an HTML fragment (`--html`). The
/// `--ordered` / `-o` flag switches from an unordered (bullet) list to an
/// ordered (numbered) list.
#[derive(ClapArgs, Debug, Clone)]
pub struct ListArgs {
    /// Render an example and show the command used.
    ///
    /// Combine with `--ordered` to render the numbered-list example.
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Positional list items.
    #[arg(value_name = "ITEMS", required_unless_present = "example")]
    pub items: Vec<String>,

    /// Render as an ordered (numbered) list instead of unordered.
    #[arg(long, short = 'o')]
    pub ordered: bool,

    /// Bullet character for unordered lists (ignored when `--ordered` is set).
    #[arg(long, short = 'b', default_value = "• ")]
    pub bullet: String,

    /// Disable hanging indent on wrapped lines (unordered lists only).
    #[arg(long)]
    pub no_hanging_indent: bool,

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

impl Run for ListArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let items: Vec<String> = if self.example {
            let source = if self.ordered {
                ORDERED_LIST_EXAMPLE
            } else {
                LIST_EXAMPLE
            };
            source.iter().map(|item| (*item).to_string()).collect()
        } else {
            self.items.clone()
        };

        if items.is_empty() {
            let usage = if self.ordered {
                "No items provided. Usage: bt list --ordered \"Step one\" \"Step two\" \"Step three\""
            } else {
                "No items provided. Usage: bt list \"First item\" \"Second item\" \"Third item\""
            };
            return Err(color_eyre::eyre::eyre!(usage));
        }

        let prose_items: Vec<RenderableTerminalContent> = items
            .iter()
            .map(|item| {
                let text = crate::types::unescape_shell_escapes(item);
                let prose = Prose::new(&text);
                RenderableTerminalContent::Component(Rc::new(prose))
            })
            .collect();

        // Route through the chosen target. Both OrderedList and
        // UnorderedList now implement TerminalRenderable, MarkdownRenderable,
        // and BrowserRenderable through the canonical render tree, so the
        // cross-target switches work uniformly across both list kinds.
        if self.ordered {
            let mut list = OrderedList::from(prose_items);
            apply_layout(&mut list, &self.layout);
            self.dispatch_ordered_render(&list, ctx.plain)?;
        } else {
            let mut list = UnorderedList::from(prose_items).with_bullet(&self.bullet);
            if self.no_hanging_indent {
                list = list.without_hanging_indent();
            }
            apply_layout(&mut list, &self.layout);
            self.dispatch_unordered_render(&list, ctx.plain)?;
        }

        if self.example {
            let cmd = if self.ordered {
                ORDERED_LIST_EXAMPLE_CMD
            } else {
                LIST_EXAMPLE_CMD
            };
            print_example_command(cmd);
        }

        Ok(())
    }
}

impl ListArgs {
    /// Dispatches an ordered-list render to the chosen target.
    ///
    /// [`OrderedList`] implements [`TerminalRenderable`],
    /// [`MarkdownRenderable`], and [`BrowserRenderable`] via the canonical
    /// render tree, so cross-target rendering is uniform.
    fn dispatch_ordered_render(&self, list: &OrderedList, plain: bool) -> color_eyre::Result<()> {
        if self.html {
            println!(
                "{}",
                render_html_with_layout(&list.render_html_fragment().render(), &self.layout)
            );
            return Ok(());
        }
        if self.md {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&list.render_markdown(), &self.layout)
            );
            return Ok(());
        }
        if self.md_plus {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&list.render_markdown_plus(), &self.layout)
            );
            return Ok(());
        }
        self.render_terminal(list, plain)
    }

    /// Dispatches an unordered-list render to the chosen target.
    ///
    /// [`UnorderedList`] implements [`TerminalRenderable`],
    /// [`MarkdownRenderable`], and [`BrowserRenderable`] via the canonical
    /// render tree, so cross-target rendering is uniform with the ordered
    /// path.
    ///
    /// A custom `--bullet` is honored on the terminal target only; the
    /// Markdown and Browser tree renderers emit standard `- ` markers and
    /// `<ul>`/`<li>` respectively regardless of the configured bullet.
    fn dispatch_unordered_render(
        &self,
        list: &UnorderedList,
        plain: bool,
    ) -> color_eyre::Result<()> {
        if self.html {
            println!(
                "{}",
                render_html_with_layout(&list.render_html_fragment().render(), &self.layout)
            );
            return Ok(());
        }
        if self.md {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&list.render_markdown(), &self.layout)
            );
            return Ok(());
        }
        if self.md_plus {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&list.render_markdown_plus(), &self.layout)
            );
            return Ok(());
        }
        self.render_terminal(list, plain)
    }

    fn render_terminal<L: TerminalRenderable>(&self, list: &L, plain: bool) -> color_eyre::Result<()> {
        let term = terminal_for_render(plain);
        let output = list.render(&term);
        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })
    }
}

/// Applies the shared `LayoutArgs` margin/alignment flags to a list component.
fn apply_layout<L: TerminalRenderable>(list: &mut L, layout: &LayoutArgs) {
    if let Some(left) = layout.margin_left {
        list.layout_mut().margin.left = TargetValue::universal(Length::ch(left));
    }
    if let Some(right) = layout.margin_right {
        list.layout_mut().margin.right = TargetValue::universal(Length::ch(right));
    }
    if let Some(align) = layout.alignment {
        list.layout_mut().alignment = align;
    }
}

/// Wraps the rendered HTML fragment in a `<div>` only when the CLI was given
/// layout properties the tree-path CSS lowering cannot express on the
/// component's own root element.
///
/// The tree renderer lowers `Layout` to CSS on the `<ol>` itself (see
/// `layout_to_css` in `renderable::tree::render::browser`). Margins (all four
/// sides) and `max_width`-driven alignment are emitted there directly. The
/// only LayoutArgs property without an `<ol>`-level peer is `--alignment`
/// without a `max_width`, which would need a `text-align` declaration on a
/// surrounding block. We wrap in that case only; otherwise we return the
/// fragment as-is to avoid double-application of properties (e.g. doubled
/// `margin-left`).
fn render_html_with_layout(fragment: &str, layout: &LayoutArgs) -> String {
    let Some(style) = wrapper_only_css(layout) else {
        return fragment.to_string();
    };
    format!("<div style=\"{style}\">{fragment}</div>")
}

/// Returns the CSS declarations that the tree-path layout lowering cannot
/// express on the component's own element. Today that is `text-align` from
/// `--alignment` when no `max_width` is in play. Margins are always
/// expressible by the tree path and therefore deliberately omitted here.
fn wrapper_only_css(layout: &LayoutArgs) -> Option<String> {
    use biscuit_terminal::utils::layout::Alignment;

    let alignment = layout.alignment?;
    let text_align = match alignment {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
    };
    Some(format!("text-align: {text_align}"))
}
