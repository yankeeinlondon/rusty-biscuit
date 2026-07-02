use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::list::{OrderedList, UnorderedList};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Length, TargetValue};
use clap::Args as ClapArgs;
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;

const COMPOSE_EXAMPLE_CMD: &str = r#"bt compose --heading 1 "Project Status" --text "Build: " --prose "<green>passing</green>" --list "Unit tests" "Integration tests""#;

/// Compose multiple renderable parts into one ordered output.
///
/// `bt compose` is the CLI entry point for the `Compose` component — a
/// sequence container that concatenates each part with no automatic
/// separator. By default the output goes to the terminal through the
/// canonical `renderable` tree renderer; pass `--md` / `--md-plus` / `--html`
/// to switch to those targets. The three output-target flags are mutually
/// exclusive.
///
/// ## Part ordering
///
/// Parts are appended in this fixed order: headings → text → prose → lists
/// → ordered lists → tables → positional `ITEMS` (each as a plain text
/// part). Within each kind, parts are appended in command-line declaration
/// order. This keeps the implementation predictable; a typical command uses
/// mostly one or two part kinds and the spec example output is exactly
/// reproduced under this policy.
#[derive(ClapArgs, Debug, Clone)]
pub struct ComposeArgs {
    /// Render an example and show the command used.
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Positional plain text items appended in order.
    #[arg(value_name = "ITEMS")]
    pub items: Vec<String>,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with_all = ["md_plus", "html"])]
    pub md: bool,

    /// Render to MarkdownPlus instead of the terminal.
    #[arg(long = "md-plus", conflicts_with_all = ["md", "html"])]
    pub md_plus: bool,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Add a plain text part. Repeatable.
    #[arg(long = "text")]
    pub text: Vec<String>,

    /// Add a Prose-styled part. Repeatable.
    #[arg(long = "prose")]
    pub prose: Vec<String>,

    /// Add a heading part as `LEVEL TITLE`; LEVEL must be 1–6. Repeatable.
    #[arg(long = "heading", num_args = 2, value_names = ["LEVEL", "TITLE"])]
    pub heading: Vec<String>,

    /// Add an unordered list part. Repeatable; each occurrence accepts one
    /// or more items. Use `--` to terminate values when the next argument
    /// would otherwise be misread as a list item.
    #[arg(long = "list", num_args = 1.., value_terminator = "--")]
    pub list: Vec<String>,

    /// Add an ordered list part. Repeatable; each occurrence accepts one or
    /// more items. Use `--` to terminate values when the next argument
    /// would otherwise be misread as a list item.
    #[arg(long = "ordered-list", num_args = 1.., value_terminator = "--")]
    pub ordered_list: Vec<String>,

    /// Add a table part. Repeatable; each occurrence takes a header spec
    /// followed by one or more row specs, e.g.
    /// `--table "Name,Age" "Alice,30" "Bob,25"`. Use `--` to terminate.
    #[arg(long = "table", num_args = 1.., value_terminator = "--")]
    pub table: Vec<String>,

    #[command(flatten)]
    pub layout: LayoutArgs,
}

impl Run for ComposeArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let example = self.example;
        let md = self.md;
        let md_plus = self.md_plus;
        let html = self.html;
        let layout = self.layout.clone();

        let compose = if example {
            build_example_compose()
        } else {
            self.build_compose()?
        };

        let compose = apply_layout_args(compose, &layout);

        if md {
            println!("{}", compose.render_markdown());
            if example {
                print_example_command(COMPOSE_EXAMPLE_CMD);
            }
            return Ok(());
        }
        if md_plus {
            println!("{}", compose.render_markdown_plus());
            if example {
                print_example_command(COMPOSE_EXAMPLE_CMD);
            }
            return Ok(());
        }
        if html {
            let fragment = BrowserRenderable::render_html_fragment(&compose).render();
            println!("{fragment}");
            if example {
                print_example_command(COMPOSE_EXAMPLE_CMD);
            }
            return Ok(());
        }

        let term = terminal_for_render(ctx.plain);
        let output = compose.render(&term);
        let output = if std::env::var("NO_COLOR").is_ok() {
            strip_sgr_sequences(&output)
        } else {
            output
        };

        emit_vertical_margins(&layout, || {
            println!("{}", output);
            Ok(())
        })?;

        if example {
            print_example_command(COMPOSE_EXAMPLE_CMD);
        }

        Ok(())
    }
}

