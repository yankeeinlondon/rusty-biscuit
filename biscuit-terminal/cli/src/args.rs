use crate::*;
use biscuit_terminal::utils::layout;
use clap::{Args as ClapArgs, Parser, Subcommand};
use clap_complete::engine::ArgValueCompleter;

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
pub struct Args {
    /// Output in JSON format
    #[arg(long, global = true, display_order = 100)]
    pub json: bool,

    /// Verbose output (show more details)
    #[arg(short, long, global = true, display_order = 101)]
    pub verbose: bool,

    /// Generate shell completions and exit.
    ///
    /// Outputs completion scripts for the specified shell to stdout.
    /// Redirect the output to the appropriate file for your shell.
    /// Use --completions help for setup instructions.
    #[arg(long, value_name = "SHELL", global = true, display_order = 102)]
    pub completions: Option<String>,

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

        #[command(flatten)]
        layout: LayoutArgs,

        /// Output rendering metadata to stderr (filename, file size, render time)
        #[arg(long)]
        meta: bool,

        /// Print cursor position diagnostics (before/after image render)
        #[arg(long)]
        debug: bool,
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

        #[command(flatten)]
        layout: LayoutArgs,

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
    #[command(
        display_order = 3,
        after_long_help = "\
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
"
    )]
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
        #[arg(
            long = "top-left",
            short = 'l',
            visible_alias = "tl",
            allow_hyphen_values = true
        )]
        top_left: Option<String>,

        /// Top-right quadrant label (quadrant-2)
        #[arg(
            long = "top-right",
            short = 'r',
            visible_alias = "tr",
            allow_hyphen_values = true
        )]
        top_right: Option<String>,

        /// Bottom-left quadrant label (quadrant-3)
        #[arg(long = "bottom-left", visible_alias = "bl", allow_hyphen_values = true)]
        bottom_left: Option<String>,

        /// Bottom-right quadrant label (quadrant-4)
        #[arg(
            long = "bottom-right",
            visible_alias = "br",
            allow_hyphen_values = true
        )]
        bottom_right: Option<String>,

        /// Use inverted colors with solid background
        #[arg(long)]
        inverse: bool,

        /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
        ///
        /// Default is 50% of terminal width. Aspect ratio is always preserved.
        #[arg(long, short = 'w')]
        width: Option<String>,

        #[command(flatten)]
        layout: LayoutArgs,

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
    #[command(
        name = "pie-chart",
        display_order = 4,
        after_long_help = "\
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
"
    )]
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

        #[command(flatten)]
        layout: LayoutArgs,

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
    #[command(
        name = "git-graph",
        display_order = 5,
        after_long_help = "\
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
"
    )]
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

        #[command(flatten)]
        layout: LayoutArgs,

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
    #[command(
        name = "bar-chart",
        display_order = 7,
        after_long_help = "\
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
"
    )]
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

        #[command(flatten)]
        layout: LayoutArgs,

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
    #[command(
        name = "line-chart",
        display_order = 8,
        after_long_help = "\
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
"
    )]
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

        #[command(flatten)]
        layout: LayoutArgs,

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
    #[command(
        name = "timeline",
        display_order = 9,
        after_long_help = "\
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
"
    )]
    Timeline {
        /// Timeline title
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., \"50%\"), characters (e.g., \"80ch\"), or \"fill\"
        #[arg(long, short = 'w')]
        width: Option<String>,

        #[command(flatten)]
        layout: LayoutArgs,

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
    #[command(
        name = "state-diagram",
        display_order = 10,
        after_long_help = "\
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
"
    )]
    StateDiagram {
        /// Diagram title
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., \"50%\"), characters (e.g., \"80ch\"), or \"fill\"
        #[arg(long, short = 'w')]
        width: Option<String>,

        #[command(flatten)]
        layout: LayoutArgs,

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

    /// Render a graph expression as a diagram
    ///
    /// Creates a graph diagram from expression syntax or DOT format.
    /// Expression syntax: "a -> b -> c" or "a -- b -- c"
    #[command(
        display_order = 11,
        after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Render an example graph:
    bt graph-expression --example

  Basic directed graph (expression syntax):
    bt graph-expression \"a -> b -> c\"

  Undirected graph (expression syntax):
    bt graph-expression \"a -- b -- c\"

  With title:
    bt graph-expression --title \"My Graph\" \"start -> validate -> render\"

  Left-to-right orientation:
    bt graph-expression --orientation left-to-right \"a -> b -> c\"

  DOT format (explicit):
    bt graph-expression --syntax dot \"digraph { A -> B; B -> C; }\"

  Custom width:
    bt graph-expression --width 60% \"a -> b -> c\"

  Inverted colors (solid background):
    bt graph-expression --inverse \"a -> b -> c\"

  JSON output (for scripting):
    bt graph-expression --json \"a -> b\"

\x1b[1m\x1b[4mSyntax Formats:\x1b[0m
  Expression (default):
    - Directed edge:   a -> b
    - Undirected edge: a -- b
    - Chain:           a -> b -> c (creates a->b and b->c)
    - Separators:      semicolons or newlines

  DOT format:
    Full Graphviz DOT language support
    Auto-detected if input contains 'digraph' or 'graph'
"
    )]
    GraphExpression {
        /// Show an example graph expression
        #[arg(long, short = 'e')]
        example: bool,

        /// Input syntax (auto, expression, dot)
        #[arg(long, default_value = "auto", value_enum)]
        syntax: GraphInputSyntaxArg,

        /// Diagram title
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
        #[arg(long, short = 'w')]
        width: Option<String>,

        /// Use inverted colors with solid background
        #[arg(long)]
        inverse: bool,

        /// Graph layout direction
        #[arg(long, value_enum, default_value = "top-to-bottom")]
        orientation: GraphOrientationArg,

        #[command(flatten)]
        layout: LayoutArgs,

        /// Show render metadata
        #[arg(long)]
        meta: bool,

        /// Graph expression or DOT input
        #[arg(value_name = "EXP", required_unless_present = "example")]
        content: Vec<String>,
    },

    /// Render an entity relationship diagram (ERD)
    ///
    /// Creates a Mermaid ERD showing entities and their relationships.
    #[command(
        name = "erd",
        display_order = 12,
        after_long_help = "\
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
"
    )]
    Erd {
        /// Diagram title
        #[arg(long, short = 't')]
        title: Option<String>,

        /// Display width: percentage (e.g., \"50%\"), characters (e.g., \"80ch\"), or \"fill\"
        #[arg(long, short = 'w')]
        width: Option<String>,

        #[command(flatten)]
        layout: LayoutArgs,

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
    #[command(
        display_order = 20,
        after_long_help = "\
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
    bt prose --ml 4 --mr 4 \"Indented with margins\"

  With alignment:
    bt prose --alignment center \"Centered text\"
    bt prose --align right \"Right-aligned text\"

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
"
    )]
    Prose {
        /// Content with {{tokens}} and <block>tags</block>
        #[arg(value_name = "CONTENT")]
        content: Vec<String>,

        /// Disable word wrapping
        #[arg(long)]
        no_wrap: bool,

        #[command(flatten)]
        layout: LayoutArgs,
    },

    /// Render styled text in a block quote
    ///
    /// Wraps prose content in a block quote with a left border.
    /// Supports the same {{tokens}} and <block>tags</block> as the prose command.
    #[command(
        display_order = 12,
        after_long_help = "\
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
    bt quote --attribution \"Albert Einstein\" \"<i>Imagination is more important than knowledge.</i>\"
