use color_eyre::Result;

pub mod block;
pub mod color_parse;
pub mod columns;
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
pub mod shared;
pub mod state_diagram;
pub mod table;
pub mod timeline;
pub mod xy_chart;

/// Shared context passed to every command's `run` method.
///
/// Holds global CLI flags that every subcommand may need to access.
pub struct CliContext {
    /// Whether `--json` was passed on the command line.
    pub json: bool,
}

/// Trait implemented by every subcommand args struct for dispatch.
///
/// The `main.rs` match block collapses to a single `cmd.run(&ctx)` call.
pub trait Run {
    fn run(self, ctx: &CliContext) -> Result<()>;
}
