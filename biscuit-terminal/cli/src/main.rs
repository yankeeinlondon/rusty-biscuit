//! Terminal information utility CLI.
//!
//! Displays terminal metadata and capabilities including:
//! - Terminal application detection
//! - Color depth and mode
//! - Feature support (italics, images, underlines, OSC links)
//! - Multiplexing status
//! - OS and distribution information

use std::path::Path;

use biscuit_terminal::{
    components::{
        mermaid::{MermaidRenderer, QuadrantTheme},
        terminal_image::{parse_filepath_and_width, parse_width_spec, ImageWidth, TerminalImage},
    },
    discovery::{
        clipboard,
        detection::{multiplex_support, Connection, MultiplexSupport},
        eval, fonts, mode_2027, osc_queries,
    },
    terminal::Terminal,
    utils::escape_codes,
};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::engine::{ArgValueCompleter, PathCompleter};
use clap_complete::Shell;
use serde::Serialize;

/// Brief pause after image rendering.
///
/// This is a minimal delay to ensure the terminal has finished processing
/// image data before we print any following text.
fn settle_terminal() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    // Small delay for terminal processing
    std::thread::sleep(std::time::Duration::from_millis(10));
}

/// Prints the command used to generate an example diagram.
///
/// Uses bold text for header and dim text for command.
/// Avoids terminal color mode queries which can interfere with Kitty graphics protocol.
fn print_example_command(cmd: &str) {
    // Check NO_COLOR environment variable
    let no_color = std::env::var("NO_COLOR").is_ok();

    if no_color {
        println!();
        println!("Command:");
        println!("{}", cmd);
    } else {
        // Use bold for header - terminal's default foreground color is already appropriate
        let bold = "\x1b[1m";
        let dim = "\x1b[2m";
        let reset = "\x1b[0m";

        println!();
        println!("{}Command:{}", bold, reset);
        println!("{}{}{}", dim, cmd, reset);
    }
}

/// Terminal information utility
#[derive(Parser, Debug)]
#[command(name = "bt")]
#[command(author, version, about = "Display terminal metadata and capabilities")]
#[command(disable_help_subcommand = true)]
#[command(after_help = "\
SHELL COMPLETIONS:
  Two methods are available:

  DYNAMIC (recommended, includes image file filtering):
    # Bash
    echo 'source <(COMPLETE=bash bt)' >> ~/.bashrc

    # Zsh
    echo 'source <(COMPLETE=zsh bt)' >> ~/.zshrc

    # Fish
    echo 'COMPLETE=fish bt | source' >> ~/.config/fish/config.fish

  STATIC (generates a completion script):
    # Bash
    bt --completions bash >> ~/.bashrc

    # Zsh (ensure fpath includes the directory)
    bt --completions zsh > ~/.zfunc/_bt

    # Fish
    bt --completions fish > ~/.config/fish/completions/bt.fish

    # PowerShell
    bt --completions powershell >> $PROFILE
")]
struct Args {
    /// Output in JSON format
    #[arg(long, global = true, display_order = 100)]
    json: bool,

    /// Verbose output (show more details)
    #[arg(short, long, global = true, display_order = 101)]
    verbose: bool,

    /// Generate shell completions and exit.
    ///
    /// Outputs completion scripts for the specified shell to stdout.
    /// Redirect the output to the appropriate file for your shell.
    /// Use --completions help for setup instructions.
    #[arg(long, value_name = "SHELL", global = true, display_order = 102)]
    completions: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,

    /// Content to analyze (positional; multiple values are joined with spaces)
    #[arg(value_name = "CONTENT")]
    content: Vec<String>,
}

/// CLI subcommands
#[derive(Subcommand, Debug)]
#[command(disable_help_subcommand = true)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Display an image in the terminal
    #[command(display_order = 1)]
    ///
    /// Supports width specification: "file.jpg|50%" or "file.jpg|80".
    /// Supports PNG, JPG, JPEG, and GIF formats.
    Image {
        /// Image file path with optional width spec (e.g., "photo.jpg|75%")
        #[arg(value_name = "FILEPATH", add = ArgValueCompleter::new(image_completer()))]
        filepath: String,

        /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
        ///
        /// Overrides inline width spec (e.g., "file.jpg|50%"). Aspect ratio is always preserved.
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Output rendering metadata to stderr (filename, file size, render time)
        #[arg(long)]
        meta: bool,
    },

    /// Render a flowchart from node definitions
    ///
    /// Creates a Mermaid flowchart and renders it to the terminal.
    /// Default direction is left-to-right (LR).
    #[command(after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example flowchart:
    bt flowchart --example

  Basic flowchart (left-to-right):
    bt flowchart \"A --> B --> C\"

  Top-down direction:
    bt flowchart --vertical \"Start --> Middle --> End\"

  With node labels and shapes:
    bt flowchart \"A[Input] --> B{Decision}\" \"B -->|Yes| C[Output]\" \"B -->|No| D[Retry]\"

  With title:
    bt flowchart --title \"My Process\" \"A --> B --> C\"

  Inverted colors (solid background):
    bt flowchart --inverse \"A --> B --> C\"

  Custom width:
    bt flowchart --width 30% \"A --> B\"    # 30% of terminal width
    bt flowchart --width 80ch \"A --> B\"   # 80 characters wide
    bt flowchart --width fill \"A --> B\"   # Full terminal width

  JSON output (for scripting):
    bt flowchart --json \"A --> B\"
")]
    #[command(display_order = 2)]
    Flowchart {
        /// Render top-down instead of left-right
        #[arg(long)]
        vertical: bool,

        /// Use inverted colors with solid background
        ///
        /// Instead of transparent background matching the terminal, renders with
        /// a solid background (white in dark mode, black in light mode) and
        /// contrasting shapes.
        #[arg(long)]
        inverse: bool,

        /// Add a title above the diagram
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
        ///
        /// Default is 50% of terminal width. Aspect ratio is always preserved.
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Render an example diagram and show the command used
        #[arg(long, short = 'e')]
        example: bool,

        /// Output rendering metadata to stderr (filename, cache hit, file size, render time)
        #[arg(long)]
        meta: bool,

        /// Flowchart node and edge definitions (e.g., "A --> B --> C")
        #[arg(value_name = "CONTENT", required_unless_present = "example")]
        content: Vec<String>,
    },

    /// Render a quadrant chart from data points
    ///
    /// Creates a Mermaid quadrantChart and renders it to the terminal.
    /// Data points are specified as "Label: [x, y]" where x and y are 0.0-1.0.
    #[command(display_order = 3, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example quadrant chart:
    bt quadrant --example

  Basic quadrant chart:
    bt quadrant \"Item A: [0.3, 0.6]\" \"Item B: [0.7, 0.4]\"

  With axis labels:
    bt quadrant --x-axis \"Low --> High\" --y-axis \"Small --> Large\" \\
                \"Item: [0.5, 0.5]\"

  With quadrant descriptions:
    bt quadrant --top-left \"Expand\" --top-right \"Promote\" \\
                --bottom-left \"Review\" --bottom-right \"Improve\" \\
                \"A: [0.3, 0.7]\" \"B: [0.8, 0.2]\"

  With title:
    bt quadrant --title \"Priority Matrix\" \"Task A: [0.2, 0.8]\"

  Full example:
    bt quadrant --title \"Campaign Analysis\" \\
                --x-axis \"Low Reach --> High Reach\" \\
                --y-axis \"Low Engagement --> High Engagement\" \\
                --top-left \"Expand\" --top-right \"Promote\" \\
                --bottom-left \"Re-evaluate\" --bottom-right \"Improve\" \\
                \"Campaign A: [0.3, 0.6]\" \"Campaign B: [0.7, 0.4]\"

  Inverted colors (solid background):
    bt quadrant --inverse \"Item: [0.5, 0.5]\"

  Custom width:
    bt quadrant --width 60% \"Item: [0.5, 0.5]\"

  Custom point styling:
    bt quadrant --point-radius 12 --label-size 16 \"Item: [0.5, 0.5]\"

  Magic Quadrangle theme (subtle green top-right, red bottom-left):
    bt quadrant --theme magic-quadrangle \"Leaders: [0.8, 0.8]\" \"Niche: [0.2, 0.2]\"

  Custom quadrant colors:
    bt quadrant --q1-fill \"#e8f5e9\" --q3-fill \"#ffebee\" \\
                \"Item A: [0.7, 0.8]\" \"Item B: [0.3, 0.2]\"

  Per-point inline styling (color, radius):
    bt quadrant \"Item A: [0.3, 0.6] color: #ff3300, radius: 10\" \\
                \"Item B: [0.7, 0.4] color: #00ff00\"

  JSON output (for scripting):
    bt quadrant --json \"Item: [0.5, 0.5]\"
")]
    Quadrant {
        /// X-axis label (e.g., \"Low --> High\")
        #[arg(long = "x-axis", short = 'x', allow_hyphen_values = true)]
        x_axis: Option<String>,

        /// Y-axis label (e.g., \"Small --> Large\")
        #[arg(long = "y-axis", short = 'y', allow_hyphen_values = true)]
        y_axis: Option<String>,

        /// Chart title (appears at top of diagram)
        #[arg(long, short = 't', allow_hyphen_values = true)]
        title: Option<String>,

        /// Top-left quadrant label (quadrant-1)
        #[arg(long = "top-left", short = 'l', visible_alias = "tl", allow_hyphen_values = true)]
        top_left: Option<String>,

        /// Top-right quadrant label (quadrant-2)
        #[arg(long = "top-right", short = 'r', visible_alias = "tr", allow_hyphen_values = true)]
        top_right: Option<String>,

        /// Bottom-left quadrant label (quadrant-3)
        #[arg(long = "bottom-left", visible_alias = "bl", allow_hyphen_values = true)]
        bottom_left: Option<String>,

        /// Bottom-right quadrant label (quadrant-4)
        #[arg(long = "bottom-right", visible_alias = "br", allow_hyphen_values = true)]
        bottom_right: Option<String>,

        /// Use inverted colors with solid background
        #[arg(long)]
        inverse: bool,

        /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
        ///
        /// Default is 50% of terminal width. Aspect ratio is always preserved.
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Default point radius (default: 5)
        ///
        /// Sets the size of all data points. Individual points can override
        /// this using inline syntax: "Item: [0.5, 0.5] radius: 10"
        #[arg(long)]
        point_radius: Option<u32>,

        /// Point label font size (default: 18 for ≤6 points, 15 for >6)
        ///
        /// Sets the font size for data point labels. The default adjusts
        /// based on point count for better readability.
        #[arg(long)]
        label_size: Option<u32>,

        /// Quadrant color theme preset
        #[arg(long, value_enum, default_value_t = QuadrantTheme::Default)]
        theme: QuadrantTheme,

        /// Top-right quadrant (q1) fill color (hex, e.g., "#e8f5e9")
        #[arg(long = "q1-fill")]
        q1_fill: Option<String>,

        /// Top-left quadrant (q2) fill color (hex, e.g., "#ffffff")
        #[arg(long = "q2-fill")]
        q2_fill: Option<String>,

        /// Bottom-left quadrant (q3) fill color (hex, e.g., "#ffebee")
        #[arg(long = "q3-fill")]
        q3_fill: Option<String>,

        /// Bottom-right quadrant (q4) fill color (hex, e.g., "#ffffff")
        #[arg(long = "q4-fill")]
        q4_fill: Option<String>,

        /// Render an example diagram and show the command used
        #[arg(long, short = 'e')]
        example: bool,

        /// Output rendering metadata to stderr (filename, cache hit, file size, render time)
        #[arg(long)]
        meta: bool,

        /// Data points as "Label: [x, y]" where x and y are 0.0-1.0
        #[arg(value_name = "POINTS", required_unless_present = "example")]
        points: Vec<String>,
    },

    /// Render a pie chart from data values
    ///
    /// Creates a Mermaid pie chart and renders it to the terminal.
    /// Data points are specified as "Label: value" pairs.
    #[command(name = "pie-chart", display_order = 4, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example pie chart:
    bt pie-chart --example

  Basic pie chart (separate arguments):
    bt pie-chart \"Dogs: 386\" \"Cats: 85\" \"Birds: 15\"

  With semicolon-delimited string:
    bt pie-chart \"Dogs: 386; Cats: 85; Birds: 15\"

  Official Mermaid syntax (quotes around labels):
    bt pie-chart '\"Dogs\" : 386' '\"Cats\" : 85'

  With title:
    bt pie-chart --title \"Pet Distribution\" \"Dogs: 386\" \"Cats: 85\"

  Inverted colors (solid background):
    bt pie-chart --inverse \"Dogs: 386\" \"Cats: 85\"

  Custom width:
    bt pie-chart --width 40% \"Dogs: 386\" \"Cats: 85\"

  Show percentages on slices:
    bt pie-chart --show-data \"Dogs: 386\" \"Cats: 85\"

  Custom slice colors (brand colors):
    bt pie-chart \"TypeScript: 45 #3178c6\" \"Rust: 35 #dea584\" \"Python: 20\"

  Custom colors with 'color:' prefix:
    bt pie-chart \"TypeScript: 45 color: #3178c6\" \"Rust: 35 color: #dea584\"

  JSON output (for scripting):
    bt pie-chart --json \"Dogs: 386\" \"Cats: 85\"

