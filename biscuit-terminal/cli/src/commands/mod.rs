use color_eyre::Result;

pub mod about;
pub mod block;
pub mod color_parse;
pub mod columns;
pub mod compose;
pub mod dir;
pub mod erd;
pub mod flowchart;
pub mod git_graph;
pub mod graph;
pub mod image;
pub mod list;
pub mod mermaid;
pub mod pad;
pub mod pie;
pub mod progress;
pub mod prose;
pub mod quadrant;
pub mod quote;
pub mod section;
pub mod shared;
pub mod state_diagram;
pub mod status_block;
pub mod table;
pub mod text_block;
pub mod timeline;
pub mod todo;
pub mod xy_chart;

/// Shared context passed to every command's `run` method.
///
/// Holds global CLI flags that every subcommand may need to access.
pub struct CliContext {
    /// Whether `--json` was passed on the command line.
    pub json: bool,
    /// Whether `--plain` was passed on the command line.
    pub plain: bool,
}

/// Trait implemented by every subcommand args struct for dispatch.
///
/// The `main.rs` match block collapses to a single `cmd.run(&ctx)` call.
pub trait Run {
    fn run(self, ctx: &CliContext) -> Result<()>;
}