"
    )]
    Quote {
        /// Content with {{tokens}} and <block>tags</block>
        #[arg(value_name = "CONTENT")]
        content: Vec<String>,

        /// Attribution (author/source) displayed below the quote
        #[arg(long)]
        attribution: Option<String>,

        #[command(flatten)]
        layout: LayoutArgs,
    },

    /// Render a bulleted list with hanging indents
    ///
    /// Each argument becomes a list item. Supports the same {{tokens}} and
    /// <block>tags</block> as the prose command for styling individual items.
    #[command(
        display_order = 13,
        after_long_help = "\
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
"
    )]
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

        #[command(flatten)]
        layout: LayoutArgs,
    },

    /// Render two columns of text side by side
    #[command(
        display_order = 14,
        after_long_help = r#"
[1m[4mExamples:[0m
  Basic columns:
    bt columns "Left column" "Right column"

  With custom gap:
    bt columns --gap 6 "Left" "Right"

  Fixed left width:
    bt columns --left 24 "Title" "Longer description on the right"

  Percentage left width:
    bt columns --left 40% "Short" "Longer content that wraps"

  With margins and alignment:
    bt columns --margin-left 2 --margin-right 2 --alignment center "Left" "Right"
"#
    )]
    Columns {
        /// Left column content
        #[arg(value_name = "LEFT", id = "left_content")]
        left: String,

        /// Right column content
        #[arg(value_name = "RIGHT")]
        right: String,

        /// Gap between columns in characters
        #[arg(long, default_value_t = 3)]
        gap: u32,

        /// Left column width (e.g., "20", "20ch", "40%")
        #[arg(long = "left", value_name = "WIDTH")]
        left_width: Option<String>,

        #[command(flatten)]
        layout: LayoutArgs,
    },

    /// Display a directory tree with icons and gitignore awareness
    ///
    /// Renders a filesystem tree with Nerd Font icons, gitignore-aware
    /// dimming, and optional depth/filter controls.
    #[command(
        display_order = 15,
        after_long_help = "\
\x1b[1m\x1b[4mExamples:\x1b[0m
  Current directory:
    bt dir

  Specific path:
    bt dir /path/to/project

  Limit depth:
    bt dir --depth 2

  Filter by extension:
    bt dir --filter \".rs\"
    bt dir -f \".rs\" -f \".toml\"

  Combined:
    bt dir src --depth 3 --filter \".rs\"

  With file metrics:
    bt dir --size           # Show file sizes
    bt dir --token         # Show estimated token counts
    bt dir --modified      # Show modification timestamps
    bt dir --updated       # Show relative modification times (\"2 days ago\")
    bt dir --size --token --modified
"
    )]
    Dir {
        /// Directory path to display (defaults to current directory)
        #[arg(value_name = "PATH", default_value = ".")]
        path: String,

        /// Maximum depth to recurse
        #[arg(long, short = 'd', value_name = "N")]
        depth: Option<u32>,

        /// Filter pattern (can be repeated, e.g., \".rs\", \".toml\")
        #[arg(long, short = 'f', value_name = "PATTERN")]
        filter: Vec<String>,

        /// Hide the root directory header line
        #[arg(long)]
        skip_root: bool,

        /// Show file sizes (human-readable, e.g., \"1.2 KB\")
        #[arg(long)]
        size: bool,

        /// Show estimated LLM token counts
        #[arg(long)]
        tokens: bool,

        /// Show absolute modification timestamps
        #[arg(long)]
        modified: bool,

        /// Show relative modification times (e.g., \"2 days ago\")
        #[arg(long)]
        updated: bool,

        #[command(flatten)]
        layout: LayoutArgs,
    },
}

