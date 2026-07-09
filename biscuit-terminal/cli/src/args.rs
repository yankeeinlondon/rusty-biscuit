use crate::commands::*;
use biscuit_terminal::utils::layout;
use clap::{Args as ClapArgs, Parser, Subcommand};

/// Shell type for completion script generation.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ShellType {
    Bash,
    Elvish,
    Fish,
    #[value(alias = "pwsh")]
    Powershell,
    Zsh,
}

/// Shared layout arguments for image and diagram commands.
#[derive(ClapArgs, Debug, Clone, Default)]
pub struct LayoutArgs {
    /// Left margin in characters
    #[arg(long, visible_alias = "ml")]
    pub margin_left: Option<u32>,

    /// Right margin in characters
    #[arg(long, visible_alias = "mr")]
    pub margin_right: Option<u32>,

    /// Top margin in blank lines
    #[arg(long, visible_alias = "mt")]
    pub margin_top: Option<u32>,

    /// Bottom margin in blank lines
    #[arg(long, visible_alias = "mb")]
    pub margin_bottom: Option<u32>,

    /// Alignment
    #[arg(long, visible_alias = "align", value_enum)]
    pub alignment: Option<layout::Alignment>,
}

/// Terminal information utility
#[derive(Parser, Debug)]
#[command(name = "bt")]
#[command(author, version, about = "Display terminal metadata and capabilities")]
#[command(disable_help_subcommand = true)]
#[command(
    after_help = "\nSHELL COMPLETIONS:\n  Two methods are available:\n\n  DYNAMIC (recommended, includes image file filtering):\n    # Bash\n    echo 'source <(COMPLETE=bash bt)' >> ~/.bashrc\n\n    # Zsh\n    echo 'source <(COMPLETE=zsh bt)' >> ~/.zshrc\n\n    # Fish\n    echo 'COMPLETE=fish bt | source' >> ~/.config/fish/config.fish\n\n  STATIC (generates a completion script):\n    # Bash\n    bt --completions bash >> ~/.bashrc\n\n    # Zsh (ensure fpath includes the directory)\n    bt --completions zsh > ~/.zfunc/_bt\n\n    # Fish\n    bt --completions fish > ~/.config/fish/completions/bt.fish\n\n    # PowerShell\n    bt --completions powershell >> $PROFILE\n"
)]
pub struct Args {
    /// Output in JSON format
    #[arg(long, global = true, display_order = 100)]
    pub json: bool,

    /// Verbose output (repeat for more detail: -v, -vv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count, display_order = 101)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long, global = true, display_order = 103)]
    pub quiet: bool,

    /// Suppress all output except errors (implies --quiet)
    #[arg(long, global = true, display_order = 104)]
    pub silent: bool,

    /// Output plain text without ANSI escape codes.
    ///
    /// Forces color depth to none and overrides FORCE_COLOR / CLICOLOR_FORCE.
    /// NO_COLOR is still honored when --plain is absent.
    #[arg(long, global = true, display_order = 105)]
    pub plain: bool,

    /// Generate shell completions and exit.
    #[arg(long, value_name = "SHELL", global = true, display_order = 102)]
    pub completions: Option<ShellType>,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Content to analyze (positional; multiple values are joined with spaces)
    #[arg(value_name = "CONTENT")]
    pub content: Vec<String>,
}

/// CLI subcommands
#[derive(Subcommand, Debug)]
#[command(disable_help_subcommand = true)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    #[command(display_order = 0)]
    About(about::AboutArgs),

    #[command(display_order = 1)]
    Image(image::ImageArgs),

    #[command(display_order = 2)]
    Flowchart(flowchart::FlowchartArgs),

    #[command(display_order = 3)]
    Quadrant(quadrant::QuadrantArgs),

    #[command(name = "pie-chart", display_order = 4)]
    PieChart(pie::PieChartArgs),

    #[command(name = "git-graph", display_order = 5)]
    GitGraph(git_graph::GitGraphArgs),

    #[command(name = "bar-chart", display_order = 7)]
    BarChart(xy_chart::BarChartArgs),

    #[command(name = "line-chart", display_order = 8)]
    LineChart(xy_chart::LineChartArgs),

    #[command(name = "timeline", display_order = 9)]
    Timeline(timeline::TimelineArgs),

    #[command(name = "state-diagram", display_order = 10)]
    StateDiagram(state_diagram::StateDiagramArgs),

    #[command(display_order = 11)]
    GraphExpression(graph::GraphExpressionArgs),

    #[command(name = "erd", display_order = 12)]
    Erd(erd::ErdArgs),

    #[command(display_order = 20)]
    Prose(prose::ProseArgs),

    #[command(display_order = 12)]
    Quote(quote::QuoteArgs),

    #[command(display_order = 13)]
    List(list::ListArgs),

    #[command(name = "padleft", display_order = 14)]
    PadLeft(pad::PadLeftArgs),

    #[command(name = "padright", display_order = 14)]
    PadRight(pad::PadRightArgs),

    #[command(display_order = 15)]
    Columns(columns::ColumnsArgs),

    #[command(display_order = 15)]
    Dir(dir::DirArgs),

    #[command(display_order = 16)]
    Block(block::BlockArgs),

    #[command(display_order = 17)]
    Progress(progress::ProgressArgs),

    #[command(display_order = 18)]
    Table(table::TableArgs),

    #[command(display_order = 19)]
    Compose(compose::ComposeArgs),

    #[command(display_order = 20)]
    Section(section::SectionArgs),

    #[command(name = "status-block", display_order = 21)]
    StatusBlock(status_block::StatusBlockArgs),

    #[command(name = "text-block", display_order = 22)]
    TextBlock(text_block::TextBlockArgs),

    #[command(display_order = 23)]
    Todo(todo::TodoArgs),
}
