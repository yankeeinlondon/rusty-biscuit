//! `md toc` subcommand implementation.

use crate::args::Cli;
use crate::io::load_markdown;
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::request::MdRequest;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::output::command::{CommandValue, prose_document};
use darkmatter::markdown::toc::{MarkdownToc, MarkdownTocNode, TocTree};
use std::path::PathBuf;

pub(super) fn run_toc(input: Option<&PathBuf>, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Object)?;
    let verbose = cli.verbose > 0;
    let result = load_markdown(input, request).map(|md| {
        let toc = md.toc();
        let prose = prose_document(&toc_markdown(&toc));
        let tree = toc.clone();
        Completed::success(CommandValue::new(toc, prose).with_terminal(move |options| {
            let mut view = TocTree::new(tree.clone());
            if verbose {
                view = view.verbose();
            }
            view.render(&Terminal::from_output_options(options))
        }))
    });
    run.finish(result)
}

/// The table of contents as nested list items linking to each heading.
fn toc_markdown(toc: &MarkdownToc) -> String {
    fn push(out: &mut String, node: &MarkdownTocNode, depth: usize) {
        out.push_str(&format!("{}- [{}](#{})\n", "  ".repeat(depth), node.title, node.slug));
        for child in &node.children {
            push(out, child, depth + 1);
        }
    }
    let mut out = String::new();
    for node in &toc.structure {
        push(&mut out, node, 0);
    }
    out
}