/// Graph input syntax argument for CLI.
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum GraphInputSyntaxArg {
    Auto,
    Expression,
    Dot,
}

impl GraphInputSyntaxArg {
    pub fn as_cli_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Expression => "expression",
            Self::Dot => "dot",
        }
    }
}

impl From<GraphInputSyntaxArg>
    for biscuit_terminal::components::graph_expression::GraphInputSyntax
{
    fn from(arg: GraphInputSyntaxArg) -> Self {
        match arg {
            GraphInputSyntaxArg::Auto => Self::Auto,
            GraphInputSyntaxArg::Expression => Self::Expression,
            GraphInputSyntaxArg::Dot => Self::Dot,
        }
    }
}

/// Graph orientation argument for CLI.
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum GraphOrientationArg {
    LeftToRight,
    TopToBottom,
}

impl GraphOrientationArg {
    pub fn as_cli_str(&self) -> &'static str {
        match self {
            Self::LeftToRight => "left-to-right",
            Self::TopToBottom => "top-to-bottom",
        }
    }
}

impl From<GraphOrientationArg>
    for biscuit_terminal::components::graph_expression::GraphOrientation
{
    fn from(arg: GraphOrientationArg) -> Self {
        match arg {
            GraphOrientationArg::LeftToRight => Self::LeftToRight,
            GraphOrientationArg::TopToBottom => Self::TopToBottom,
        }
    }
}
