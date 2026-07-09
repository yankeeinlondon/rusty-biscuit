use crate::args::LayoutArgs;
use crate::commands::shared::terminal_for_render;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::filesystem::FileSystem;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::utils::layout::{Length, TargetValue};
use clap::Args as ClapArgs;
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;

const DIR_EXAMPLE_PATH: &str = ".";
const DIR_EXAMPLE_CMD: &str = r#"bt dir . --depth 1 --filter ".rs""#;

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

    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

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

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus instead of the terminal.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for DirArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let path = if self.example {
            DIR_EXAMPLE_PATH
        } else {
            &self.path
        };
        let depth = if self.example && self.depth.is_none() {
            Some(1)
        } else {
            self.depth
        };
        let filter = if self.example && self.filter.is_empty() {
            vec![".rs".to_string()]
        } else {
            self.filter
        };
        let options = DirOptions {
            show_size: self.size,
            show_token: self.tokens,
            show_modified: self.modified,
            show_updated: self.updated,
        };

        // Cross-target output: HTML fragment, portable Markdown, or
        // MarkdownPlus. These targets route through the canonical render
        // tree; the terminal remains the bespoke FileSystem renderer until
        // parity tests for the tree path land. LayoutArgs are applied to
        // the component before projection so non-default `Layout` slots
        // (margins, alignment) ride along on the projected root.
        if self.html || self.md || self.md_plus {
            render_dir_alt_target(
                path,
                depth,
                &filter,
                self.skip_root,
                &self.layout,
                &options,
                AltTarget::from_flags(self.html, self.md, self.md_plus),
            )?;
            if self.example {
                crate::commands::shared::print_example_command(DIR_EXAMPLE_CMD);
            }
            return Ok(());
        }

        render_dir(
            path,
            depth,
            &filter,
            self.skip_root,
            &self.layout,
            &options,
            ctx.plain,
        )?;

        if self.example {
            crate::commands::shared::print_example_command(DIR_EXAMPLE_CMD);
        }

        Ok(())
    }
}

/// Cross-target output selection for `bt dir`.
#[derive(Debug, Clone, Copy)]
enum AltTarget {
    Html,
    Markdown,
    MarkdownPlus,
}

impl AltTarget {
    fn from_flags(html: bool, md: bool, md_plus: bool) -> Self {
        if html {
            AltTarget::Html
        } else if md_plus {
            AltTarget::MarkdownPlus
        } else if md {
            AltTarget::Markdown
        } else {
            // The caller in `Run::run` only takes this branch when at least
            // one of html / md / md_plus is set, so the fall-through is
            // statically unreachable. clap's `conflicts_with_all` guards
            // also prevent multiple flags being set simultaneously.
            unreachable!("at least one alt-target flag must be set")
        }
    }
}

fn render_dir_alt_target(
    path: &str,
    depth: Option<u32>,
    filter: &[String],
    skip_root: bool,
    layout: &LayoutArgs,
    options: &DirOptions,
    target: AltTarget,
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

    // Mirror the terminal path: LayoutArgs lifts onto the component so the
    // projection emits the non-default Layout on the projected root.
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

    let output = match target {
        AltTarget::Html => fs.render_html_fragment().render(),
        AltTarget::Markdown => fs.render_markdown(),
        AltTarget::MarkdownPlus => fs.render_markdown_plus(),
    };
    println!("{output}");
    Ok(())
}

pub fn render_dir(
    path: &str,
    depth: Option<u32>,
    filter: &[String],
    skip_root: bool,
    layout: &LayoutArgs,
    options: &DirOptions,
    plain: bool,
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

    let term = terminal_for_render(plain);
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
