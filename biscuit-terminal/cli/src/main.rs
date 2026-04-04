//! Terminal information utility CLI.
//!
//! Displays terminal metadata and capabilities including:
//! - Terminal application detection
//! - Color depth and mode
//! - Feature support (italics, images, underlines, OSC links)
//! - Multiplexing status
//! - OS and distribution information

use std::path::Path;

#[allow(unused_imports)] // layout is re-exported via `use crate::*` for args.rs
use biscuit_terminal::{
    components::{
        mermaid::QuadrantTheme,
        renderable::Renderable,
        terminal_image::{ImageWidth, parse_filepath_and_width},
        two_column::TwoColumn,
    },
    discovery::{clipboard, eval, fonts, mode_2027, osc_queries},
    terminal::Terminal,
    utils::{escape_codes, layout},
};
pub mod args;
pub mod commands;
pub mod output;
pub mod types;
use args::*;
use commands::*;
use output::*;

use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use clap_complete::engine::{CompletionCandidate, PathCompleter};

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

/// Emit rendered image output and flush immediately.
fn emit_image_output(output: &str) -> color_eyre::Result<()> {
    use std::io::Write;

    if output.is_empty() {
        return Ok(());
    }

    print!("{}", output);
    std::io::stdout().flush()?;
    Ok(())
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
        Some(Command::Image {
            ref filepath,
            ref width,
            ref layout,
            meta,
            debug,
        }) => {
            tracing::debug!(command = "image", filepath, width = ?width, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_image(filepath, width_str.as_deref(), layout, meta, debug);
        }
        Some(Command::Flowchart {
            vertical,
            inverse,
            ref title,
            ref width,
            ref layout,
            example,
            meta,
            ref content,
        }) => {
            tracing::debug!(command = "flowchart", vertical, inverse, example, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_flowchart(
                vertical,
                inverse,
                title.as_deref(),
                width_str.as_deref(),
                layout,
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
            ref layout,
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
            tracing::debug!(command = "quadrant", inverse, example, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_quadrant(
                x_axis.as_deref(),
                y_axis.as_deref(),
                title.as_deref(),
                top_left.as_deref(),
                top_right.as_deref(),
                bottom_left.as_deref(),
                bottom_right.as_deref(),
                inverse,
                width_str.as_deref(),
                layout,
                point_radius,
                label_size,
                *theme,
                q1_fill.as_ref().map(|c| c.as_str()),
                q2_fill.as_ref().map(|c| c.as_str()),
                q3_fill.as_ref().map(|c| c.as_str()),
                q4_fill.as_ref().map(|c| c.as_str()),
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
            ref layout,
            show_data,
            example,
            meta,
            ref data,
        }) => {
            tracing::debug!(command = "pie-chart", inverse, example, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_pie_chart(
                inverse,
                title.as_deref(),
                width_str.as_deref(),
                layout,
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
            ref layout,
            example,
            meta,
            ref commands,
        }) => {
            tracing::debug!(command = "git-graph", inverse, example, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_git_graph(
                inverse,
                title.as_deref(),
                width_str.as_deref(),
                layout,
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
            ref layout,
            horizontal,
            show_data_label,
            aspect_ratio,
            line,
            inverse,
            example,
            meta,
            ref data,
        }) => {
            tracing::debug!(command = "bar-chart", inverse, horizontal, example, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_xy_chart(
                XyChartType::Bar,
                title.as_deref(),
                x_axis.as_deref(),
                y_axis.as_deref(),
                width_str.as_deref(),
                layout,
                horizontal,
                show_data_label,
                aspect_ratio.map(|a| a.value()),
                line,  // add_line for bar chart
                false, // add_bar is false since we're a bar chart
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
            ref layout,
            horizontal,
            show_data_label,
            aspect_ratio,
            bar,
            inverse,
            example,
            meta,
            ref data,
        }) => {
            tracing::debug!(command = "line-chart", inverse, horizontal, example, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_xy_chart(
                XyChartType::Line,
                title.as_deref(),
                x_axis.as_deref(),
                y_axis.as_deref(),
                width_str.as_deref(),
                layout,
                horizontal,
                show_data_label,
                aspect_ratio.map(|a| a.value()),
                false, // add_line is false since we're a line chart
                bar,   // add_bar for line chart
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
            ref layout,
            ref section,
            inverse,
            example,
            meta,
            ref events,
        }) => {
            tracing::debug!(command = "timeline", inverse, example, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_timeline(
                title.as_deref(),
                width_str.as_deref(),
                layout,
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
            ref layout,
            inverse,
            example,
            meta,
            ref transitions,
        }) => {
            tracing::debug!(command = "state-diagram", inverse, example, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_state_diagram(
                title.as_deref(),
                width_str.as_deref(),
                layout,
                inverse,
                example,
                meta,
                transitions,
                args.json,
            );
        }
        Some(Command::GraphExpression {
            example,
            ref syntax,
            ref title,
            ref width,
            inverse,
            ref font,
            ref orientation,
            ref layout,
            meta,
            ref content,
        }) => {
            tracing::debug!(command = "graph-expression", inverse, example, syntax = ?syntax, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_graph_expression(
                example,
                syntax.clone(),
                title.as_deref(),
                width_str.as_deref(),
                inverse,
                font.as_deref(),
                orientation.clone(),
                layout,
                meta,
                content,
                args.json,
            );
        }
        Some(Command::Erd {
            ref title,
            ref width,
            ref layout,
            ref entity,
            inverse,
            example,
            meta,
            ref relationships,
        }) => {
            tracing::debug!(command = "erd", inverse, example, "Dispatching subcommand");
            let width_str = width.as_ref().map(|w| w.to_string());
            return render_erd(
                title.as_deref(),
                width_str.as_deref(),
                layout,
                entity,
                inverse,
                example,
                meta,
                relationships,
                args.json,
            );
        }
        Some(Command::PadLeft {
            width,
            ref text,
            truncate,
        }) => {
            tracing::debug!(command = "pad-left", width, truncate, "Dispatching subcommand");
            return render_pad_left(text, width, truncate);
        }
        Some(Command::PadRight {
            width,
            ref text,
            truncate,
        }) => {
            tracing::debug!(command = "pad-right", width, truncate, "Dispatching subcommand");
            return render_pad_right(text, width, truncate);
        }
        Some(Command::Prose {
            ref content,
            no_wrap,
            ref layout,
        }) => {
            tracing::debug!(command = "prose", no_wrap, "Dispatching subcommand");
            return render_prose(content, no_wrap, layout);
        }
        Some(Command::Quote {
            ref content,
            ref attribution,
            ref layout,
        }) => {
            tracing::debug!(command = "quote", "Dispatching subcommand");
            return render_quote(content, attribution.as_deref(), layout);
        }
        Some(Command::List {
            ref items,
            ref bullet,
            no_hanging_indent,
            ref layout,
        }) => {
            tracing::debug!(command = "list", items = items.len(), "Dispatching subcommand");
            return render_list(items, bullet, no_hanging_indent, layout);
        }
        Some(Command::Columns {
            ref left,
            ref right,
            gap,
            ref left_width,
            ref layout,
        }) => {
            tracing::debug!(command = "columns", gap, left_width = ?left_width, "Dispatching subcommand");
            let left_width_str = left_width.as_ref().map(|w| w.to_string());
            return render_columns(left, right, gap, left_width_str.as_deref(), layout);
        }
        Some(Command::Dir {
            ref path,
            depth,
            ref filter,
            skip_root,
            size,
            tokens,
            modified,
            updated,
            ref layout,
        }) => {
            let options = commands::DirOptions {
                show_size: size,
                show_token: tokens,
                show_modified: modified,
                show_updated: updated,
            };
            tracing::debug!(command = "dir", ?path, depth, ?filter, "Dispatching subcommand");
            return render_dir(path, depth, filter, skip_root, layout, &options);
        }
        None => {
            tracing::debug!(json = args.json, "No subcommand, showing terminal metadata");
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

/// Handles the --completions flag by generating shell completion scripts.
fn handle_completions(shell_type: &ShellType) -> color_eyre::Result<()> {
    let shell = match shell_type {
        ShellType::Bash => Shell::Bash,
        ShellType::Elvish => Shell::Elvish,
        ShellType::Fish => Shell::Fish,
        ShellType::Powershell => Shell::PowerShell,
        ShellType::Zsh => Shell::Zsh,
    };

    print_completions(shell);
    Ok(())
}

/// Prints shell completions to stdout.
fn print_completions(shell: Shell) {
    let mut cmd = Args::command();
    clap_complete::generate(shell, &mut cmd, "bt", &mut std::io::stdout());
}


/// Completes font family names from system fonts available to resvg.
pub fn font_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    use std::sync::OnceLock;

    static FONTS: OnceLock<Vec<String>> = OnceLock::new();
    let fonts =
        FONTS.get_or_init(biscuit_terminal::components::graph_expression::available_font_families);

    let prefix = current.to_str().unwrap_or("");
    let prefix_lower = prefix.to_lowercase();

    fonts
        .iter()
        .filter(|name| name.to_lowercase().starts_with(&prefix_lower))
        .map(|name| CompletionCandidate::new(name.as_str()))
        .collect()
}

/// Completes files with extensions: png, jpg, jpeg, gif (case-insensitive).
/// Also completes directories to allow navigation.
pub fn image_completer() -> PathCompleter {
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
