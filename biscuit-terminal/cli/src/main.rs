//! Terminal information utility CLI.
//!
//! Displays terminal metadata and capabilities including:
//! - Terminal application detection
//! - Color depth and mode
//! - Feature support (italics, images, underlines, OSC links)
//! - Multiplexing status
//! - OS and distribution information

use std::path::Path;

use biscuit_terminal::{
    components::{
        mermaid::{MermaidRenderer, QuadrantTheme},
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
use args::*;
use commands::*;
use output::*;

use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use clap_complete::engine::PathCompleter;

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
            return render_image(filepath, width.as_deref(), layout, meta, debug);
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
            return render_flowchart(
                vertical,
                inverse,
                title.as_deref(),
                width.as_deref(),
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
            return render_quadrant(
                x_axis.as_deref(),
                y_axis.as_deref(),
                title.as_deref(),
                top_left.as_deref(),
                top_right.as_deref(),
                bottom_left.as_deref(),
                bottom_right.as_deref(),
                inverse,
                width.as_deref(),
                layout,
                point_radius,
                label_size,
                *theme,
                q1_fill.as_deref(),
                q2_fill.as_deref(),
                q3_fill.as_deref(),
                q4_fill.as_deref(),
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
            return render_pie_chart(
                inverse,
                title.as_deref(),
                width.as_deref(),
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
            return render_git_graph(
                inverse,
                title.as_deref(),
                width.as_deref(),
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
            return render_xy_chart(
                XyChartType::Bar,
                title.as_deref(),
                x_axis.as_deref(),
                y_axis.as_deref(),
                width.as_deref(),
                layout,
                horizontal,
                show_data_label,
                aspect_ratio,
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
            return render_xy_chart(
                XyChartType::Line,
                title.as_deref(),
                x_axis.as_deref(),
                y_axis.as_deref(),
                width.as_deref(),
                layout,
                horizontal,
                show_data_label,
                aspect_ratio,
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
            return render_timeline(
                title.as_deref(),
                width.as_deref(),
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
            return render_state_diagram(
                title.as_deref(),
                width.as_deref(),
                layout,
                inverse,
                example,
                meta,
                transitions,
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
            return render_erd(
                title.as_deref(),
                width.as_deref(),
                layout,
                entity,
                inverse,
                example,
                meta,
                relationships,
                args.json,
            );
        }
        Some(Command::Prose {
            ref content,
            no_wrap,
            ref layout,
        }) => {
            return render_prose(content, no_wrap, layout);
        }
        Some(Command::Quote {
            ref content,
            ref attribution,
            ref layout,
        }) => {
            return render_quote(content, attribution.as_deref(), layout);
        }
        Some(Command::List {
            ref items,
            ref bullet,
            no_hanging_indent,
            ref layout,
        }) => {
            return render_list(items, bullet, no_hanging_indent, layout);
        }
        Some(Command::Columns {
            ref left,
            ref right,
            gap,
            ref left_width,
            ref layout,
        }) => {
            return render_columns(left, right, gap, left_width.as_deref(), layout);
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
            return render_dir(path, depth, filter, skip_root, layout, &options);
        }
        None => {
            // Default behavior: content analysis or terminal metadata
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

/// Handles the --completions flag.
///
/// If "help" is provided, shows setup instructions.
/// Otherwise, generates shell completion scripts.
fn handle_completions(shell_arg: &str) -> color_eyre::Result<()> {
    let shell_lower = shell_arg.to_lowercase();

    if shell_lower == "help" {
        print_completions_help();
        return Ok(());
    }

    let shell = match shell_lower.as_str() {
        "bash" => Shell::Bash,
        "elvish" => Shell::Elvish,
        "fish" => Shell::Fish,
        "powershell" | "pwsh" => Shell::PowerShell,
        "zsh" => Shell::Zsh,
        _ => {
            eprintln!(
                "error: invalid shell '{}'\n\nValid shells: bash, elvish, fish, powershell, zsh\n\nUse 'bt --completions help' for setup instructions.",
                shell_arg
            );
            std::process::exit(1);
        }
    };

    print_completions(shell);
    Ok(())
}

/// Prints shell completions to stdout.
fn print_completions(shell: Shell) {
    let mut cmd = Args::command();
    clap_complete::generate(shell, &mut cmd, "bt", &mut std::io::stdout());
}

/// Prints help about setting up shell completions.
fn print_completions_help() {
    println!(
        r#"bt Shell Completions Setup

Two methods are available for enabling tab completion:

DYNAMIC COMPLETIONS (recommended)
=================================
Dynamic completions call bt at completion time, providing:
- Image file filtering (only *.png, *.jpg, *.jpeg, *.gif)
- Always up-to-date with current bt version

Setup:
  Bash:  echo 'source <(COMPLETE=bash bt)' >> ~/.bashrc
  Zsh:   echo 'source <(COMPLETE=zsh bt)' >> ~/.zshrc
  Fish:  echo 'COMPLETE=fish bt | source' >> ~/.config/fish/config.fish

STATIC COMPLETIONS
==================
Static completions generate a script once. Faster but less features.

Setup:
  Bash:       bt --completions bash >> ~/.bashrc
  Zsh:        bt --completions zsh > ~/.zfunc/_bt
  Fish:       bt --completions fish > ~/.config/fish/completions/bt.fish
  PowerShell: bt --completions powershell >> $PROFILE

After setup, restart your shell or source the file to activate completions.
"#
    );
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