impl ComposeArgs {
    /// Builds a [`Compose`] from the parsed arguments.
    ///
    /// Returns an error if `--heading` arguments do not parse as `LEVEL TITLE`
    /// pairs with a valid level (1–6), or if no parts are supplied.
    fn build_compose(self) -> color_eyre::Result<Compose> {
        let mut compose = Compose::default();

        // Headings: each pair is `LEVEL TITLE`.
        if !self.heading.len().is_multiple_of(2) {
            return Err(color_eyre::eyre::eyre!(
                "--heading requires LEVEL and TITLE; got an odd number of values"
            ));
        }
        for pair in self.heading.chunks(2) {
            let level: u8 = pair[0].parse().map_err(|e| {
                color_eyre::eyre::eyre!("invalid --heading LEVEL '{}': {e}", pair[0])
            })?;
            if !(1..=6).contains(&level) {
                return Err(color_eyre::eyre::eyre!(
                    "--heading LEVEL must be 1..=6; got {level}"
                ));
            }
            let title = crate::types::unescape_shell_escapes(&pair[1]);
            compose.add_heading(title, level);
        }

        for text in self.text {
            let s = crate::types::unescape_shell_escapes(&text);
            compose.add_text(s);
        }

        for prose_text in self.prose {
            let s = crate::types::unescape_shell_escapes(&prose_text);
            compose.add_prose(Prose::new(&s));
        }

        if !self.list.is_empty() {
            let items: Vec<String> = self
                .list
                .iter()
                .map(|s| crate::types::unescape_shell_escapes(s))
                .collect();
            compose.add_unordered_list(UnorderedList::new(items));
        }

        if !self.ordered_list.is_empty() {
            let items: Vec<String> = self
                .ordered_list
                .iter()
                .map(|s| crate::types::unescape_shell_escapes(s))
                .collect();
            compose.add_ordered_list(OrderedList::new(items));
        }

        if !self.table.is_empty() {
            compose.add_table(parse_table_spec(&self.table)?);
        }

        for item in self.items {
            let s = crate::types::unescape_shell_escapes(&item);
            compose.add_text(s);
        }

        if compose.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No parts provided. Pass positional ITEMS or one of --text/--prose/--heading/--list/--ordered-list/--table, or try --example.",
            ));
        }

        Ok(compose)
    }
}

/// Parses a `--table` value list as `"H1,H2,..." "C1,C2,..."` rows.
fn parse_table_spec(spec: &[String]) -> color_eyre::Result<Table> {
    if spec.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "--table requires a header spec followed by zero or more rows"
        ));
    }
    let header_spec = &spec[0];
    let headers: Vec<&str> = header_spec
        .split(',')
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .collect();
    if headers.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "--table header spec '{header_spec}' has no columns"
        ));
    }

    let columns: Vec<TableColumn> = headers.iter().map(|h| TableColumn::new(*h)).collect();
    let data: Vec<Vec<TableCellContent>> = spec[1..]
        .iter()
        .map(|row| {
            row.split(',')
                .map(|cell| TableCellContent::from(cell.trim().to_string()))
                .collect()
        })
        .collect();
    Ok(Table::new().with_columns(columns).with_data(data))
}

/// Builds the canonical example used by `--example`.
fn build_example_compose() -> Compose {
    let mut compose = Compose::default();
    compose
        .add_heading("Project Status", 1)
        .add_text("Build: ")
        .add_prose(Prose::new("<green>passing</green>"))
        .add_unordered_list(UnorderedList::new(vec!["Unit tests", "Integration tests"]));
    compose
}

/// Applies the shared `LayoutArgs` to a [`Compose`].
fn apply_layout_args(mut compose: Compose, layout: &LayoutArgs) -> Compose {
    if let Some(left) = layout.margin_left {
        compose = compose.left_margin(TargetValue::universal(Length::ch(left)));
    }
    if let Some(right) = layout.margin_right {
        compose = compose.right_margin(TargetValue::universal(Length::ch(right)));
    }
    if let Some(align) = layout.alignment {
        compose = compose.alignment(align);
    }
    compose
}

