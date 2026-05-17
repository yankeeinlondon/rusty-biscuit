use crate::args::LayoutArgs;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::filesystem::FileSystem;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Length, TargetValue};
use clap::Args as ClapArgs;

/// Options for rendering a directory tree.
#[derive(Debug, Clone, Default)]
pub struct DirOptions {
    pub show_size: bool,
    pub show_token: bool,
    pub show_modified: bool,
    pub show_updated: bool,
}

/// Display a directory tree with icons and gitignore awareness
#[derive(ClapArgs, Debug, Clone)]
pub struct DirArgs {
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: String,

    #[arg(long, short = 'd', value_name = "N")]
    pub depth: Option<u32>,

    #[arg(long, short = 'f', value_name = "PATTERN")]
    pub filter: Vec<String>,

    #[arg(long)]
    pub skip_root: bool,

    #[arg(long)]
    pub size: bool,

    #[arg(long)]
    pub tokens: bool,

    #[arg(long)]
    pub modified: bool,

    #[arg(long)]
    pub updated: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for DirArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let options = DirOptions {
            show_size: self.size,
            show_token: self.tokens,
            show_modified: self.modified,
            show_updated: self.updated,
        };
        render_dir(
            &self.path,
            self.depth,
            &self.filter,
            self.skip_root,
            &self.layout,
            &options,
        )
    }
}

pub fn render_dir(
    path: &str,
    depth: Option<u32>,
    filter: &[String],
    skip_root: bool,
    layout: &LayoutArgs,
    options: &DirOptions,
) -> color_eyre::Result<()> {
    let mut fs = FileSystem::new_with_formatting(path)?;

    if let Some(d) = depth {
        fs = fs.depth(d);
    }

    for pat in filter {
        fs = fs.filter(pat);
    }

    if skip_root {
        fs = fs.show_root(false);
    }

    if options.show_size {
        fs = fs.show_file_size();
    }
    if options.show_token {
        fs = fs.show_tokens();
    }
    if options.show_modified {
        fs = fs.show_modified();
    }
    if options.show_updated {
        fs = fs.show_modified_since();
    }

    if let Some(left) = layout.margin_left {
        fs = fs.left_margin(TargetValue::universal(Length::ch(left)));
    }
    if let Some(right) = layout.margin_right {
        fs = fs.right_margin(TargetValue::universal(Length::ch(right)));
    }
    if let Some(align) = layout.alignment {
        fs = fs.alignment(align);
    }

    fs.ensure_tree_built();

    let term = Terminal::new();
    let output = fs.render(&term);

    let top = layout.margin_top.unwrap_or(1);
    for _ in 0..top {
        eprintln!();
    }

    let output = output.trim_end();
    println!("{output}");

    let bottom = layout.margin_bottom.unwrap_or(0);
    for _ in 0..bottom {
        eprintln!();
    }

    Ok(())
}