\x1b[1m\x1b[4mInput Formats:\x1b[0m
  Simplified:    \"Label: value\" (quotes around label optional)
  Semicolon:     \"Label1: 10; Label2: 20; Label3: 30\"
  Official:      '\"Label\" : value' (Mermaid's native syntax)

\x1b[1m\x1b[4mCustom Colors:\x1b[0m
  Add a hex color at the end of any data point:
    \"Label: value #rrggbb\"       (shorthand)
    \"Label: value color: #rrggbb\" (explicit)

  Slices without colors use Mermaid's default palette.
")]
    PieChart {
        /// Use inverted colors with solid background
        ///
        /// Instead of transparent background matching the terminal, renders with
        /// a solid background (white in dark mode, black in light mode) and
        /// contrasting shapes.
        #[arg(long)]
        inverse: bool,

        /// Add a title above the chart
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
        ///
        /// Default is 50% of terminal width. Aspect ratio is always preserved.
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Show data values on the pie slices
        #[arg(long)]
        show_data: bool,

        /// Render an example diagram and show the command used
        #[arg(long, short = 'e')]
        example: bool,

        /// Output rendering metadata to stderr (filename, cache hit, file size, render time)
        #[arg(long)]
        meta: bool,

        /// Data points as "Label: value" pairs (e.g., "Dogs: 386" "Cats: 85")
        #[arg(value_name = "DATA", required_unless_present = "example")]
        data: Vec<String>,
    },

    /// Render a git graph from git commands
    ///
    /// Creates a Mermaid gitGraph and renders it to the terminal.
    /// Git commands include: commit, branch, checkout, merge, cherry-pick.
    #[command(name = "git-graph", display_order = 5, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example git graph:
    bt git-graph --example

  Simple commit history:
    bt git-graph \"commit\" \"commit\" \"commit\"

  Feature branch workflow:
    bt git-graph \"commit\" \"branch feature\" \"checkout feature\" \"commit\" \"commit\" \\
                 \"checkout main\" \"merge feature\"

  With commit IDs and tags:
    bt git-graph \"commit id: \\\"abc123\\\"\" \"commit tag: \\\"v1.0\\\"\"

  With title:
    bt git-graph --title \"Release Flow\" \"commit\" \"branch release\" \"commit\"

  Inverted colors (solid background):
    bt git-graph --inverse \"commit\" \"commit\"

  Custom width:
    bt git-graph --width 30% \"commit\" \"commit\"   # 30% of terminal width
    bt git-graph --width 80ch \"commit\"            # 80 characters wide
    bt git-graph --width fill \"commit\"            # Full terminal width

  JSON output (for scripting):
    bt git-graph --json \"commit\" \"branch dev\"

\x1b[1m\x1b[4mGit commands:\x1b[0m
  commit                    Add a commit to the current branch
  commit id: \"abc\"          Commit with custom ID
  commit tag: \"v1.0\"        Commit with a tag
  branch <name>             Create a new branch
  checkout <name>           Switch to a branch
  merge <name>              Merge a branch into current
  cherry-pick id: \"abc\"     Cherry-pick a commit
")]
    GitGraph {
        /// Use inverted colors with solid background
        ///
        /// Instead of transparent background matching the terminal, renders with
        /// a solid background (white in dark mode, black in light mode) and
        /// contrasting shapes.
        #[arg(long)]
        inverse: bool,

        /// Add a title above the diagram
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
        ///
        /// Default is 50% of terminal width. Aspect ratio is always preserved.
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Render an example diagram and show the command used
        #[arg(long, short = 'e')]
        example: bool,

        /// Output rendering metadata to stderr (filename, cache hit, file size, render time)
        #[arg(long)]
        meta: bool,

        /// Git graph commands (commit, branch <name>, checkout <name>, merge <name>)
        #[arg(value_name = "COMMANDS", required_unless_present = "example")]
        commands: Vec<String>,
    },

    /// Render a bar chart from data values
    ///
    /// Creates a Mermaid XY chart with bar series and renders it to the terminal.
    /// Data can be provided as JSON array, comma-separated, or space-separated values.
    #[command(name = "bar-chart", display_order = 7, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example bar chart:
    bt bar-chart --example

  Basic bar chart (space-separated):
    bt bar-chart 1 8 7 5

  Comma-separated values:
    bt bar-chart \"1,8,7,5\"

  JSON array format:
    bt bar-chart \"[1,8,7,5]\"

  With axis labels:
    bt bar-chart --x-axis \"Q1,Q2,Q3,Q4\" --y-axis Sales 10 20 15 25

  With title:
    bt bar-chart --title \"Quarterly Sales\" 10 20 15 25

  Horizontal bars:
    bt bar-chart --horizontal 1 8 7 5

  Show data labels on bars:
    bt bar-chart --show-data-label 1 8 7 5

  Add a line series:
    bt bar-chart --line 1 8 7 5

  Custom width and aspect ratio:
    bt bar-chart --width 60% --aspect-ratio 2.0 1 8 7 5

  Inverted colors (solid background):
    bt bar-chart --inverse 1 8 7 5

\x1b[1m\x1b[4mInput Formats:\x1b[0m
  JSON:          \"[1, 8, 7, 5]\"
  Comma-sep:     \"1,8,7,5\"  or  \"1, 8, 7, 5\"
  Space-sep:     1 8 7 5
")]
    BarChart {
        /// Chart title
        #[arg(long, short = 't')]
        title: Option<String>,

        /// X-axis label or comma-separated category labels
        #[arg(long = "x-axis", short = 'x')]
        x_axis: Option<String>,

        /// Y-axis label
        #[arg(long = "y-axis", short = 'y')]
        y_axis: Option<String>,

        /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Render bars horizontally instead of vertically
        #[arg(long)]
        horizontal: bool,

        /// Show data labels on bars
        #[arg(long)]
        show_data_label: bool,

        /// Aspect ratio (width/height). Default: 1.5
        #[arg(long)]
        aspect_ratio: Option<f32>,

        /// Also render data as a line
        #[arg(long)]
        line: bool,

        /// Use inverted colors with solid background
        #[arg(long)]
        inverse: bool,

        /// Render an example chart and show the command used
        #[arg(long, short = 'e')]
        example: bool,

        /// Output rendering metadata to stderr (filename, cache hit, file size, render time)
        #[arg(long)]
        meta: bool,

        /// Data values (JSON array, comma-separated, or space-separated)
        #[arg(value_name = "DATA", required_unless_present = "example")]
        data: Vec<String>,
    },

    /// Render a line chart from data values
    ///
    /// Creates a Mermaid XY chart with line series and renders it to the terminal.
    /// Same input formats as bar-chart.
    #[command(name = "line-chart", display_order = 8, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example line chart:
    bt line-chart --example

  Basic line chart:
    bt line-chart 1 8 7 5 9 3

  With axis labels:
    bt line-chart --x-axis \"Mon,Tue,Wed,Thu,Fri\" --y-axis Temperature 20 22 19 21 23

  With title:
    bt line-chart --title \"Weekly Temps\" 20 22 19 21 23

  Show data points:
    bt line-chart --show-data-label 1 8 7 5

  Also add bars:
    bt line-chart --bar 1 8 7 5

  Custom width:
    bt line-chart --width 60% 1 8 7 5

\x1b[1m\x1b[4mInput Formats:\x1b[0m
  JSON:          \"[1, 8, 7, 5]\"
  Comma-sep:     \"1,8,7,5\"  or  \"1, 8, 7, 5\"
  Space-sep:     1 8 7 5
")]
    LineChart {
        /// Chart title
        #[arg(long, short = 't')]
        title: Option<String>,

        /// X-axis label or comma-separated category labels
        #[arg(long = "x-axis", short = 'x')]
        x_axis: Option<String>,

        /// Y-axis label
        #[arg(long = "y-axis", short = 'y')]
        y_axis: Option<String>,

        /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Render horizontally instead of vertically
        #[arg(long)]
        horizontal: bool,

        /// Show data labels on points
        #[arg(long)]
        show_data_label: bool,

        /// Aspect ratio (width/height). Default: 1.5
        #[arg(long)]
        aspect_ratio: Option<f32>,

        /// Also render data as bars
        #[arg(long)]
        bar: bool,

        /// Use inverted colors with solid background
        #[arg(long)]
        inverse: bool,

        /// Render an example chart and show the command used
        #[arg(long, short = 'e')]
        example: bool,

        /// Output rendering metadata to stderr (filename, cache hit, file size, render time)
        #[arg(long)]
        meta: bool,

        /// Data values (JSON array, comma-separated, or space-separated)
        #[arg(value_name = "DATA", required_unless_present = "example")]
        data: Vec<String>,
    },

    /// Render a timeline diagram
    ///
    /// Creates a Mermaid timeline showing events over time periods.
    /// Events are specified as "YYYY: Event description" format.
    #[command(name = "timeline", display_order = 9, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example timeline:
    bt timeline --example

  Basic timeline:
    bt timeline \"2020: Project started\" \"2021: First release\" \"2022: Major update\"

  With title:
    bt timeline --title \"Company History\" \"2020: Founded\" \"2022: IPO\"

  With sections (grouped time periods):
    bt timeline --section \"Early Years\" \"2020: Founded\" \"2021: Seed funding\" \\
                --section \"Growth\" \"2022: Series A\" \"2023: Expansion\"

  Custom width:
    bt timeline --width 60% \"2020: Event A\" \"2021: Event B\"

  Inverted colors:
    bt timeline --inverse \"2020: Event\" \"2021: Event\"

\x1b[1m\x1b[4mInput Format:\x1b[0m
  Each event: \"YYYY: Description\" where YYYY is a year or time period
  Sections group related events with --section \"Section Name\"
")]
    Timeline {
        /// Timeline title
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., \"50%\"), characters (e.g., \"80ch\"), or \"fill\"
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Section name (can be used multiple times, applies to following events)
        #[arg(long, short = 's', action = clap::ArgAction::Append)]
        section: Vec<String>,

        /// Use inverted colors with solid background
        #[arg(long)]
        inverse: bool,

        /// Render an example timeline and show the command used
        #[arg(long, short = 'e')]
        example: bool,

        /// Output rendering metadata to stderr (filename, cache hit, file size, render time)
        #[arg(long)]
        meta: bool,

        /// Timeline events as \"YYYY: Description\"
        #[arg(value_name = "EVENTS", required_unless_present = "example")]
        events: Vec<String>,
    },

    /// Render a state diagram
    ///
    /// Creates a Mermaid state diagram showing states and transitions.
    /// Uses the same syntax as flowchart for defining states and transitions.
    #[command(name = "state-diagram", display_order = 10, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example state diagram:
    bt state-diagram --example

  Basic state diagram:
    bt state-diagram \"[*] --> Idle\" \"Idle --> Running\" \"Running --> [*]\"

  With state descriptions:
    bt state-diagram \"[*] --> Idle\" \"Idle --> Running: start\" \"Running --> Stopped: stop\"

  With title:
    bt state-diagram --title \"Process States\" \"[*] --> Ready\" \"Ready --> Running\"

  Custom width:
    bt state-diagram --width 60% \"[*] --> A\" \"A --> B\"

  Inverted colors:
    bt state-diagram --inverse \"[*] --> A\" \"A --> [*]\"

\x1b[1m\x1b[4mSyntax:\x1b[0m
  [*]           Start/end state
  State1 --> State2          Transition
  State1 --> State2: label   Labeled transition
")]
    StateDiagram {
        /// Diagram title
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., \"50%\"), characters (e.g., \"80ch\"), or \"fill\"
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Use inverted colors with solid background
        #[arg(long)]
        inverse: bool,

        /// Render an example state diagram and show the command used
        #[arg(long, short = 'e')]
        example: bool,

        /// Output rendering metadata to stderr (filename, cache hit, file size, render time)
        #[arg(long)]
        meta: bool,

        /// State transitions (e.g., \"[*] --> Idle\", \"Idle --> Running\")
        #[arg(value_name = "TRANSITIONS", required_unless_present = "example")]
        transitions: Vec<String>,
    },

    /// Render an entity relationship diagram (ERD)
    ///
    /// Creates a Mermaid ERD showing entities and their relationships.
    #[command(name = "erd", display_order = 11, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example ERD:
    bt erd --example

  Basic ERD with relationships:
    bt erd \"Customer ||--o{ Order : places\" \"Order ||--|{ LineItem : contains\"

  Entity with attributes:
    bt erd --entity \"Customer { id int PK, name string, email string }\" \\
           --entity \"Order { id int PK, date date, customer_id int FK }\" \\
           \"Customer ||--o{ Order : places\"

  With title:
    bt erd --title \"E-Commerce Schema\" \"Customer ||--o{ Order : places\"

  Custom width:
    bt erd --width 60% \"A ||--o{ B : has\"

\x1b[1m\x1b[4mRelationship Syntax:\x1b[0m
  ||--||   One to one
  ||--o{   One to many
  }o--o{   Many to many
  ||--o|   One to zero or one

  Entity1 <rel> Entity2 : label
")]
    Erd {
        /// Diagram title
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., \"50%\"), characters (e.g., \"80ch\"), or \"fill\"
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Entity definition (can be used multiple times)
        #[arg(long, short = 'E', action = clap::ArgAction::Append)]
        entity: Vec<String>,

        /// Use inverted colors with solid background
        #[arg(long)]
        inverse: bool,

        /// Render an example ERD and show the command used
        #[arg(long, short = 'e')]
        example: bool,

        /// Output rendering metadata to stderr (filename, cache hit, file size, render time)
        #[arg(long)]
        meta: bool,

        /// Relationships (e.g., \"Customer ||--o{ Order : places\")
        #[arg(value_name = "RELATIONSHIPS", required_unless_present = "example")]
        relationships: Vec<String>,
    },

    /// Render prose text with inline styling tokens
    ///
    /// Renders text with inline styling using atomic tokens ({{bold}}, {{red}})
    /// and block tags (<b>, <i>, <a href="...">).
    #[command(display_order = 20, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Atomic tokens:
    bt prose \"Hello {{bold}}world{{reset}}!\"
    bt prose \"{{red}}Error:{{reset}} Something went wrong\"
    bt prose \"{{bg-yellow}}Warning{{reset}}: Check this\"

  Block tags:
    bt prose \"<b>Bold</b> and <i>italic</i> text\"
    bt prose \"<u>Underlined</u> and <~>strikethrough</~>\"
    bt prose \"Visit <a href='https://example.com'>our site</a>\"
    bt prose \"<red>Error message</red>\"

  With margins:
    bt prose --margin-left 4 \"Indented text\"
    bt prose -l 4 -r 4 \"Indented with margins\"

  With alignment:
    bt prose --alignment center \"Centered text\"
    bt prose -a right \"Right-aligned text\"

  Disable word wrapping:
    bt prose --no-wrap \"Long line that should not wrap\"

\x1b[1m\x1b[4mAtomic Tokens:\x1b[0m
  Styles:     {{bold}}, {{dim}}, {{italic}}, {{underline}}, {{strikethrough}}
  Underline:  {{double-underline}}, {{curly-underline}}, {{dotted-underline}}, {{dashed-underline}}
  More:       {{blink}}, {{inverse}}, {{hidden}}
  Colors:     {{red}}, {{blue}}, {{green}}, {{yellow}}, {{cyan}}, {{magenta}}
  Bright:     {{bright-red}}, {{bright-blue}}, etc.
  Background: {{bg-red}}, {{bg-blue}}, etc.
  Reset:      {{reset}}, {{reset-fg}}, {{reset-bg}} (atomic only)
  Undo style: {{normal-font-weight}}, {{not-italic}}, {{not-underline}}, {{not-strikethrough}} (atomic only)

\x1b[1m\x1b[4mBlock Tags:\x1b[0m

  \x1b[1mColor Blocks:\x1b[0m
    Use a named color (basic colors, web colors, tailwind):
      \x1b[1m<red>\x1b[2ma basic color\x1b[22m</red>\x1b[0m
      \x1b[1m<bright-red>\x1b[2ma bright variant of a basic color\x1b[22m</bright-red>\x1b[0m
      \x1b[1m<alice-blue>\x1b[2ma web/CSS color variant\x1b[22m</alice-blue>\x1b[0m
      \x1b[1m<purple-500>\x1b[2ma tailwind class variant\x1b[22m</purple-500>\x1b[0m
    Use a bespoke RGB color value:
      \x1b[1m<rgb 255,0,0>\x1b[2musing an RGB color\x1b[22m</rgb>\x1b[0m

  \x1b[1mHyperlinks:\x1b[0m
    You can create OSC8 hyperlinks (with a fallback for terminals that don't support it):
      \x1b[1m<a href=https://google.com>\x1b[2mGoogle Search\x1b[22m</a>\x1b[0m
      \x1b[1m<a href=/fully/qualified/path/filename.ext>\x1b[2mSome File\x1b[22m</a>\x1b[0m
      \x1b[1m<a href=./relative/filename.ext>\x1b[2mSome File\x1b[22m</a>\x1b[0m a relative file reference:
        - relative from CWD
      \x1b[1m<a href=relative/filename.ext>\x1b[2mSome File\x1b[22m</a>\x1b[0m a relative file reference, where:
        - if in git repo then relative from either:
          - if monorepo, root of the package the CWD is in
          - repo's root
        - will check in the order specified and resolve

  \x1b[1mOther Styling:\x1b[0m
    All atomic tokens -- \x1b[3mother than negations like \x1b[1mnot-italic\x1b[22m, \x1b[1mreset\x1b[22m, etc.\x1b[23m -- are
    available as block tokens as well.
    In addition we have some shortcut aliases for convenience:
      \x1b[1m<u>\x1b[2m...\x1b[22m</u>\x1b[0m \x1b[3mprovides underlined text\x1b[23m
      \x1b[1m<uu>\x1b[2m...\x1b[22m</uu>\x1b[0m \x1b[3mprovides double-underlined text\x1b[23m
      \x1b[1m<~>\x1b[2m...\x1b[22m</~>\x1b[0m \x1b[3mprovides strikethrough text\x1b[23m
      \x1b[1m<i>\x1b[2m...\x1b[22m</i>\x1b[0m \x1b[3mprovides italics text\x1b[23m
      \x1b[1m<b>\x1b[2m...\x1b[22m</b>\x1b[0m \x1b[3mprovides bold text\x1b[23m
")]
    Prose {
        /// Content with {{tokens}} and <block>tags</block>
        #[arg(value_name = "CONTENT")]
        content: Vec<String>,

        /// Disable word wrapping
        #[arg(long)]
        no_wrap: bool,

        /// Left margin in characters
        #[arg(long, short = 'l')]
        margin_left: Option<u32>,

        /// Right margin in characters
        #[arg(long, short = 'r')]
        margin_right: Option<u32>,

        /// Text alignment
        #[arg(long, short = 'a', value_enum)]
        alignment: Option<biscuit_terminal::utils::layout::Alignment>,
    },

    /// Render styled text in a block quote
    ///
    /// Wraps prose content in a block quote with a left border.
    /// Supports the same {{tokens}} and <block>tags</block> as the prose command.
    #[command(display_order = 12, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Simple quote:
    bt quote \"To be or not to be\"

  With attribution:
    bt quote --attribution \"Shakespeare\" \"To be or not to be\"

  With styling:
    bt quote \"<bold>Important:</bold> This is <red>critical</red> information\"

  Multiline (use \\n for newlines):
    bt quote \"First line\\nSecond line\\nThird line\"

  With attribution and styling:
    bt quote -a \"Albert Einstein\" \"<i>Imagination is more important than knowledge.</i>\"
")]
    Quote {
        /// Content with {{tokens}} and <block>tags</block>
        #[arg(value_name = "CONTENT")]
        content: Vec<String>,

        /// Attribution (author/source) displayed below the quote
        #[arg(long)]
        attribution: Option<String>,

        /// Left margin in characters
        #[arg(long, short = 'l')]
        margin_left: Option<u32>,

        /// Right margin in characters
        #[arg(long, short = 'r')]
        margin_right: Option<u32>,

        /// Text alignment
        #[arg(long, short = 'a', value_enum)]
        alignment: Option<biscuit_terminal::utils::layout::Alignment>,
    },

    /// Render a bulleted list with hanging indents
    ///
    /// Each argument becomes a list item. Supports the same {{tokens}} and
    /// <block>tags</block> as the prose command for styling individual items.
    #[command(display_order = 13, after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Simple list:
    bt list \"First item\" \"Second item\" \"Third item\"

  With styled content:
    bt list \"<bold>Important:</bold> First point\" \"<dim>Note:</dim> Second point\"

  Custom bullet character:
    bt list --bullet \"- \" \"Item one\" \"Item two\"

  Arrow bullets:
    bt list --bullet \"→ \" \"Step one\" \"Step two\" \"Step three\"

  Checkbox style:
    bt list --bullet \"☐ \" \"Task A\" \"Task B\" \"Task C\"

  Long items wrap with hanging indent:
    bt list \"This is a very long list item that will wrap to multiple lines while maintaining proper indentation\" \"Short item\"

  No hanging indent:
    bt list --no-hanging-indent \"Item without hanging indent on wrap\"
")]
    List {
        /// List items with {{tokens}} and <block>tags</block>
        #[arg(value_name = "ITEMS", required = true)]
        items: Vec<String>,

        /// Custom bullet string (default: \"• \")
        #[arg(long, short = 'b', default_value = "• ")]
        bullet: String,

        /// Disable hanging indent on wrapped lines
        #[arg(long)]
        no_hanging_indent: bool,

        /// Left margin in characters
        #[arg(long, short = 'l')]
        margin_left: Option<u32>,

        /// Right margin in characters
        #[arg(long, short = 'r')]
        margin_right: Option<u32>,

        /// Text alignment
        #[arg(long, short = 'a', value_enum)]
        alignment: Option<biscuit_terminal::utils::layout::Alignment>,
    },
}

#[derive(Debug, Serialize)]
struct TerminalMetadata {
    /// Terminal application name
    app: String,
    /// Operating system type
    os: String,
    /// Linux distribution info (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    distro: Option<DistroInfo>,

    /// Terminal width in columns
    width: u32,
    /// Terminal height in rows
    height: u32,

    /// Whether stdout is connected to a TTY
    is_tty: bool,
    /// Whether running in a CI environment
    is_ci: bool,

    /// Font name (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    font: Option<String>,
    /// Font size in pixels (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    font_size: Option<u32>,
    /// Whether using a Nerd Font (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    is_nerd_font: Option<bool>,
    /// Font ligatures (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    font_ligatures: Option<Vec<String>>,
    /// Whether the terminal likely supports font ligatures (heuristic)
    ligatures_likely: bool,

    /// Supported color depth
    color_depth: String,
    /// Light/dark mode
    color_mode: String,
    /// Background color (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    bg_color: Option<ColorInfo>,
    /// Text/foreground color (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    text_color: Option<ColorInfo>,
    /// Cursor color (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor_color: Option<ColorInfo>,

    /// Whether italics are supported
    supports_italic: bool,
    /// Image rendering support
    image_support: String,
    /// Underline style support
    underline_support: UnderlineInfo,
    /// OSC8 hyperlink support
    osc_link_support: bool,
    /// OSC10 foreground color query support
    osc10_fg_color: bool,
    /// OSC11 background color query support
    osc11_bg_color: bool,
    /// OSC12 cursor color query support
    osc12_cursor_color: bool,
    /// OSC52 clipboard support
    osc52_clipboard: bool,
    /// Mode 2027 grapheme cluster width support
    mode_2027_graphemes: bool,

    /// Multiplexer type
    multiplex: String,

    /// Connection type (Local, SSH, Mosh)
    connection: ConnectionInfo,
    /// Raw locale string from environment (e.g., "en_US.UTF-8", "C")
    #[serde(skip_serializing_if = "Option::is_none")]
    locale_raw: Option<String>,
    /// Normalized locale tag (BCP47 format, e.g., "en-US", "und" for C/POSIX)
    #[serde(skip_serializing_if = "Option::is_none")]
    locale_tag: Option<String>,
    /// Character encoding
    char_encoding: String,

    /// Path to terminal config file
    #[serde(skip_serializing_if = "Option::is_none")]
    config_file: Option<String>,
}

#[derive(Debug, Serialize)]
struct ContentAnalysis {
    /// Number of lines in the content
    line_count: u32,
    /// Length of each line in characters (escape codes stripped)
    line_lengths: Vec<u32>,
    /// Whether the content contains SGR color escape codes
    contains_color_escape_codes: bool,
    /// Whether the content contains OSC8 links
    contains_osc8_links: bool,
    /// Total character length (escape codes stripped)
    total_length: u32,
}

/// Metadata about a rendered image or diagram.
///
/// Output to stderr as JSON when --meta flag is used.
#[derive(Debug, Serialize)]
struct RenderMeta {
    /// Absolute path to the rendered/loaded image file
    filename: String,
    /// Whether this was a cache hit (true) or generated fresh (false)
    cache_hit: bool,
    /// File size in bytes
    file_size_bytes: u64,
    /// Time to render/load in milliseconds
    render_time_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ConnectionInfo {
    Local,
    #[serde(rename = "SSH")]
    Ssh {
        host: String,
        source_port: u32,
        server_port: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        tty_path: Option<String>,
    },
    Mosh {
        connection: String,
    },
}

#[derive(Debug, Serialize)]
struct DistroInfo {
    /// Distribution ID (e.g., "ubuntu", "fedora")
    id: String,
    /// Pretty name
    name: String,
    /// Version number
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    /// Version codename
    #[serde(skip_serializing_if = "Option::is_none")]
    codename: Option<String>,
    /// Distribution family
    family: String,
}

#[derive(Debug, Serialize)]
struct ColorInfo {
    /// Red component (0-255)
    r: u8,
    /// Green component (0-255)
    g: u8,
    /// Blue component (0-255)
    b: u8,
    /// Hex color code
    #[serde(skip_serializing_if = "Option::is_none")]
    hex: Option<String>,
}

#[derive(Debug, Serialize)]
struct UnderlineInfo {
    /// Straight/single underline
    straight: bool,
    /// Double underline
    double: bool,
    /// Curly/squiggly underline
    curly: bool,
    /// Dotted underline
    dotted: bool,
    /// Dashed underline
    dashed: bool,
    /// Colored underlines
    colored: bool,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Handle dynamic completions (COMPLETE env var)
    // This must run before any other initialization
    clap_complete::CompleteEnv::with_factory(Args::command).complete();

    // Setup logging if RUST_LOG is set
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    let args = Args::parse();

    // Handle --completions flag (generates static completion scripts)
    if let Some(ref shell_arg) = args.completions {
        return handle_completions(shell_arg);
    }

    // Handle subcommands
    match args.command {
        Some(Command::Image { ref filepath, ref width, meta }) => {
            return render_image(filepath, width.as_deref(), meta);
        }
        Some(Command::Flowchart {
            vertical,
            inverse,
            ref title,
            ref width,
            example,
            meta,
            ref content,
        }) => {
            return render_flowchart(
                vertical,
                inverse,
                title.as_deref(),
                width.as_deref(),
                example,
                meta,
                content,
                args.json,
            );
        }
        Some(Command::Quadrant {
            ref x_axis,
            ref y_axis,
            ref title,
            ref top_left,
            ref top_right,
            ref bottom_left,
            ref bottom_right,
            inverse,
            ref width,
            point_radius,
            label_size,
            ref theme,
            ref q1_fill,
            ref q2_fill,
            ref q3_fill,
            ref q4_fill,
            example,
            meta,
            ref points,
        }) => {
            return render_quadrant(
                x_axis.as_deref(),
                y_axis.as_deref(),
                title.as_deref(),
                top_left.as_deref(),
                top_right.as_deref(),
                bottom_left.as_deref(),
                bottom_right.as_deref(),
                inverse,
                width.as_deref(),
                point_radius,
                label_size,
                *theme,
                q1_fill.as_deref(),
                q2_fill.as_deref(),
                q3_fill.as_deref(),
                q4_fill.as_deref(),
                example,
                meta,
                points,
                args.json,
            );
        }
        Some(Command::PieChart {
            inverse,
            ref title,
            ref width,
            show_data,
            example,
            meta,
            ref data,
        }) => {
            return render_pie_chart(
                inverse,
                title.as_deref(),
                width.as_deref(),
                show_data,
                example,
                meta,
                data,
                args.json,
            );
        }
        Some(Command::GitGraph {
            inverse,
            ref title,
            ref width,
            example,
            meta,
            ref commands,
        }) => {
            return render_git_graph(
                inverse,
                title.as_deref(),
                width.as_deref(),
                example,
                meta,
                commands,
                args.json,
            );
        }
        Some(Command::BarChart {
            ref title,
            ref x_axis,
            ref y_axis,
            ref width,
            horizontal,
            show_data_label,
            aspect_ratio,
            line,
            inverse,
            example,
            meta,
            ref data,
        }) => {
            return render_xy_chart(
                XyChartType::Bar,
                title.as_deref(),
                x_axis.as_deref(),
                y_axis.as_deref(),
                width.as_deref(),
                horizontal,
                show_data_label,
                aspect_ratio,
                line,       // add_line for bar chart
                false,      // add_bar is false since we're a bar chart
                inverse,
                example,
                meta,
                data,
                args.json,
            );
        }
        Some(Command::LineChart {
            ref title,
            ref x_axis,
            ref y_axis,
            ref width,
            horizontal,
            show_data_label,
            aspect_ratio,
            bar,
            inverse,
            example,
            meta,
            ref data,
        }) => {
            return render_xy_chart(
                XyChartType::Line,
                title.as_deref(),
                x_axis.as_deref(),
                y_axis.as_deref(),
                width.as_deref(),
                horizontal,
                show_data_label,
                aspect_ratio,
                false,      // add_line is false since we're a line chart
                bar,        // add_bar for line chart
                inverse,
                example,
                meta,
                data,
                args.json,
            );
        }
        Some(Command::Timeline {
            ref title,
            ref width,
            ref section,
            inverse,
            example,
            meta,
            ref events,
        }) => {
            return render_timeline(
                title.as_deref(),
                width.as_deref(),
                section,
                inverse,
                example,
                meta,
                events,
                args.json,
            );
        }
        Some(Command::StateDiagram {
            ref title,
            ref width,
            inverse,
            example,
            meta,
            ref transitions,
        }) => {
            return render_state_diagram(
                title.as_deref(),
                width.as_deref(),
                inverse,
                example,
                meta,
                transitions,
                args.json,
            );
        }
        Some(Command::Erd {
            ref title,
            ref width,
            ref entity,
            inverse,
            example,
            meta,
            ref relationships,
        }) => {
            return render_erd(
                title.as_deref(),
                width.as_deref(),
                entity,
                inverse,
                example,
                meta,
                relationships,
                args.json,
            );
        }
        Some(Command::Prose {
            ref content,
            no_wrap,
            margin_left,
            margin_right,
            alignment,
        }) => {
            return render_prose(content, no_wrap, margin_left, margin_right, alignment);
        }
        Some(Command::Quote {
            ref content,
            ref attribution,
            margin_left,
            margin_right,
            alignment,
        }) => {
            return render_quote(content, attribution.as_deref(), margin_left, margin_right, alignment);
        }
        Some(Command::List {
            ref items,
            ref bullet,
            no_hanging_indent,
            margin_left,
            margin_right,
            alignment,
        }) => {
            return render_list(items, bullet, no_hanging_indent, margin_left, margin_right, alignment);
        }
        None => {
            // Default behavior: content analysis or terminal metadata
        }
    }

    let content = if args.content.is_empty() {
        None
    } else {
        Some(args.content.join(" "))
    };

    if let Some(content) = content.as_deref() {
        let analysis = analyze_content(content);
        if args.json {
            println!("{}", serde_json::to_string_pretty(&analysis)?);
        } else {
            print_content_analysis(&analysis);
        }
        return Ok(());
    }

    let metadata = collect_metadata();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&metadata)?);
    } else {
        print_pretty(&metadata, args.verbose);
    }

    Ok(())
}

/// Handles the --completions flag.
///
/// If "help" is provided, shows setup instructions.
/// Otherwise, generates shell completion scripts.
fn handle_completions(shell_arg: &str) -> color_eyre::Result<()> {
    let shell_lower = shell_arg.to_lowercase();

    if shell_lower == "help" {
        print_completions_help();
        return Ok(());
    }

    let shell = match shell_lower.as_str() {
        "bash" => Shell::Bash,
        "elvish" => Shell::Elvish,
        "fish" => Shell::Fish,
        "powershell" | "pwsh" => Shell::PowerShell,
        "zsh" => Shell::Zsh,
        _ => {
            eprintln!(
                "error: invalid shell '{}'\n\nValid shells: bash, elvish, fish, powershell, zsh\n\nUse 'bt --completions help' for setup instructions.",
                shell_arg
            );
            std::process::exit(1);
        }
    };

    print_completions(shell);
    Ok(())
}

/// Prints shell completions to stdout.
fn print_completions(shell: Shell) {
    let mut cmd = Args::command();
    clap_complete::generate(shell, &mut cmd, "bt", &mut std::io::stdout());
}

/// Prints help about setting up shell completions.
fn print_completions_help() {
    println!(
        r#"bt Shell Completions Setup

Two methods are available for enabling tab completion:

DYNAMIC COMPLETIONS (recommended)
=================================
Dynamic completions call bt at completion time, providing:
- Image file filtering (only *.png, *.jpg, *.jpeg, *.gif)
- Always up-to-date with current bt version

Setup:
  Bash:  echo 'source <(COMPLETE=bash bt)' >> ~/.bashrc
  Zsh:   echo 'source <(COMPLETE=zsh bt)' >> ~/.zshrc
  Fish:  echo 'COMPLETE=fish bt | source' >> ~/.config/fish/config.fish

STATIC COMPLETIONS
==================
Static completions generate a script once. Faster but less features.

Setup:
  Bash:       bt --completions bash >> ~/.bashrc
  Zsh:        bt --completions zsh > ~/.zfunc/_bt
  Fish:       bt --completions fish > ~/.config/fish/completions/bt.fish
  PowerShell: bt --completions powershell >> $PROFILE

After setup, restart your shell or source the file to activate completions.
"#
    );
}

/// Creates a path completer that filters for image files.
///
/// Formats an axis label for Mermaid quadrant charts.
///
/// If the label contains ` --> `, it's split into left and right parts:
///   "Low --> High" becomes `"Low" --> "High"`
///
/// Otherwise, the entire label is quoted (appears at axis start):
///   "My Label" becomes `"My Label"`
fn format_axis_label(label: &str) -> String {
    if let Some((left, right)) = label.split_once(" --> ") {
        format!("\"{}\" --> \"{}\"", left.trim(), right.trim())
    } else {
        format!("\"{}\"", label)
    }
}

/// Completes files with extensions: png, jpg, jpeg, gif (case-insensitive).
/// Also completes directories to allow navigation.
fn image_completer() -> PathCompleter {
    PathCompleter::any().filter(|path| {
        // Always allow directories for navigation
        if path.is_dir() {
            return true;
        }

        // Check for image extensions
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                let ext_lower = ext.to_lowercase();
                matches!(ext_lower.as_str(), "png" | "jpg" | "jpeg" | "gif")
            })
    })
}

/// Render an image to the terminal.
///
/// Supports width specification syntax: "file.jpg|50%" or "file.jpg|80"
/// CLI `--width` flag takes precedence over inline spec.
fn render_image(image_spec: &str, cli_width: Option<&str>, meta: bool) -> color_eyre::Result<()> {
    use std::time::Instant;

    let start_time = Instant::now();

    // Parse the filepath and optional inline width
    let (filepath, inline_width_spec) = parse_filepath_and_width(image_spec)
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    // Resolve path relative to CWD
    let path = Path::new(&filepath);

    // Create the terminal image
    let mut term_image = TerminalImage::new(path)
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    // CLI --width takes precedence over inline spec (e.g., "file.jpg|50%")
    let effective_width_spec = cli_width.or(inline_width_spec.as_deref());
    if let Some(ws) = effective_width_spec {
        term_image.width = parse_width_spec(ws)
            .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
        term_image.width_raw = Some(format!("|{}", ws));
    }

    // Get terminal capabilities
    let terminal = Terminal::new();

    // Render the image
    let output = term_image.render_to_terminal(&terminal)
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    // Output the result
    print!("{}", output);

    // Output metadata if requested
    if meta {
        let render_time_ms = start_time.elapsed().as_millis() as u64;
        let absolute_path = std::fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf());
        let file_size_bytes = std::fs::metadata(path)
            .map(|m| m.len())
            .unwrap_or(0);

        let render_meta = RenderMeta {
            filename: absolute_path.to_string_lossy().to_string(),
            cache_hit: false, // Images are loaded directly, no caching
            file_size_bytes,
            render_time_ms,
        };

        eprintln!("{}", serde_json::to_string(&render_meta)?);
    }

    Ok(())
}

/// Example data for flowchart --example
/// Note: Each element is joined with newlines in the flowchart body
const FLOWCHART_EXAMPLE: &[&str] = &[
    "A[Start] --> B{Decision}",
    "B -->|Yes| C[Success]",
    "B -->|No| D[Retry]",
    "D --> B",
];
const FLOWCHART_EXAMPLE_CMD: &str = r#"bt flowchart "A[Start] --> B{Decision}" "B -->|Yes| C[Success]" "B -->|No| D[Retry]" "D --> B""#;

/// Display a mermaid diagram and optionally output metadata.
///
/// This helper function:
/// 1. Renders the diagram using the cached renderer
/// 2. Displays it in the terminal
/// 3. Optionally outputs metadata to stderr
///
/// Returns the render metadata (path, cache_hit, file_size, render_time) for further use.
fn display_mermaid_diagram(
    renderer: &MermaidRenderer,
    instructions: &str,
    diagram_type: &str,
    width: Option<&str>,
    meta: bool,
) -> color_eyre::Result<()> {
    use std::time::Instant;

    let start_time = Instant::now();

    // Render the diagram to a cached PNG file
    let (png_path, cache_hit) = match renderer.render_to_cached_png() {
        Ok((path, hit)) => (path, hit),
        Err(e) => {
            return handle_mermaid_error(e, instructions, diagram_type);
        }
    };

    let render_time_ms = start_time.elapsed().as_millis() as u64;

    // Parse width specification: default to 50% if not specified
    let image_width = match width {
        Some(w) => parse_width_spec(w).map_err(|e| color_eyre::eyre::eyre!("{}", e))?,
        None => ImageWidth::Percent(0.5),
    };

    // Use TerminalImage to display
    let terminal = Terminal::new();
    let term_image = TerminalImage::new(&png_path)
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?
        .with_width(image_width);

    match term_image.render_to_terminal(&terminal) {
        Ok(output) => print!("{}", output),
        Err(e) => {
            return Err(color_eyre::eyre::eyre!("Failed to display {}: {}", diagram_type, e));
        }
    }

    // Output metadata if requested
    if meta {
        let file_size_bytes = std::fs::metadata(&png_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let render_meta = RenderMeta {
            filename: png_path.to_string_lossy().to_string(),
            cache_hit,
            file_size_bytes,
            render_time_ms,
        };

        eprintln!("{}", serde_json::to_string(&render_meta)?);
    }

    // Let terminal settle after image rendering
    settle_terminal();

    Ok(())
}

/// Render a flowchart to the terminal.
///
/// Creates a Mermaid flowchart with the given content and renders it
/// using the MermaidRenderer. Default direction is left-right (LR),
/// use `vertical` for top-down (TD).
fn render_flowchart(
    vertical: bool,
    inverse: bool,
    title: Option<&str>,
    width: Option<&str>,
    example: bool,
    meta: bool,
    content: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let content: Vec<String> = if example {
        FLOWCHART_EXAMPLE.iter().map(|s| s.to_string()).collect()
    } else {
        content.to_vec()
    };

    let direction = if vertical { "TD" } else { "LR" };
    // Join content with newlines and indentation for proper Mermaid syntax
    let body = content.join("\n    ");

    // Build mermaid instructions with optional title frontmatter
    let instructions = if let Some(title) = title {
        format!(
            "---\ntitle: {}\n---\nflowchart {}\n    {}",
            title, direction, body
        )
    } else {
        format!("flowchart {}\n    {}", direction, body)
    };

    if json {
        let output = serde_json::json!({
            "type": "flowchart",
            "direction": direction,
            "inverse": inverse,
            "title": title,
            "width": width,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer based on inverse flag
    let renderer = if inverse {
        // Inverse: solid background with opposite theme
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        // Default: transparent background with theme matching terminal
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "flowchart", width, meta)?;

    // Print command used if example mode
    if example {
        print_example_command(FLOWCHART_EXAMPLE_CMD);
    }

    Ok(())
}

/// Example data for quadrant --example
const QUADRANT_EXAMPLE: &[&str] = &[
    "Campaign A: [0.3, 0.78]",
    "Campaign B: [0.45, 0.23]",
    "Campaign C: [0.57, 0.69]",
    "Campaign D: [0.78, 0.34]",
    "Campaign E: [0.40, 0.34]",
    "Campaign F: [0.65, 0.78]",
];
const QUADRANT_EXAMPLE_CMD: &str = r#"bt quadrant --title "Campaign Analysis" --x-axis "Low Reach --> High Reach" --y-axis "Low Engagement --> High Engagement" "Campaign A: [0.3, 0.78]" "Campaign B: [0.45, 0.23]" "Campaign C: [0.57, 0.69]" "Campaign D: [0.78, 0.34]" "Campaign E: [0.40, 0.34]" "Campaign F: [0.65, 0.78]""#;

/// Render a quadrant chart to the terminal.
///
/// Creates a Mermaid quadrantChart with the given configuration and data points,
/// then renders it using the MermaidRenderer.
#[allow(clippy::too_many_arguments)]
fn render_quadrant(
    x_axis: Option<&str>,
    y_axis: Option<&str>,
    title: Option<&str>,
    top_left: Option<&str>,
    top_right: Option<&str>,
    bottom_left: Option<&str>,
    bottom_right: Option<&str>,
    inverse: bool,
    width: Option<&str>,
    point_radius: Option<u32>,
    label_size: Option<u32>,
    theme: QuadrantTheme,
    q1_fill: Option<&str>,
    q2_fill: Option<&str>,
    q3_fill: Option<&str>,
    q4_fill: Option<&str>,
    example: bool,
    meta: bool,
    points: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::{MermaidConfig, MermaidTheme};
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (title, x_axis, y_axis, points): (Option<&str>, Option<&str>, Option<&str>, Vec<String>) = if example {
        (
            Some("Campaign Analysis"),
            Some("Low Reach --> High Reach"),
            Some("Low Engagement --> High Engagement"),
            QUADRANT_EXAMPLE.iter().map(|s| s.to_string()).collect(),
        )
    } else {
        (title, x_axis, y_axis, points.to_vec())
    };

    // Build the quadrantChart body
    let mut body_lines = Vec::new();

    // Title goes inside the chart body for quadrantChart (unlike other diagrams)
    if let Some(t) = title {
        body_lines.push(format!("    title \"{}\"", t));
    }

    // Axis labels: if contains " --> ", format as "Left" --> "Right"
    // Otherwise, quote the whole string for a centered label
    if let Some(x) = x_axis {
        body_lines.push(format!("    x-axis {}", format_axis_label(x)));
    }
    if let Some(y) = y_axis {
        body_lines.push(format!("    y-axis {}", format_axis_label(y)));
    }

    // Quadrant descriptions (1=top-left, 2=top-right, 3=bottom-left, 4=bottom-right)
    if let Some(tl) = top_left {
        body_lines.push(format!("    quadrant-1 \"{}\"", tl));
    }
    if let Some(tr) = top_right {
        body_lines.push(format!("    quadrant-2 \"{}\"", tr));
    }
    if let Some(bl) = bottom_left {
        body_lines.push(format!("    quadrant-3 \"{}\"", bl));
    }
    if let Some(br) = bottom_right {
        body_lines.push(format!("    quadrant-4 \"{}\"", br));
    }

    // Data points
    for point in &points {
        body_lines.push(format!("    {}", point));
    }

    let body = body_lines.join("\n");
    let instructions = format!("quadrantChart\n{}", body);

    if json {
        let output = serde_json::json!({
            "type": "quadrant",
            "x_axis": x_axis,
            "y_axis": y_axis,
            "title": title,
            "top_left": top_left,
            "top_right": top_right,
            "bottom_left": bottom_left,
            "bottom_right": bottom_right,
            "inverse": inverse,
            "width": width,
            "point_radius": point_radius,
            "label_size": label_size,
            "theme": theme.as_str(),
            "q1_fill": q1_fill,
            "q2_fill": q2_fill,
            "q3_fill": q3_fill,
            "q4_fill": q4_fill,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Build Mermaid config with styling options
    // Default label size: 18 for <= 6 points, 15 for > 6 points
    let config = {
        let mut cfg = MermaidConfig::new();

        // Apply point styling
        if let Some(r) = point_radius {
            cfg = cfg.with_point_radius(r);
        }
        let effective_label_size = label_size.unwrap_or(
            if points.len() <= 6 { 18 } else { 15 }
        );
        cfg = cfg.with_point_label_font_size(effective_label_size);

        // Apply theme preset (sets default quadrant colors based on terminal color mode)
        let color_mode = Terminal::color_mode();
        cfg = theme.apply(cfg, color_mode);

        // Apply individual fill overrides (these take precedence over theme)
        if let Some(color) = q1_fill {
            cfg = cfg.with_quadrant_fill(1, color);
        }
        if let Some(color) = q2_fill {
            cfg = cfg.with_quadrant_fill(2, color);
        }
        if let Some(color) = q3_fill {
            cfg = cfg.with_quadrant_fill(3, color);
        }
        if let Some(color) = q4_fill {
            cfg = cfg.with_quadrant_fill(4, color);
        }

        cfg
    };

    // Configure renderer based on inverse flag, applying config for point styling
    let renderer = if inverse {
        // Inverse: solid background with opposite theme
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
            .with_config(config)
    } else {
        // Default: transparent background with theme matching terminal
        MermaidRenderer::for_terminal(&instructions)
            .with_config(config)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "quadrant chart", width, meta)?;

    // Print command used if example mode
    if example {
        print_example_command(QUADRANT_EXAMPLE_CMD);
    }

    Ok(())
}

/// A parsed pie chart entry with optional color.
struct PieEntry {
    /// The Mermaid-formatted data line (e.g., `"Label" : value`)
    line: String,
    /// Optional hex color for this slice (e.g., `#3178c6`)
    color: Option<String>,
}

/// Parses pie chart data from various input formats.
///
/// Supports three formats:
/// 1. Simple: `"Label: value"` - quotes around label optional
/// 2. Semicolon-delimited: `"Label1: 10; Label2: 20"`
/// 3. Official Mermaid: `"\"Label\" : value"` - with quotes around label
///
/// Each format also supports an optional color suffix:
/// - `"Label: value color: #hex"` or `"Label: value #hex"`
///
/// Returns a vector of parsed entries with their optional colors.
fn parse_pie_data(data: &[String]) -> Vec<PieEntry> {
    let mut result = Vec::new();

    for item in data {
        // Check if this is a semicolon-delimited string
        if item.contains(';') {
            // Split by semicolon and process each part
            for part in item.split(';') {
                let part = part.trim();
                if !part.is_empty() && let Some(parsed) = parse_single_pie_entry(part) {
                    result.push(parsed);
                }
            }
        } else {
            // Single entry
            if let Some(parsed) = parse_single_pie_entry(item) {
                result.push(parsed);
            }
        }
    }

    result
}

/// Extracts a hex color from the end of a string.
///
/// Looks for patterns like:
/// - `color: #3178c6` or `color:#3178c6`
/// - `#3178c6` (standalone at end)
///
/// Returns (remaining_string, Some(color)) if found, or (original, None) if not.
fn extract_color(s: &str) -> (&str, Option<String>) {
    let s = s.trim();

    // Try "color: #hex" or "color:#hex" pattern first
    if let Some(color_idx) = s.to_lowercase().rfind("color:") {
        let before = s[..color_idx].trim();
        let color_part = s[color_idx + 6..].trim(); // Skip "color:"

        if let Some(color) = parse_hex_color(color_part) {
            return (before, Some(color));
        }
    }

    // Try standalone #hex at the end
    // Find the last whitespace and check if what follows is a hex color
    if let Some(last_space) = s.rfind(char::is_whitespace) {
        let potential_color = s[last_space + 1..].trim();
        if let Some(color) = parse_hex_color(potential_color) {
            return (s[..last_space].trim(), Some(color));
        }
    }

    (s, None)
}

/// Parses a hex color string, returning it normalized if valid.
///
/// Accepts: `#rgb`, `#rrggbb`, `#rrggbbaa`
fn parse_hex_color(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with('#') {
        return None;
    }

    let hex_part = &s[1..];
    // Valid lengths: 3 (#rgb), 6 (#rrggbb), or 8 (#rrggbbaa)
    if !matches!(hex_part.len(), 3 | 6 | 8) {
        return None;
    }

    // Check all characters are valid hex
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    Some(s.to_string())
}

/// Parses a single pie chart entry into Mermaid format with optional color.
///
/// Handles:
/// - `Label: value` → `"Label" : value`
/// - `"Label" : value` → `"Label" : value` (passthrough)
/// - `"Label": value` → `"Label" : value`
///
/// Also extracts optional color from end:
/// - `Label: value color: #hex`
/// - `Label: value #hex`
fn parse_single_pie_entry(entry: &str) -> Option<PieEntry> {
    let entry = entry.trim();
    if entry.is_empty() {
        return None;
    }

    // Extract color from the end first (before parsing the rest)
    let (entry_without_color, color) = extract_color(entry);

    // Check if it's already in official Mermaid format (starts with quote)
    if let Some(stripped) = entry_without_color.strip_prefix('"') {
        // Find the closing quote
        if let Some(close_quote_idx) = stripped.find('"') {
            let label = &stripped[..close_quote_idx];
            let rest = &stripped[close_quote_idx + 1..]; // Skip the closing quote

            // Find the colon and value
            if let Some(colon_idx) = rest.find(':') {
                let value = rest[colon_idx + 1..].trim();
                if !value.is_empty() {
                    return Some(PieEntry {
                        line: format!("\"{}\" : {}", label, value),
                        color,
                    });
                }
            }
        }
        // If parsing failed, try the simple format below
    }

    // Simple format: Label: value
    if let Some(colon_idx) = entry_without_color.find(':') {
        let label = entry_without_color[..colon_idx].trim();
        let value = entry_without_color[colon_idx + 1..].trim();

        if !label.is_empty() && !value.is_empty() {
            // Remove surrounding quotes if present
            let label = label.trim_matches('"');
            return Some(PieEntry {
                line: format!("\"{}\" : {}", label, value),
                color,
            });
        }
    }

    None
}

/// Builds the Mermaid init directive for pie chart colors.
///
/// If any entries have colors, generates:
/// `%%{init: {'themeVariables': {'pie1': '#color', 'pie2': '#color', ...}}}%%`
fn build_pie_init_directive(entries: &[PieEntry]) -> Option<String> {
    let color_vars: Vec<String> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            entry.color.as_ref().map(|c| format!("'pie{}': '{}'", i + 1, c))
        })
        .collect();

    if color_vars.is_empty() {
        None
    } else {
        Some(format!(
            "%%{{init: {{'themeVariables': {{{}}}}}}}%%",
            color_vars.join(", ")
        ))
    }
}

/// Example data for pie-chart --example
const PIE_CHART_EXAMPLE: &[&str] = &["TypeScript: 45 #3178C6", "Rust: 35 #A72145", "Python: 20"];
const PIE_CHART_EXAMPLE_CMD: &str = r#"bt pie-chart "TypeScript: 45 #3178C6" "Rust: 35 #A72145" "Python: 20""#;

/// Render a pie chart to the terminal.
///
/// Creates a Mermaid pie chart with the given data and renders it
/// using the MermaidRenderer.
fn render_pie_chart(
    inverse: bool,
    title: Option<&str>,
    width: Option<&str>,
    show_data: bool,
    example: bool,
    meta: bool,
    data: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let data: Vec<String> = if example {
        PIE_CHART_EXAMPLE.iter().map(|s| s.to_string()).collect()
    } else {
        data.to_vec()
    };

    // Parse the input data into Mermaid format (with optional colors)
    let parsed_entries = parse_pie_data(&data);

    if parsed_entries.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No valid data points provided. Use format: \"Label: value\""
        ));
    }

    // Build the init directive for custom colors (if any)
    let init_directive = build_pie_init_directive(&parsed_entries);

    // Build the pie chart body
    let show_data_str = if show_data { " showData" } else { "" };
    let title_line = title.map(|t| format!("    title {}", t)).unwrap_or_default();

    let data_lines: String = parsed_entries
        .iter()
        .map(|e| format!("    {}", e.line))
        .collect::<Vec<_>>()
        .join("\n");

    // Combine all parts: init directive (optional) + pie declaration + title (optional) + data
    let mut instructions_parts = Vec::new();

    if let Some(ref init) = init_directive {
        instructions_parts.push(init.clone());
    }

    if title_line.is_empty() {
        instructions_parts.push(format!("pie{}\n{}", show_data_str, data_lines));
    } else {
        instructions_parts.push(format!("pie{}\n{}\n{}", show_data_str, title_line, data_lines));
    }

    let instructions = instructions_parts.join("\n");

    if json {
        let output = serde_json::json!({
            "type": "pie-chart",
            "inverse": inverse,
            "title": title,
            "width": width,
            "show_data": show_data,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer based on inverse flag
    let renderer = if inverse {
        // Inverse: solid background with opposite theme
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        // Default: transparent background with theme matching terminal
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "pie chart", width, meta)?;

    // Print command used if example mode
    if example {
        print_example_command(PIE_CHART_EXAMPLE_CMD);
    }

    Ok(())
}

/// Example data for git-graph --example
const GIT_GRAPH_EXAMPLE: &[&str] = &[
    "commit",
    "commit",
    "branch feature",
    "checkout feature",
    "commit",
    "commit",
    "checkout main",
    "commit",
    "merge feature",
    "commit",
];
const GIT_GRAPH_EXAMPLE_CMD: &str = r#"bt git-graph "commit" "commit" "branch feature" "checkout feature" "commit" "commit" "checkout main" "commit" "merge feature" "commit""#;

/// Render a git graph to the terminal.
///
/// Creates a Mermaid gitGraph with the given commands and renders it
/// using the MermaidRenderer.
fn render_git_graph(
    inverse: bool,
    title: Option<&str>,
    width: Option<&str>,
    example: bool,
    meta: bool,
    commands: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let commands: Vec<String> = if example {
        GIT_GRAPH_EXAMPLE.iter().map(|s| s.to_string()).collect()
    } else {
        commands.to_vec()
    };

    let body = commands
        .iter()
        .map(|cmd| format!("    {}", cmd))
        .collect::<Vec<_>>()
        .join("\n");

    // Build mermaid instructions with optional title frontmatter
    let instructions = if let Some(title) = title {
        format!("---\ntitle: {}\n---\ngitGraph\n{}", title, body)
    } else {
        format!("gitGraph\n{}", body)
    };

    if json {
        let output = serde_json::json!({
            "type": "git-graph",
            "inverse": inverse,
            "title": title,
            "width": width,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer based on inverse flag
    let renderer = if inverse {
        // Inverse: solid background with opposite theme
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        // Default: transparent background with theme matching terminal
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "git-graph", width, meta)?;

    // Print command used if example mode
    if example {
        print_example_command(GIT_GRAPH_EXAMPLE_CMD);
    }

    Ok(())
}

/// XY chart type selector
#[derive(Debug, Clone, Copy, PartialEq)]
enum XyChartType {
    Bar,
    Line,
}

/// Example data for bar-chart --example
const BAR_CHART_EXAMPLE: &[&str] = &["12", "28", "45", "38", "22", "55"];
const BAR_CHART_EXAMPLE_CMD: &str = "bt bar-chart --title \"Monthly Revenue\" --x-axis \"Jan,Feb,Mar,Apr,May,Jun\" --y-axis \"$ (thousands)\" 12 28 45 38 22 55";

/// Example data for line-chart --example
const LINE_CHART_EXAMPLE: &[&str] = &["20", "22", "19", "23", "25", "21", "24"];
const LINE_CHART_EXAMPLE_CMD: &str = "bt line-chart --title \"Weekly Temperature\" --x-axis \"Mon,Tue,Wed,Thu,Fri,Sat,Sun\" --y-axis \"°C\" 20 22 19 23 25 21 24";

/// Render an XY chart (bar or line) to the terminal.
///
/// Uses Mermaid's xychart-beta syntax.
#[allow(clippy::too_many_arguments)]
fn render_xy_chart(
    chart_type: XyChartType,
    title: Option<&str>,
    x_axis: Option<&str>,
    y_axis: Option<&str>,
    width: Option<&str>,
    horizontal: bool,
    show_data_label: bool,
    aspect_ratio: Option<f32>,
    add_line: bool,
    add_bar: bool,
    inverse: bool,
    example: bool,
    meta: bool,
    data: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (data, use_example_labels): (Vec<String>, bool) = if example {
        let example_data = match chart_type {
            XyChartType::Bar => BAR_CHART_EXAMPLE,
            XyChartType::Line => LINE_CHART_EXAMPLE,
        };
        (example_data.iter().map(|s| s.to_string()).collect(), true)
    } else {
        (data.to_vec(), false)
    };

    // Parse input data
    let values = parse_xy_data(&data)?;

    if values.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No valid data values provided. Use format: \"1 2 3\" or \"[1,2,3]\" or \"1,2,3\""
        ));
    }

    // Get example titles/labels for example mode
    let (eff_title, eff_x_axis, eff_y_axis) = if use_example_labels {
        match chart_type {
            XyChartType::Bar => (
                Some("Monthly Revenue"),
                Some("Jan,Feb,Mar,Apr,May,Jun"),
                Some("$ (thousands)"),
            ),
            XyChartType::Line => (
                Some("Weekly Temperature"),
                Some("Mon,Tue,Wed,Thu,Fri,Sat,Sun"),
                Some("°C"),
            ),
        }
    } else {
        (title, x_axis, y_axis)
    };

    // Build init directive for configuration
    let aspect = aspect_ratio.unwrap_or(1.5);
    let init_config = format!(
        "%%{{init: {{\"xychart\": {{\"showTitle\": {}, \"xAxis\": {{\"showLabel\": {}}}, \"yAxis\": {{\"showLabel\": {}}}}}}}}}%%",
        eff_title.is_some(),
        eff_x_axis.is_some(),
        eff_y_axis.is_some()
    );

    // Build chart declaration
    let orientation = if horizontal { "horizontal" } else { "" };
    let chart_decl = format!("xychart-beta {}", orientation).trim().to_string();

    // Build x-axis line
    let x_axis_line = if let Some(labels) = eff_x_axis {
        // Check if it contains commas (categories) or is just a label
        if labels.contains(',') {
            let cats: Vec<&str> = labels.split(',').map(|s| s.trim()).collect();
            format!("    x-axis [{}]", cats.join(", "))
        } else {
            format!("    x-axis \"{}\"", labels)
        }
    } else {
        // Generate default labels based on data count
        let default_labels: Vec<String> = (1..=values.len()).map(|i| i.to_string()).collect();
        format!("    x-axis [{}]", default_labels.join(", "))
    };

    // Build y-axis line
    let y_axis_line = if let Some(label) = eff_y_axis {
        format!("    y-axis \"{}\"", label)
    } else {
        String::new()
    };

    // Build title line
    let title_line = eff_title.map(|t| format!("    title \"{}\"", t)).unwrap_or_default();

    // Build data series
    let data_str: String = values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");

    let primary_series = match chart_type {
        XyChartType::Bar => format!("    bar [{}]", data_str),
        XyChartType::Line => format!("    line [{}]", data_str),
    };

    let secondary_series = if add_line && chart_type == XyChartType::Bar {
        format!("\n    line [{}]", data_str)
    } else if add_bar && chart_type == XyChartType::Line {
        format!("\n    bar [{}]", data_str)
    } else {
        String::new()
    };

    // Combine all parts
    let mut parts = vec![init_config, chart_decl];
    if !title_line.is_empty() {
        parts.push(title_line);
    }
    parts.push(x_axis_line);
    if !y_axis_line.is_empty() {
        parts.push(y_axis_line);
    }
    parts.push(primary_series);
    if !secondary_series.is_empty() {
        parts.push(secondary_series.trim().to_string());
    }

    let instructions = parts.join("\n");

    if json {
        let output = serde_json::json!({
            "type": match chart_type {
                XyChartType::Bar => "bar-chart",
                XyChartType::Line => "line-chart",
            },
            "inverse": inverse,
            "title": eff_title,
            "x_axis": eff_x_axis,
            "y_axis": eff_y_axis,
            "horizontal": horizontal,
            "show_data_label": show_data_label,
            "aspect_ratio": aspect,
            "add_line": add_line,
            "add_bar": add_bar,
            "values": values,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer based on inverse flag
    let renderer = if inverse {
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    let chart_name = match chart_type {
        XyChartType::Bar => "bar chart",
        XyChartType::Line => "line chart",
    };
    display_mermaid_diagram(&renderer, &instructions, chart_name, width, meta)?;

    // Print command used if example mode
    if example {
        let cmd = match chart_type {
            XyChartType::Bar => BAR_CHART_EXAMPLE_CMD,
            XyChartType::Line => LINE_CHART_EXAMPLE_CMD,
        };
        print_example_command(cmd);
    }

    Ok(())
}

/// Parse XY chart data from various input formats.
///
/// Supports:
/// - JSON array: "[1, 8, 7, 5]"
/// - Comma-separated: "1,8,7,5" or "1, 8, 7, 5"
/// - Space-separated arguments: "1" "8" "7" "5"
fn parse_xy_data(data: &[String]) -> color_eyre::Result<Vec<f64>> {
    let mut values = Vec::new();

    for item in data {
        let trimmed = item.trim();

        // Try JSON array first
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len() - 1];
            for part in inner.split(',') {
                let v: f64 = part.trim().parse().map_err(|_| {
                    color_eyre::eyre::eyre!("Invalid number in JSON array: '{}'", part.trim())
                })?;
                values.push(v);
            }
            continue;
        }

        // Try comma-separated
        if trimmed.contains(',') {
            for part in trimmed.split(',') {
                let v: f64 = part.trim().parse().map_err(|_| {
                    color_eyre::eyre::eyre!("Invalid number: '{}'", part.trim())
                })?;
                values.push(v);
            }
            continue;
        }

        // Single value
        let v: f64 = trimmed.parse().map_err(|_| {
            color_eyre::eyre::eyre!("Invalid number: '{}'", trimmed)
        })?;
        values.push(v);
    }

    Ok(values)
}

/// Example data for timeline --example
const TIMELINE_EXAMPLE: &[&str] = &[
    "2002: LinkedIn",
    "2004: Facebook",
    "2005: YouTube",
    "2006: Twitter",
    "2010: Instagram",
    "2011: Snapchat",
];
const TIMELINE_EXAMPLE_CMD: &str = "bt timeline --title \"Social Media History\" \"2002: LinkedIn\" \"2004: Facebook\" \"2005: YouTube\" \"2006: Twitter\" \"2010: Instagram\" \"2011: Snapchat\"";

/// Render a timeline diagram to the terminal.
fn render_timeline(
    title: Option<&str>,
    width: Option<&str>,
    sections: &[String],
    inverse: bool,
    example: bool,
    meta: bool,
    events: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (events, eff_title): (Vec<String>, Option<&str>) = if example {
        (
            TIMELINE_EXAMPLE.iter().map(|s| s.to_string()).collect(),
            Some("Social Media History"),
        )
    } else {
        (events.to_vec(), title)
    };

    if events.is_empty() && sections.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No events provided. Use format: \"YYYY: Event description\""
        ));
    }

    // Validate event format
    for event in &events {
        if !event.contains(':') {
            return Err(color_eyre::eyre::eyre!(
                "Invalid event format '{}'. Expected 'YYYY: Description'",
                event
            ));
        }
    }

    // Build the timeline
    let mut lines = vec!["timeline".to_string()];

    if let Some(t) = eff_title {
        lines.push(format!("    title {}", t));
    }

    // If no sections, add all events directly
    if sections.is_empty() {
        for event in &events {
            lines.push(format!("    {}", event));
        }
    } else {
        // With sections, we need to interleave section headers and events
        // For now, put all events under the first section if sections are provided
        // Users can use multiple --section flags for grouping
        for (i, section) in sections.iter().enumerate() {
            lines.push(format!("    section {}", section));
            // Put a portion of events under each section
            let events_per_section = events.len().div_ceil(sections.len());
            let start = i * events_per_section;
            let end = ((i + 1) * events_per_section).min(events.len());
            for event in events.get(start..end).unwrap_or(&[]) {
                lines.push(format!("        {}", event));
            }
        }
    }

    let instructions = lines.join("\n");

    if json {
        let output = serde_json::json!({
            "type": "timeline",
            "inverse": inverse,
            "title": eff_title,
            "sections": sections,
            "events": events,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer
    let renderer = if inverse {
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "timeline", width, meta)?;

    if example {
        print_example_command(TIMELINE_EXAMPLE_CMD);
    }

    Ok(())
}

/// Example data for state-diagram --example
const STATE_DIAGRAM_EXAMPLE: &[&str] = &[
    "[*] --> Idle",
    "Idle --> Running: start",
    "Running --> Idle: stop",
    "Running --> Error: failure",
    "Error --> Idle: reset",
    "Idle --> [*]: shutdown",
];
const STATE_DIAGRAM_EXAMPLE_CMD: &str = "bt state-diagram --title \"Process States\" \"[*] --> Idle\" \"Idle --> Running: start\" \"Running --> Idle: stop\" \"Running --> Error: failure\" \"Error --> Idle: reset\" \"Idle --> [*]: shutdown\"";

/// Render a state diagram to the terminal.
fn render_state_diagram(
    title: Option<&str>,
    width: Option<&str>,
    inverse: bool,
    example: bool,
    meta: bool,
    transitions: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (transitions, eff_title): (Vec<String>, Option<&str>) = if example {
        (
            STATE_DIAGRAM_EXAMPLE.iter().map(|s| s.to_string()).collect(),
            Some("Process States"),
        )
    } else {
        (transitions.to_vec(), title)
    };

    if transitions.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No transitions provided. Use format: \"State1 --> State2\" or \"[*] --> State\""
        ));
    }

    // Build the state diagram
    let mut lines = vec!["stateDiagram-v2".to_string()];

    // Add title if provided (using note or direction for now, title isn't directly supported)
    // Actually, stateDiagram doesn't have a title directive, we'll skip it for the diagram itself
    // but include it in JSON output

    for transition in &transitions {
        lines.push(format!("    {}", transition));
    }

    let instructions = lines.join("\n");

    if json {
        let output = serde_json::json!({
            "type": "state-diagram",
            "inverse": inverse,
            "title": eff_title,
            "transitions": transitions,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer
    let renderer = if inverse {
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "state diagram", width, meta)?;

    if example {
        print_example_command(STATE_DIAGRAM_EXAMPLE_CMD);
    }

    Ok(())
}

/// Example data for erd --example
/// Note: Mermaid ERD requires each attribute on its own line inside the entity block
const ERD_EXAMPLE_ENTITIES: &[&str] = &[
    "Customer {\n        int id PK\n        string name\n        string email\n    }",
    "Order {\n        int id PK\n        date orderDate\n        int customerId FK\n    }",
    "Product {\n        int id PK\n        string name\n        decimal price\n    }",
    "OrderItem {\n        int orderId FK\n        int productId FK\n        int quantity\n    }",
];
const ERD_EXAMPLE_RELATIONSHIPS: &[&str] = &[
    "Customer ||--o{ Order : places",
    "Order ||--|{ OrderItem : contains",
    "Product ||--o{ OrderItem : \"ordered in\"",
];
const ERD_EXAMPLE_CMD: &str = "bt erd --title \"E-Commerce Schema\" \\\n  --entity \"Customer { int id PK }\" \\\n  --entity \"Order { int id PK }\" \\\n  \"Customer ||--o{ Order : places\"";

/// Render an ERD to the terminal.
fn render_erd(
    title: Option<&str>,
    width: Option<&str>,
    entities: &[String],
    inverse: bool,
    example: bool,
    meta: bool,
    relationships: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (entities, relationships, eff_title): (Vec<String>, Vec<String>, Option<&str>) = if example {
        (
            ERD_EXAMPLE_ENTITIES.iter().map(|s| s.to_string()).collect(),
            ERD_EXAMPLE_RELATIONSHIPS.iter().map(|s| s.to_string()).collect(),
            Some("E-Commerce Schema"),
        )
    } else {
        (entities.to_vec(), relationships.to_vec(), title)
    };

    if relationships.is_empty() && entities.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No relationships or entities provided. Use format: \"Entity1 ||--o{{ Entity2 : label\""
        ));
    }

    // Build the ERD
    let mut lines = vec!["erDiagram".to_string()];

    // Add title if provided
    if let Some(t) = eff_title {
        // ERD doesn't have native title support, but we can add it as a note
        // For now, we'll just skip it in the diagram
        let _ = t; // suppress unused warning
    }

    // Add entity definitions
    for entity in &entities {
        lines.push(format!("    {}", entity));
    }

    // Add relationships
    for rel in &relationships {
        lines.push(format!("    {}", rel));
    }

    let instructions = lines.join("\n");

    if json {
        let output = serde_json::json!({
            "type": "erd",
            "inverse": inverse,
            "title": eff_title,
            "entities": entities,
            "relationships": relationships,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer
    let renderer = if inverse {
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "ERD", width, meta)?;

    if example {
        print_example_command(ERD_EXAMPLE_CMD);
    }

    Ok(())
}

/// Handle Mermaid rendering errors with user-friendly output.
///
/// Parses mmdc errors to extract syntax information and formats
/// them nicely without JavaScript callstacks.
fn handle_mermaid_error(
    error: biscuit_terminal::components::mermaid::MermaidRenderError,
    instructions: &str,
    diagram_type: &str,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidRenderError;

    // Check for NO_COLOR
    let no_color = std::env::var("NO_COLOR").is_ok();
    let red = if no_color { "" } else { "\x1b[31m" };
    let bold = if no_color { "" } else { "\x1b[1m" };
    let dim = if no_color { "" } else { "\x1b[2m" };
    let reset = if no_color { "" } else { "\x1b[0m" };

    match error {
        MermaidRenderError::MmdcExecutionFailed { stderr, .. } => {
            // Check if this is a parse/syntax error
            if stderr.contains("Parse error") || stderr.contains("Expecting") {
                // Add breathing room before error
                eprintln!();
                eprintln!("{}{}Error:{} Mermaid Syntax Error\n", red, bold, reset);

                // Extract useful lines from stderr (skip JS callstack and useless line numbers)
                for line in stderr.lines() {
                    // Include:
                    // - Context lines showing actual mermaid code (starts with ...)
                    // - Error pointer lines (contains ^ and dashes)
                    // - "Expecting" lines describing what was expected
                    // Skip: "Error: Parse error on line X:", JS callstack lines
                    let is_context_line = line.starts_with("...");
                    let is_pointer_line =
                        line.contains("^") && line.chars().filter(|c| *c == '-').count() > 3;
                    let is_expecting_line =
                        line.starts_with("Expecting") || line.contains("Expecting '");

                    if is_context_line || is_pointer_line || is_expecting_line {
                        eprintln!("{}", line);
                    }
                }

                // Show the mermaid block that was defined
                eprintln!(
                    "\n{}Mermaid {} was defined as:{}\n",
                    dim, diagram_type, reset
                );
                eprintln!("```mermaid\n{}\n```", instructions);
            } else {
                // Non-syntax error, show the full error (with breathing room)
                eprintln!();
                eprintln!("{}{}Error:{} {}", red, bold, reset, stderr);
            }
        }
        MermaidRenderError::MmdcNotFound => {
            eprintln!(
                "{}{}Error:{} mmdc CLI not found.\n\nInstall with: npm install -g @mermaid-js/mermaid-cli",
                red, bold, reset
            );
        }
        MermaidRenderError::NpmNotFound => {
            eprintln!(
                "{}{}Error:{} npm not found.\n\nInstall Node.js and npm to render Mermaid diagrams.",
                red, bold, reset
            );
        }
        _ => {
            eprintln!("{}{}Error:{} {}", red, bold, reset, error);
        }
    }

    // Return error to get non-zero exit code
    std::process::exit(1);
}

fn collect_metadata() -> TerminalMetadata {
    let terminal = Terminal::new();

    // Get colors
    let bg_color = osc_queries::bg_color().map(|c| ColorInfo {
        r: c.r,
        g: c.g,
        b: c.b,
        hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
    });

    let text_color = osc_queries::text_color().map(|c| ColorInfo {
        r: c.r,
        g: c.g,
        b: c.b,
        hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
    });

    let cursor_color = osc_queries::cursor_color().map(|c| ColorInfo {
        r: c.r,
        g: c.g,
        b: c.b,
        hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
    });

    // Get distro info
    let distro = terminal.distro.as_ref().map(|d| DistroInfo {
        id: d.id.clone(),
        name: d.name.clone(),
        version: d.version.clone(),
        codename: d.codename.clone(),
        family: d.family.to_string(),
    });

    TerminalMetadata {
        app: format!("{:?}", terminal.app),
        os: terminal.os.to_string(),
        distro,
        width: terminal.width(),
        height: terminal.height(),
        is_tty: terminal.is_tty,
        is_ci: terminal.is_ci,
        color_depth: format!("{:?}", terminal.color_depth),
        color_mode: format!("{:?}", Terminal::color_mode()),
        bg_color,
        text_color,
        cursor_color,
        font: terminal.font.clone(),
        font_size: terminal.font_size,
        is_nerd_font: terminal.is_nerd_font,
        font_ligatures: terminal
            .font_ligatures
            .as_ref()
            .map(|ligatures| ligatures.iter().map(|l| format!("{:?}", l)).collect()),
        ligatures_likely: fonts::ligature_support_likely(),

        supports_italic: terminal.supports_italic,
        image_support: format!("{:?}", terminal.image_support),
        underline_support: UnderlineInfo {
            straight: terminal.underline_support.straight,
            double: terminal.underline_support.double,
            curly: terminal.underline_support.curly,
            dotted: terminal.underline_support.dotted,
            dashed: terminal.underline_support.dashed,
            colored: terminal.underline_support.colored,
        },
        osc_link_support: terminal.osc_link_support,
        osc10_fg_color: osc_queries::osc10_support(),
        osc11_bg_color: osc_queries::osc11_support(),
        osc12_cursor_color: osc_queries::osc12_support(),
        osc52_clipboard: clipboard::osc52_support(),
        mode_2027_graphemes: mode_2027::supports_mode_2027(),
        multiplex: format_multiplex(multiplex_support()),
        connection: format_connection(&terminal.remote),
        locale_raw: terminal.locale.raw().map(|s| s.to_string()),
        locale_tag: terminal.locale.tag().map(|s| s.to_string()),
        char_encoding: format!("{:?}", terminal.char_encoding),
        config_file: terminal
            .config_file
            .as_ref()
            .map(|p| p.display().to_string()),
    }
}

fn analyze_content(content: &str) -> ContentAnalysis {
    let stripped = escape_codes::strip_escape_codes(content);
    let line_lengths: Vec<u32> = stripped
        .split('\n')
        .map(|line| line.chars().count() as u32)
        .collect();
    let line_count = line_lengths.len() as u32;
    let total_length = line_lengths.iter().copied().sum();

    ContentAnalysis {
        line_count,
        line_lengths,
        contains_color_escape_codes: escape_codes::strip_color_codes(content) != content,
        contains_osc8_links: eval::has_osc8_link(content),
        total_length,
    }
}

fn print_content_analysis(analysis: &ContentAnalysis) {
    let no_color = std::env::var("NO_COLOR").is_ok();
    let bold = if no_color { "" } else { "\x1b[1m" };
    let dim = if no_color { "" } else { "\x1b[2m" };
    let reset = if no_color { "" } else { "\x1b[0m" };
    let green = if no_color { "" } else { "\x1b[32m" };

    let yes = format!("{}yes{}", green, reset);
    let no_mark = format!("{}no{}", dim, reset);
    let check = |b: bool| if b { &yes } else { &no_mark };

    let line_lengths = analysis
        .line_lengths
        .iter()
        .map(|len| len.to_string())
        .collect::<Vec<String>>()
        .join(", ");

    println!();
    println!("{}Content Analysis{}", bold, reset);
    println!("{}══════════════════{}", dim, reset);
    println!("  Lines:        {}", analysis.line_count);
    println!("  Line lengths: {}", line_lengths);
    println!("  Total length: {}", analysis.total_length);
    println!(
        "  Color codes:  {}",
        check(analysis.contains_color_escape_codes)
    );
    println!("  OSC8 links:   {}", check(analysis.contains_osc8_links));
    println!();
}

fn format_connection(conn: &Connection) -> ConnectionInfo {
    match conn {
        Connection::Local => ConnectionInfo::Local,
        Connection::SshClient(ssh) => ConnectionInfo::Ssh {
            host: ssh.host.clone(),
            source_port: ssh.source_port,
            server_port: ssh.server_port,
            tty_path: ssh.tty_path.clone(),
        },
        Connection::MoshClient(mosh) => ConnectionInfo::Mosh {
            connection: mosh.connection.clone(),
        },
    }
}

fn format_multiplex(m: MultiplexSupport) -> String {
    match m {
        MultiplexSupport::None => "None".to_string(),
        MultiplexSupport::Native { .. } => "Native".to_string(),
        MultiplexSupport::Tmux { .. } => "tmux".to_string(),
        MultiplexSupport::Zellij { .. } => "Zellij".to_string(),
    }
}

fn print_pretty(metadata: &TerminalMetadata, verbose: bool) {
    // Respect NO_COLOR environment variable
    let no_color = std::env::var("NO_COLOR").is_ok();

    let bold = if no_color { "" } else { "\x1b[1m" };
    let dim = if no_color { "" } else { "\x1b[2m" };
    let reset = if no_color { "" } else { "\x1b[0m" };
    let green = if no_color { "" } else { "\x1b[32m" };
    let yellow = if no_color { "" } else { "\x1b[33m" };
    let blue = if no_color { "" } else { "\x1b[34m" };

    println!();
    println!("{}Terminal Metadata{}", bold, reset);
    println!("{}═══════════════════════════════════════{}", dim, reset);

    // Basic info section
    println!("\n{}{}Basic Info{}", bold, blue, reset);
    println!("  App:        {}", metadata.app);
    println!("  OS:         {}", metadata.os);
    if let Some(distro) = &metadata.distro {
        println!("  Distro:     {} ({})", distro.name, distro.family);
    }
    println!("  Size:       {} x {}", metadata.width, metadata.height);
    println!(
        "  Is TTY:     {}",
        if metadata.is_tty {
            format!("{}yes{}", green, reset)
        } else {
            "no".to_string()
        }
    );
    println!(
        "  In CI:      {}",
        if metadata.is_ci {
            format!("{}yes{}", yellow, reset)
        } else {
            "no".to_string()
        }
    );

    // Font section (always displayed)
    println!("\n{}{}Fonts{}", bold, blue, reset);
    if let Some(font) = &metadata.font {
        println!("  Name:       {}", font);
    } else {
        println!("  Name:       {}n/a{}", dim, reset);
    }
    if let Some(size) = metadata.font_size {
        println!("  Size:       {}pt", size);
    } else {
        println!("  Size:       {}n/a{}", dim, reset);
    }
    println!(
        "  Nerd Font:  {}",
        match metadata.is_nerd_font {
            Some(true) => format!("{}yes{}", green, reset),
            Some(false) => "no".to_string(),
            None => format!("{}unknown{}", dim, reset),
        }
    );
    println!(
        "  Ligatures:  {}",
        if metadata.ligatures_likely {
            format!("{}likely{}", green, reset)
        } else {
            format!("{}unlikely{}", dim, reset)
        }
    );

    // Color section
    println!("\n{}{}Colors{}", bold, blue, reset);
    println!("  Depth:      {}", metadata.color_depth);
    println!("  Mode:       {}", metadata.color_mode);
    if let Some(bg) = &metadata.bg_color {
        println!(
            "  Background: {} ({}, {}, {})",
            bg.hex.as_ref().unwrap_or(&"?".to_string()),
            bg.r,
            bg.g,
            bg.b
        );
    }
    if let Some(fg) = &metadata.text_color {
        println!(
            "  Foreground: {} ({}, {}, {})",
            fg.hex.as_ref().unwrap_or(&"?".to_string()),
            fg.r,
            fg.g,
            fg.b
        );
    }
    if let Some(cursor) = &metadata.cursor_color {
        println!(
            "  Cursor:     {} ({}, {}, {})",
            cursor.hex.as_ref().unwrap_or(&"?".to_string()),
            cursor.r,
            cursor.g,
            cursor.b
        );
    }

    // Features section
    println!("\n{}{}Features{}", bold, blue, reset);
    let yes = format!("{}yes{}", green, reset);
    let no_mark = format!("{}no{}", dim, reset);
    let check = |b: bool| if b { &yes } else { &no_mark };

    println!("  Italics:      {}", check(metadata.supports_italic));
    println!("  Images:       {}", metadata.image_support);
    println!("  OSC8 Links:   {}", check(metadata.osc_link_support));
    println!("  OSC10 FG:     {}", check(metadata.osc10_fg_color));
    println!("  OSC11 BG:     {}", check(metadata.osc11_bg_color));
    println!("  OSC12 Cursor: {}", check(metadata.osc12_cursor_color));
    println!("  OSC52 Clip:   {}", check(metadata.osc52_clipboard));
    println!("  Mode 2027:    {}", check(metadata.mode_2027_graphemes));

    // Underline section
    println!("\n{}{}Underline Support{}", bold, blue, reset);
    println!(
        "  Straight:   {}",
        check(metadata.underline_support.straight)
    );
    println!("  Double:     {}", check(metadata.underline_support.double));
    println!("  Curly:      {}", check(metadata.underline_support.curly));
    println!("  Dotted:     {}", check(metadata.underline_support.dotted));
    println!("  Dashed:     {}", check(metadata.underline_support.dashed));
    println!(
        "  Colored:    {}",
        check(metadata.underline_support.colored)
    );

    // Reserved for future verbose-only output
    let _ = verbose;

    // Multiplexing
    println!("\n{}{}Multiplexing{}", bold, blue, reset);
    println!("  Type:       {}", metadata.multiplex);

    // Connection
    println!("\n{}{}Connection{}", bold, blue, reset);
    match &metadata.connection {
        ConnectionInfo::Local => {
            println!("  Type:       {}Local{}", green, reset);
        }
        ConnectionInfo::Ssh {
            host,
            source_port,
            server_port,
            tty_path,
        } => {
            println!("  Type:       {}SSH{}", yellow, reset);
            println!("  Host:       {}", host);
            println!("  Ports:      {} -> {}", source_port, server_port);
            if let Some(tty) = tty_path {
                println!("  TTY:        {}", tty);
            }
        }
        ConnectionInfo::Mosh { connection } => {
            println!("  Type:       {}Mosh{}", yellow, reset);
            println!("  Connection: {}", connection);
        }
    }

    // Locale & Encoding
    println!("\n{}{}Locale{}", bold, blue, reset);
    let na = format!("{}n/a{}", dim, reset);
    println!(
        "  Raw:        {}",
        metadata.locale_raw.as_deref().unwrap_or(&na)
    );
    println!(
        "  Tag:        {}",
        metadata.locale_tag.as_deref().unwrap_or(&na)
    );
    println!("  Encoding:   {}", metadata.char_encoding);

    // Config
    if let Some(config) = &metadata.config_file {
        println!("\n{}{}Config{}", bold, blue, reset);
        println!("  File:       {}", config);
    }

    println!();
}

/// Render prose content with styling tokens to the terminal.
fn render_prose(
    content: &[String],
    no_wrap: bool,
    margin_left: Option<u32>,
    margin_right: Option<u32>,
    alignment: Option<biscuit_terminal::utils::layout::Alignment>,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::Renderable;
    use biscuit_terminal::utils::layout::{Margin, WordWrap};

    // Join all content pieces with spaces
    let text = content.join(" ");

    if text.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No content provided. Usage: bt prose \"Hello {{bold}}world{{reset}}!\""
        ));
    }

    // Unescape common escape sequences (shell passes literal \n, \t, etc.)
    let text = text
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r");

    // Build the Prose component
    let mut prose = Prose::new(&text);

    // Configure word wrapping
    if no_wrap {
        prose = prose.with_word_wrap(WordWrap::None);
    } else {
        prose = prose.with_word_wrap(WordWrap::WrapProse(None, None));
    }

    // Configure margins
    if let Some(left) = margin_left {
        prose = prose.with_left_margin(Margin::Chars(left));
    }
    if let Some(right) = margin_right {
        prose = prose.with_right_margin(Margin::Chars(right));
    }

    // Configure alignment
    if let Some(align) = alignment {
        prose = prose.alignment(align);
    }

    // Render using fallback_render for terminal-aware output
    let term = Terminal::new();
    let output = prose.fallback_render(&term);

    println!("{}", output);

    Ok(())
}

/// Render prose content inside a block quote.
fn render_quote(
    content: &[String],
    attribution: Option<&str>,
    margin_left: Option<u32>,
    margin_right: Option<u32>,
    alignment: Option<biscuit_terminal::utils::layout::Alignment>,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::block_quote::BlockQuote;
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
    use biscuit_terminal::utils::layout::Margin;
    use std::sync::Arc;

    // Join all content pieces with spaces
    let text = content.join(" ");

    if text.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No content provided. Usage: bt quote \"To be or not to be\" --attribution \"Shakespeare\""
        ));
    }

    // Unescape common escape sequences (shell passes literal \n, \t, etc.)
    let text = text
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r");

    // Build the Prose component for the content
    let prose = Prose::new(&text);

    // Build the BlockQuote with the Prose content
    let mut quote = BlockQuote::new(
        RenderableContent::Component(Arc::new(prose)),
        attribution,
    );

    // Configure margins
    if let Some(left) = margin_left {
        quote = quote.left_margin(Margin::Chars(left));
    }
    if let Some(right) = margin_right {
        quote = quote.right_margin(Margin::Chars(right));
    }

    // Configure alignment
    if let Some(align) = alignment {
        quote = quote.alignment(align);
    }

    // Render using fallback_render for terminal-aware output
    let term = Terminal::new();
    let output = quote.fallback_render(&term);

    println!("{}", output);

    Ok(())
}

/// Render a bulleted list with hanging indents.
fn render_list(
    items: &[String],
    bullet: &str,
    no_hanging_indent: bool,
    margin_left: Option<u32>,
    margin_right: Option<u32>,
    alignment: Option<biscuit_terminal::utils::layout::Alignment>,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::list::UnorderedList;
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
    use biscuit_terminal::utils::layout::Margin;
    use std::sync::Arc;

    if items.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No items provided. Usage: bt list \"First item\" \"Second item\" \"Third item\""
        ));
    }

    // Convert each item to a Prose component wrapped in RenderableContent
    let prose_items: Vec<RenderableContent> = items
        .iter()
        .map(|item| {
            // Unescape common escape sequences (shell passes literal \n, \t, etc.)
            let text = item
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r");

            let prose = Prose::new(&text);
            RenderableContent::Component(Arc::new(prose))
        })
        .collect();

    // Build the UnorderedList with custom bullet
    let mut list = UnorderedList::from(prose_items).with_bullet(bullet);

    // Disable hanging indent if requested
    if no_hanging_indent {
        list = list.without_hanging_indent();
    }

    // Configure margins
    if let Some(left) = margin_left {
        list = list.left_margin(Margin::Chars(left));
    }
    if let Some(right) = margin_right {
        list = list.right_margin(Margin::Chars(right));
    }

    // Configure alignment
    if let Some(align) = alignment {
        list = list.alignment(align);
    }

    // Render using fallback_render for terminal-aware output
    let term = Terminal::new();
    let output = list.fallback_render(&term);

    println!("{}", output);

    Ok(())
}
