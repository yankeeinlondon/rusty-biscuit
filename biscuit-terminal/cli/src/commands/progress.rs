use crate::commands::color_parse::parse_color;
use crate::commands::shared::{detect_terminal_honoring_force_color, print_example_command};
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use clap::Args as ClapArgs;
use renderable::tree::{RenderNode, RenderStrictness};

const PROGRESS_EXAMPLE_PERCENT: u8 = 72;
const PROGRESS_EXAMPLE_CMD: &str =
    r#"bt progress 72 --label "Indexing" --width 28 --fill-color green --bracket-color cyan"#;

/// Render a progress bar through the render tree
#[derive(ClapArgs, Debug, Clone)]
pub struct ProgressArgs {
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Completion percentage (0-100).
    #[arg(value_name = "PERCENT", required_unless_present = "example")]
    pub percent: Option<u8>,

    /// Label shown before the bar.
    #[arg(long)]
    pub label: Option<String>,

    /// Width of the bar portion in characters.
    #[arg(long)]
    pub width: Option<u32>,

    /// Color of the filled portion of the track (named or #rrggbb).
    #[arg(long = "fill-color")]
    pub fill_color: Option<String>,

    /// Color of the empty portion of the track (named or #rrggbb).
    #[arg(long = "empty-color")]
    pub empty_color: Option<String>,

    /// Color of the bracket glyphs (named or #rrggbb).
    #[arg(long = "bracket-color")]
    pub bracket_color: Option<String>,
}

impl Run for ProgressArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let percent = self.percent.unwrap_or(PROGRESS_EXAMPLE_PERCENT);
        if percent > 100 {
            return Err(color_eyre::eyre::eyre!(
                "Percentage must be 0-100. Usage: bt progress 60 --label Loading"
            ));
        }

        let mut progress = Progress::new(f32::from(percent) / 100.0);
        let label = self
            .label
            .or_else(|| self.example.then(|| "Indexing".to_string()));
        if let Some(label) = label {
            progress = progress.with_label(label);
        }
        let width = self.width.or_else(|| self.example.then_some(28));
        if let Some(width) = width {
            progress = progress.with_bar_width(width);
        }
        let fill_color = self
            .fill_color
            .or_else(|| self.example.then(|| "green".to_string()));
        if let Some(color) = &fill_color {
            progress = progress.with_filled_color(parse_color(color)?);
        }
        if let Some(color) = &self.empty_color {
            progress = progress.with_empty_color(parse_color(color)?);
        }
        let bracket_color = self
            .bracket_color
            .or_else(|| self.example.then(|| "cyan".to_string()));
        if let Some(color) = &bracket_color {
            progress = progress.with_bracket_color(parse_color(color)?);
        }

        let node = progress.render_tree_node().ok_or_else(|| {
            color_eyre::eyre::eyre!("Progress component produced no render-tree node")
        })?;
        let root = RenderNode::root(vec![node]);

        let term = detect_terminal_honoring_force_color();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let rendered = render_terminal_node(&root, &opts)
            .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;

        println!("{}", rendered.output);
        if self.example {
            print_example_command(PROGRESS_EXAMPLE_CMD);
        }
        Ok(())
    }
}
