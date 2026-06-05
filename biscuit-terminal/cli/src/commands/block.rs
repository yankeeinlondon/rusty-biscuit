use crate::commands::color_parse::parse_color;
use crate::commands::shared::{detect_terminal_honoring_force_color, print_example_command};
use crate::commands::{CliContext, Run};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use clap::Args as ClapArgs;
use renderable::layout::{Length, TargetValue};
use renderable::style::{Background, Border, BorderSides, PerMode, Style, TextEmphasis};
use renderable::tree::{RenderNode, RenderStrictness};

const BLOCK_EXAMPLE: &str = "Release candidate passed";
const BLOCK_EXAMPLE_CMD: &str =
    r#"bt block "Release candidate passed" --fg green --bold --border left --fill subtle"#;

/// The adaptive background tint selected by `--fill`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum FillArg {
    /// A faint tint.
    Subtle,
    /// A strong, clearly visible tint.
    Pronounced,
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
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Text to render; multiple values are joined with spaces.
    #[arg(value_name = "TEXT", required_unless_present = "example")]
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

    /// Paint an adaptive background tint behind the text.
    #[arg(long, value_enum)]
    pub fill: Option<FillArg>,

    /// Draw a border around the block.
    #[arg(long, value_enum)]
    pub border: Option<BorderArg>,

    /// Border color (named or #rrggbb).
    #[arg(long = "border-color")]
    pub border_color: Option<String>,

    /// Corner radius, in columns; any non-zero value rounds the corners.
    #[arg(long = "border-radius")]
    pub border_radius: Option<u32>,
}

impl BlockArgs {
    /// Builds the declared [`Style`] from the parsed flags.
    fn build_style(&self) -> color_eyre::Result<Style> {
        let mut style = Style::default();

        if let Some(fg) = &self.fg {
            style.color = Some(TargetValue::universal(PerMode::universal(parse_color(fg)?)));
        }
        if let Some(bg) = &self.bg {
            style.background = Some(TargetValue::universal(PerMode::universal(parse_color(bg)?)));
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
            style.background = Some(match fill {
                FillArg::Subtle => Background::subtle(),
                FillArg::Pronounced => Background::pronounced(),
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
                radius: self
                    .border_radius
                    .map(|n| TargetValue::universal(Length::ch(n))),
                ..Border::default()
            });
        }

        Ok(style)
    }
}

impl Run for BlockArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let text = if self.example {
            BLOCK_EXAMPLE.to_string()
        } else {
            self.text.join(" ")
        };
        if text.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No text provided. Usage: bt block \"line one\" --fg red --border all"
            ));
        }
        let text = crate::types::unescape_shell_escapes(&text);

        let mut style_args = self.clone();
        if self.example {
            style_args.fg.get_or_insert_with(|| "green".to_string());
            style_args.bold = true;
            style_args.border.get_or_insert(BorderArg::Left);
            style_args.fill.get_or_insert(FillArg::Subtle);
        }
        let style = style_args.build_style()?;

        let mut node = RenderNode::paragraph(vec![RenderNode::text(text)]);
        node.attrs.set_style(&style);
        let root = RenderNode::root(vec![node]);

        let term = detect_terminal_honoring_force_color();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let rendered = render_terminal_node(&root, &opts)
            .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;

        println!("{}", rendered.output);
        if self.example {
            print_example_command(BLOCK_EXAMPLE_CMD);
        }
        Ok(())
    }
}
