use crate::args::LayoutArgs;
use crate::commands::color_parse::parse_color;
use crate::commands::shared::{
    apply_renderable_layout, emit_vertical_margins, print_example_command,
    render_markdown_with_layout_frontmatter, terminal_for_render,
};
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use clap::Args as ClapArgs;
use renderable::markdown::MarkdownRenderable;

const STATUS_BLOCK_EXAMPLE_HEADER: &str = "<b>Shell Expansion Failed</b>";
const STATUS_BLOCK_EXAMPLE_BODY: &str = "Missing closing brace in `${...}` directive.";
const STATUS_BLOCK_EXAMPLE_HINT: &str = "Check the template syntax and retry.";
const STATUS_BLOCK_EXAMPLE_CMD: &str = r#"bt status-block --severity error --header "<b>Shell Expansion Failed</b>" --hint "Check the template syntax and retry." "Missing closing brace in \`${...}\` directive.""#;

/// Severity values accepted by the CLI.
///
/// Mirrors the public [`StatusState`] variants except for the deprecated
/// `Failure` alias, which is intentionally not exposed on new CLI surfaces.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum StatusSeverityArg {
    Error,
    Warning,
    Info,
    Success,
    Active,
    #[clap(name = "not-started")]
    NotStarted,
    #[clap(name = "tool-use")]
    ToolUse,
    Subagent,
}

impl From<StatusSeverityArg> for StatusState {
    fn from(value: StatusSeverityArg) -> Self {
        match value {
            StatusSeverityArg::Error => StatusState::Error,
            StatusSeverityArg::Warning => StatusState::Warning,
            StatusSeverityArg::Info => StatusState::Info,
            StatusSeverityArg::Success => StatusState::Success,
            StatusSeverityArg::Active => StatusState::Active,
            StatusSeverityArg::NotStarted => StatusState::NotStarted,
            StatusSeverityArg::ToolUse => StatusState::ToolUse,
            StatusSeverityArg::Subagent => StatusState::Subagent,
        }
    }
}

/// Render a severity-colored StatusBlock (header + body + hint) through the
/// canonical render tree.
///
/// The default target is the terminal. Switch to a different render target
/// with one of the mutually-exclusive `--html` or `--md` flags. The CLI does
/// not expose the arbitrary `border(String)` knob — that is a terminal-only
/// compatibility surface and is not portable across `--md` and `--html`.
#[derive(ClapArgs, Debug, Clone)]
pub struct StatusBlockArgs {
    /// Render an example status block and show the command used.
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Severity (error, warning, info, success, active, not-started,
    /// tool-use, subagent).
    #[arg(long, value_enum, required_unless_present = "example")]
    pub severity: Option<StatusSeverityArg>,

    /// Optional header line (prose-formatted).
    #[arg(long)]
    pub header: Option<String>,

    /// Optional hint line shown below the body block quote.
    #[arg(long)]
    pub hint: Option<String>,

    /// Override the severity-derived border color (named or `#rrggbb`).
    #[arg(long = "border-color")]
    pub border_color: Option<String>,

    /// Body text. Multiple positional values are joined with spaces and used
    /// as a single body Prose item.
    #[arg(value_name = "BODY", required_unless_present = "example")]
    pub body: Vec<String>,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with = "html")]
    pub md: bool,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with = "md")]
    pub html: bool,

    /// Shared layout arguments.
    ///
    /// `--margin-left`, `--margin-right`, and `--alignment` are lowered
    /// through the canonical render tree and therefore affect every target.
    /// `--margin-top` and `--margin-bottom` only affect terminal output: the
    /// cross-target branches (`--html`, `--md`) `return Ok(())` before
    /// [`emit_vertical_margins`][crate::commands::shared::emit_vertical_margins]
    /// runs, so vertical-margin flags are silently ignored on those targets.
    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for StatusBlockArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        // Resolve severity. `required_unless_present = "example"` guarantees
        // it is present when `--example` is not set.
        let severity: StatusState = if self.example {
            self.severity
                .clone()
                .map(Into::into)
                .unwrap_or(StatusState::Error)
        } else {
            self.severity
                .clone()
                .ok_or_else(|| {
                    color_eyre::eyre::eyre!(
                        "--severity is required. Usage: bt status-block --severity error \"body text\""
                    )
                })?
                .into()
        };

        // Resolve body, header, hint — fall back to canned example values
        // only when `--example` is set.
        let body: Vec<Prose> = if self.example && self.body.is_empty() {
            vec![Prose::new(STATUS_BLOCK_EXAMPLE_BODY)]
        } else {
            // Multiple positional values are joined into a single body line.
            if self.body.is_empty() {
                return Err(color_eyre::eyre::eyre!(
                    "Body text is required. Usage: bt status-block --severity error \"body text\""
                ));
            }
            vec![Prose::new(self.body.join(" "))]
        };

        let header = self.header.clone().or_else(|| {
            self.example
                .then(|| STATUS_BLOCK_EXAMPLE_HEADER.to_string())
        });
        let hint = self
            .hint
            .clone()
            .or_else(|| self.example.then(|| STATUS_BLOCK_EXAMPLE_HINT.to_string()));

        let mut block = StatusBlock::new(severity).body(body);
        if let Some(header_text) = header {
            block = block.header(header_text);
        }
        if let Some(hint_text) = hint {
            block = block.hint(hint_text);
        }
        if let Some(color_str) = &self.border_color {
            let color = parse_color(color_str)?;
            block = block.border_color(color);
        }

        apply_renderable_layout(&mut block, &self.layout);

        if self.html {
            println!("{}", block.render_html_fragment().render());
            if self.example {
                print_example_command(STATUS_BLOCK_EXAMPLE_CMD);
            }
            return Ok(());
        }
        if self.md {
            println!(
                "{}",
                render_markdown_with_layout_frontmatter(&block.render_markdown(), &self.layout)
            );
            if self.example {
                print_example_command(STATUS_BLOCK_EXAMPLE_CMD);
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
            print_example_command(STATUS_BLOCK_EXAMPLE_CMD);
        }
        Ok(())
    }
}