// Re-export for unit tests so the integration of clap derive + manual parsing
// can be exercised without a full process spawn.
#[cfg(test)]
#[allow(dead_code)]
fn parse_from<I, T>(iter: I) -> color_eyre::Result<ComposeArgs>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    use clap::Parser;
    #[derive(Parser)]
    #[command(no_binary_name = true)]
    struct Wrapper {
        #[command(flatten)]
        inner: ComposeArgs,
    }
    Wrapper::try_parse_from(iter)
        .map(|w| w.inner)
        .map_err(|e| color_eyre::eyre::eyre!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut in_esc = false;
        for ch in s.chars() {
            if in_esc {
                if ch.is_ascii_alphabetic() {
                    in_esc = false;
                }
            } else if ch == '\x1b' {
                in_esc = true;
            } else {
                out.push(ch);
            }
        }
        out
    }

    #[test]
    fn parse_with_no_args_only_example_flag_accepts() {
        // `--example` alone is a valid invocation; build_compose isn't
        // exercised because `Run::run` short-circuits on `--example`.
        let args = parse_from(["--example"]).expect("parse");
        assert!(args.example);
    }

    #[test]
    fn parse_positional_items_become_text_parts() {
        let args = parse_from(["foo", "bar"]).expect("parse");
        let compose = args.build_compose().expect("build");
        assert_eq!(compose.render_markdown(), "foobar");
    }

    #[test]
    fn parse_text_flag_repeats() {
        let args = parse_from(["--text", "a", "--text", "b"]).expect("parse");
        let compose = args.build_compose().expect("build");
        assert_eq!(compose.render_markdown(), "ab");
    }

    #[test]
    fn parse_prose_renders_with_styling() {
        let args = parse_from(["--prose", "<b>bold</b>"]).expect("parse");
        let compose = args.build_compose().expect("build");
        let terminal_output = compose.render_optimistic(Some(80));
        assert!(
            terminal_output.contains("\x1b[1m"),
            "expected bold SGR; got: {terminal_output:?}"
        );
        assert!(strip_ansi(&terminal_output).contains("bold"));
    }

    #[test]
    fn parse_heading_pair() {
        let args = parse_from(["--heading", "2", "Title"]).expect("parse");
        let compose = args.build_compose().expect("build");
        assert_eq!(compose.render_markdown(), "## Title");
    }

    #[test]
    fn parse_heading_rejects_invalid_level() {
        let args = parse_from(["--heading", "9", "Bad"]).expect("parse");
        assert!(args.build_compose().is_err());
    }

    #[test]
    fn parse_heading_rejects_non_numeric_level() {
        let args = parse_from(["--heading", "foo", "Bad"]).expect("parse");
        assert!(args.build_compose().is_err());
    }

    #[test]
    fn parse_list_creates_unordered_list() {
        let args = parse_from(["--list", "one", "two"]).expect("parse");
        let compose = args.build_compose().expect("build");
        let md = compose.render_markdown();
        assert!(md.contains("- one"));
        assert!(md.contains("- two"));
    }

    #[test]
    fn parse_ordered_list_creates_ordered_list() {
        let args = parse_from(["--ordered-list", "first", "second"]).expect("parse");
        let compose = args.build_compose().expect("build");
        let md = compose.render_markdown();
        assert!(md.contains("1. first"));
        assert!(md.contains("2. second"));
    }

    #[test]
    fn parse_table_creates_table_with_rows() {
        let args = parse_from(["--table", "Name,Age", "Alice,30", "Bob,25"]).expect("parse");
        let compose = args.build_compose().expect("build");
        let md = compose.render_markdown();
        assert!(md.contains("Name"));
        assert!(md.contains("Alice"));
        assert!(md.contains("Bob"));
    }

    #[test]
    fn parse_table_rejects_empty_header() {
        let args = parse_from(["--table", "", "row1"]).expect("parse");
        assert!(args.build_compose().is_err());
    }

    #[test]
    fn no_parts_returns_error() {
        let args = parse_from([] as [&str; 0]).expect("parse");
        assert!(args.build_compose().is_err());
    }

    #[test]
    fn md_and_html_are_mutually_exclusive() {
        // Clap should reject combinations of mutually-exclusive output
        // targets at parse time.
        assert!(parse_from(["--md", "--html", "x"]).is_err());
        assert!(parse_from(["--md", "--md-plus", "x"]).is_err());
        assert!(parse_from(["--md-plus", "--html", "x"]).is_err());
    }

    #[test]
    fn example_compose_renders_terminal_with_heading_and_list() {
        let compose = build_example_compose();
        let output = compose.render_optimistic(Some(80));
        let stripped = strip_ansi(&output);
        assert!(stripped.contains("# Project Status"));
        assert!(stripped.contains("Build: "));
        assert!(stripped.contains("passing"));
        assert!(stripped.contains("Unit tests"));
        assert!(stripped.contains("Integration tests"));
    }

    #[test]
    fn example_compose_renders_markdown() {
        let md = build_example_compose().render_markdown();
        assert!(md.contains("# Project Status"));
        assert!(md.contains("Build: "));
        // Prose `<green>` lowers to a styled Span which Markdown ignores —
        // only the literal text survives.
        assert!(md.contains("passing"));
        assert!(md.contains("- Unit tests"));
        assert!(md.contains("- Integration tests"));
    }

    #[test]
    fn example_compose_renders_html() {
        let html = BrowserRenderable::render_html_fragment(&build_example_compose()).render();
        assert!(html.contains("<h1"));
        assert!(html.contains("Project Status"));
        assert!(html.contains("Build:"));
        assert!(html.contains("<ul"));
        assert!(html.contains("Unit tests"));
    }

    #[test]
    fn layout_args_apply_to_compose() {
        use biscuit_terminal::utils::layout::Alignment;
        let layout = LayoutArgs {
            alignment: Some(Alignment::Center),
            ..LayoutArgs::default()
        };
        let compose = apply_layout_args(Compose::from("text"), &layout);
        // The layout flows onto the layout, observable via `layout()`.
        assert_eq!(
            biscuit_terminal::components::renderable::TerminalRenderable::layout(&compose)
                .alignment,
            Alignment::Center,
        );
    }
}
