use crate::args::LayoutArgs;
use crate::commands::shared::{
    apply_renderable_layout, emit_vertical_margins, print_example_command,
    render_markdown_with_layout_frontmatter, terminal_for_render,
};
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::todo::{Todo, TodoState};
use clap::Args as ClapArgs;
use renderable::markdown::MarkdownRenderable;

const TODO_EXAMPLE_DESCRIPTION: &str = "Review pull request #42";
const TODO_EXAMPLE_CMD: &str = r#"bt todo "Review pull request #42" --state completed"#;

/// Five visible states a [`Todo`] item can be in.
///
/// Mirrors the public [`TodoState`] variants. Markdown and Browser degrade
/// `InProgress`, `Blocked`, and `Cancelled` to unchecked task items, while
/// `Completed` becomes `- [x]` / a `checked` checkbox. The terminal target
/// preserves all five states via state-aware Nerd Font / colored / no-color
/// markers.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum TodoStateArg {
    Open,
    #[clap(name = "in-progress")]
    InProgress,
    Completed,
    Blocked,
    Cancelled,
}

impl From<TodoStateArg> for TodoState {
    fn from(value: TodoStateArg) -> Self {
        match value {
            TodoStateArg::Open => TodoState::Open,
            TodoStateArg::InProgress => TodoState::InProgress,
            TodoStateArg::Completed => TodoState::Completed,
            TodoStateArg::Blocked => TodoState::Blocked,
            TodoStateArg::Cancelled => TodoState::Cancelled,
        }
    }
}

/// Render a single Todo item through the canonical render tree.
///
/// The default target is the terminal. Switch to a different target with one
/// of the mutually-exclusive `--html`, `--md`, or `--md-plus` flags. The five
/// task states degrade to a portable GFM `- [ ]` / `- [x]` shape on
/// Markdown/Browser; the terminal target preserves all five via
/// state-specific markers (Nerd Font glyphs, colored ASCII fallbacks, or
/// no-color ASCII fallbacks).
#[derive(ClapArgs, Debug, Clone)]
pub struct TodoArgs {
    /// Render an example and show the command used.
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Description text for the todo item. Multiple positional values are
    /// joined with spaces.
    #[arg(value_name = "DESCRIPTION", required_unless_present = "example")]
    pub description: Vec<String>,

    /// State of the todo item.
    #[arg(long, value_enum, default_value = "open")]
    pub state: TodoStateArg,

    /// Interpret the description as Prose markup (e.g. `<b>bold</b>`).
    ///
    /// Inline styling is preserved on the terminal target's bespoke path but
    /// is flattened to plain text on the canonical tree path (the same lossy
    /// projection used by BlockQuote and Compose for inline content sources).
    #[arg(long)]
    pub prose: bool,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus instead of the terminal.
    ///
    /// Identical to `--md` for `Todo` — every state degrades to GFM
    /// task-list syntax (`- [ ]` / `- [x]`), and `Cancelled` uses `~~text~~`
    /// strikethrough which is valid in both dialects.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,

    /// Shared layout arguments.
    ///
    /// `--margin-left`, `--margin-right`, and `--alignment` are lowered
    /// through the canonical render tree and therefore affect every target.
    /// `--margin-top` and `--margin-bottom` only affect terminal output —
    /// the cross-target branches (`--html`, `--md`, `--md-plus`) return
    /// before [`emit_vertical_margins`] runs.
    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for TodoArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let description = if self.example && self.description.is_empty() {
            TODO_EXAMPLE_DESCRIPTION.to_string()
        } else {
            self.description.join(" ")
        };

        // `--example` with no explicit `--state` defaults to a representative
        // Completed item so the rendered output matches the printed command.
        let state: TodoState = if self.example {
            match self.state {
                TodoStateArg::Open => TodoState::Completed,
                other => other.into(),
            }
        } else {
            self.state.into()
        };

        let mut todo = if self.prose {
            Todo::from_prose(description)
        } else {
            Todo::new(description)
        }
        .with_state(state);

        apply_renderable_layout(&mut todo, &self.layout);

        if self.html {
            println!("{}", todo.render_html_fragment().render());
            if self.example {
                print_example_command(TODO_EXAMPLE_CMD);
            }
            return Ok(());
        }
        if self.md {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&todo.render_markdown(), &self.layout)
            );
            if self.example {
                print_example_command(TODO_EXAMPLE_CMD);
            }
            return Ok(());
        }
        if self.md_plus {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&todo.render_markdown_plus(), &self.layout)
            );
            if self.example {
                print_example_command(TODO_EXAMPLE_CMD);
            }
            return Ok(());
        }

        // Terminal (default).
        let term = terminal_for_render(ctx.plain);
        let output = todo.render(&term);
        emit_vertical_margins(&self.layout, || {
            println!("{}", output);
            Ok(())
        })?;

        if self.example {
            print_example_command(TODO_EXAMPLE_CMD);
        }
        Ok(())
    }
}
