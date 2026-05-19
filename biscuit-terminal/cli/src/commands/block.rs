use crate::commands::color_parse::parse_color;
use crate::commands::shared::detect_terminal_honoring_force_color;
use crate::commands::{CliContext, Run};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use clap::Args as ClapArgs;
use renderable::layout::{Length, TargetValue};
use renderable::style::{
    Border, BorderSides, Fill, FillBand, FillIntensity, PerMode, Style, TextEmphasis,
};
use renderable::tree::{RenderNode, RenderStrictness};

/// The fill intensity selected by `--fill`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum FillArg {
    /// A faint tint.
    Subtle,
    /// A strong, clearly visible band.
    Pronounced,
}

/// The band a `--fill` paints across the available width.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum FillBandArg {
    /// Paint the full available width.
    Full,
    /// Paint the content band only.
    Padded,
    /// Paint an indented band, inset from both edges.
    Indented,
}

/// Which sides a `--border` is drawn on.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum BorderArg {
    /// All four sides.
    All,
    /// The left edge only.
    Left,
    /// The right edge only.
    Right,
    /// The top edge only.
    Top,
    /// The bottom edge only.
    Bottom,
}

/// Render a text block through the render tree with a declared `Style`
#[derive(ClapArgs, Debug, Clone)]
pub struct BlockArgs {
    /// Text to render; multiple values are joined with spaces.
    #[arg(value_name = "TEXT", required = true)]
    pub text: Vec<String>,

    /// Foreground color (named or #rrggbb).
    #[arg(long)]
    pub fg: Option<String>,

    /// Background color (named or #rrggbb).
    #[arg(long)]
    pub bg: Option<String>,

    /// Bold text.
    #[arg(long)]
    pub bold: bool,

    /// Italic text.
    #[arg(long)]
    pub italic: bool,

    /// Underlined text.
    #[arg(long)]
    pub underline: bool,

    /// Struck-through text.
    #[arg(long)]
    pub strike: bool,

    /// Paint a background fill band behind the text.
    #[arg(long, value_enum)]
    pub fill: Option<FillArg>,

    /// The band painted by `--fill`.
    #[arg(long = "fill-band", value_enum, default_value = "full")]
    pub fill_band: FillBandArg,

    /// Inset, in columns, applied to the fill band.
    #[arg(long)]
    pub inset: Option<u32>,

    /// Draw a border around the block.
    #[arg(long, value_enum)]
    pub border: Option<BorderArg>,

    /// Border color (named or #rrggbb).
    #[arg(long = "border-color")]
    pub border_color: Option<String>,
}

impl BlockArgs {
    /// Builds the declared [`Style`] from the parsed flags.
    fn build_style(&self) -> color_eyre::Result<Style> {
        let mut style = Style::default();

        if let Some(fg) = &self.fg {
            style.color = Some(TargetValue::universal(PerMode::universal(parse_color(fg)?)));
        }
        if let Some(bg) = &self.bg {
            style.background =
                Some(TargetValue::universal(PerMode::universal(parse_color(bg)?)));
        }

        style.emphasis = TextEmphasis {
            bold: self.bold,
            italic: self.italic,
            strikethrough: self.strike,
            underline: self
                .underline
                .then_some(renderable::style::UnderlineStyle::Straight),
            ..TextEmphasis::default()
        };

        if let Some(fill) = self.fill {
            let intensity = match fill {
                FillArg::Subtle => FillIntensity::Subtle,
                FillArg::Pronounced => FillIntensity::Pronounced,
            };
            let band = match self.fill_band {
                FillBandArg::Full => FillBand::Full,
                FillBandArg::Padded => FillBand::Padded,
                FillBandArg::Indented => FillBand::Indented,
            };
            style.fill = Some(Fill {
                color: None,
                intensity,
                band,
                inset: self
                    .inset
                    .map(|n| TargetValue::universal(Length::ch(n))),
            });
        }

        if let Some(border) = self.border {
            let sides = match border {
                BorderArg::All => BorderSides::All,
                BorderArg::Left => BorderSides::Sides {
                    top: false,
                    right: false,
                    bottom: false,
                    left: true,
                },
                BorderArg::Right => BorderSides::Sides {
                    top: false,
                    right: true,
                    bottom: false,
                    left: false,
                },
                BorderArg::Top => BorderSides::Sides {
                    top: true,
                    right: false,
                    bottom: false,
                    left: false,
                },
                BorderArg::Bottom => BorderSides::Sides {
                    top: false,
                    right: false,
                    bottom: true,
                    left: false,
                },
            };
            let color = match &self.border_color {
                Some(c) => Some(TargetValue::universal(PerMode::universal(parse_color(c)?))),
                None => None,
            };
            style.border = Some(Border {
                color,
                sides,
                ..Border::default()
            });
        }

        Ok(style)
    }
}

impl Run for BlockArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let text = self.text.join(" ");
        if text.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No text provided. Usage: bt block \"line one\" --fg red --border all"
            ));
        }
        let text = crate::types::unescape_shell_escapes(&text);

        let style = self.build_style()?;

        let mut node = RenderNode::paragraph(vec![RenderNode::text(text)]);
        node.attrs.set_style(&style);
        let root = RenderNode::root(vec![node]);

        let term = detect_terminal_honoring_force_color();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let rendered = render_terminal_node(&root, &opts)
            .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;

        println!("{}", rendered.output);
        Ok(())
    }
}
