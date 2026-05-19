use crate::commands::color_parse::parse_color;
use crate::commands::shared::detect_terminal_honoring_force_color;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use clap::Args as ClapArgs;
use renderable::tree::{RenderNode, RenderStrictness};

/// Render a progress bar through the render tree
#[derive(ClapArgs, Debug, Clone)]
pub struct ProgressArgs {
    /// Completion percentage (0-100).
    #[arg(value_name = "PERCENT")]
    pub percent: u8,

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
        if self.percent > 100 {
            return Err(color_eyre::eyre::eyre!(
                "Percentage must be 0-100. Usage: bt progress 60 --label Loading"
            ));
        }

        let mut progress = Progress::new(f32::from(self.percent) / 100.0);
        if let Some(label) = self.label {
            progress = progress.with_label(label);
        }
        if let Some(width) = self.width {
            progress = progress.with_bar_width(width);
        }
        if let Some(color) = &self.fill_color {
            progress = progress.with_filled_color(parse_color(color)?);
        }
        if let Some(color) = &self.empty_color {
            progress = progress.with_empty_color(parse_color(color)?);
        }
        if let Some(color) = &self.bracket_color {
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
        Ok(())
    }
}
