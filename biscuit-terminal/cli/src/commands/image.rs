use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use crate::output::RenderMeta;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::terminal_image::{TerminalImage, parse_filepath_and_width, parse_width_spec};
use biscuit_terminal::discovery::cursor_position::cursor_position;
use biscuit_terminal::terminal::Terminal;
use clap::Args as ClapArgs;
use std::path::Path;
use std::time::Instant;

/// Display an image in the terminal
///
/// Supports width specification: "file.jpg|50%" or "file.jpg|80".
/// Supports PNG, JPG, JPEG, and GIF formats.
#[derive(ClapArgs, Debug, Clone)]
pub struct ImageArgs {
    /// Image file path with optional width spec (e.g., "photo.jpg|75%")
    #[arg(value_name = "FILEPATH", add = clap_complete::engine::ArgValueCompleter::new(crate::image_completer()))]
    pub filepath: String,

    /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
    ///
    /// Overrides inline width spec (e.g., "file.jpg|50%"). Aspect ratio is always preserved.
    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    /// Output rendering metadata to stderr (filename, file size, render time)
    #[arg(long)]
    pub meta: bool,

    /// Print cursor position diagnostics (before/after image render)
    #[arg(long)]
    pub debug: bool,
}

impl Run for ImageArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let width_str = self.width.as_ref().map(|w| w.to_string());
        render_image(
            &self.filepath,
            width_str.as_deref(),
            &self.layout,
            self.meta,
            self.debug,
        )
    }
}

/// Render an image to the terminal.
///
/// Supports width specification syntax: "file.jpg|50%" or "file.jpg|80"
/// CLI `--width` flag takes precedence over inline spec.
#[tracing::instrument(skip(layout, debug))]
pub fn render_image(
    image_spec: &str,
    cli_width: Option<&str>,
    layout: &LayoutArgs,
    meta: bool,
    debug: bool,
) -> color_eyre::Result<()> {
    let start_time = Instant::now();

    // Parse the filepath and optional inline width
    let (filepath, inline_width_spec) =
        parse_filepath_and_width(image_spec).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    // Resolve path relative to CWD
    let path = Path::new(&filepath);

    // Create the terminal image
    let mut term_image = TerminalImage::new(path).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    // CLI --width takes precedence over inline spec (e.g., "file.jpg|50%")
    let effective_width_spec = cli_width.or(inline_width_spec.as_deref());
    if let Some(ws) = effective_width_spec {
        term_image.width = parse_width_spec(ws).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
        term_image.width_raw = Some(format!("|{}", ws));
    }

    // Apply margin and alignment overrides
    apply_image_layout(&mut term_image, layout);

    // Get terminal capabilities
    let terminal = Terminal::new();

    // Debug: query cursor position BEFORE image render
    let pre_cursor = if debug { cursor_position() } else { None };

    // Render the image
    let output = term_image.render(&terminal);

    // Output the result with vertical margins
    emit_vertical_margins(layout, || emit_image_output(&output))?;

    // Debug: query cursor position AFTER image render + output
    if debug {
        let post_cursor = cursor_position();

        let term_height = terminal.height();
        let term_width = terminal.width();
        let dims = term_image.resolve_dimensions(term_width);
        let img = term_image.load_image().ok();
        let (cell_pw, cell_ph) = biscuit_terminal::discovery::fonts::cell_size()
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((8u32, 16u32));

        let (raw_height, image_rows_ceil) = if let Some(ref img) = img {
            let image_aspect = img.height() as f32 / img.width() as f32;
            let cell_aspect = cell_pw as f32 / cell_ph as f32;
            let raw = dims.image_width as f32 * image_aspect * cell_aspect;
            (raw, raw.ceil() as u32)
        } else {
            (0.0, 0)
        };

        let image_rows_floor = raw_height.floor() as u32;

        eprintln!("\x1b[2m--- image debug ---");
        eprintln!("terminal:     {}x{} (cols x rows)", term_width, term_height);
        eprintln!("cell size:    {}x{} px", cell_pw, cell_ph);
        eprintln!("image width:  {} cells", dims.image_width);
        eprintln!(
            "image height: {:.2} raw -> ceil={} floor={}",
            raw_height, image_rows_ceil, image_rows_floor
        );
        eprintln!("app:          {:?}", terminal.app);
        eprintln!(
            "cursor rows:  {} (used for CUD)",
            match terminal.app {
                biscuit_terminal::discovery::detection::TerminalApp::Warp =>
                    image_rows_floor.max(1),
                _ => image_rows_ceil,
            }
        );

        if let Some(pre) = pre_cursor {
            eprintln!("cursor BEFORE: row={} col={}", pre.row, pre.col);
            let would_extend_to = pre.row + image_rows_ceil;
            let predicted_scroll = would_extend_to.saturating_sub(term_height);
            eprintln!(
                "predicted:    image extends to row {} (screen has {})",
                would_extend_to, term_height
            );
            if predicted_scroll > 0 {
                eprintln!(
                    "predicted:    SCROLL needed ({} rows past bottom)",
                    predicted_scroll
                );
            } else {
                eprintln!("predicted:    no scroll needed");
            }
        } else {
            eprintln!("cursor BEFORE: (query failed)");
        }

        if let Some(post) = post_cursor {
            eprintln!("cursor AFTER:  row={} col={}", post.row, post.col);
        } else {
            eprintln!("cursor AFTER:  (query failed)");
        }

        if let (Some(pre), Some(post)) = (pre_cursor, post_cursor) {
            let actual_advance = post.row as i64 - pre.row as i64;
            eprintln!("actual delta:  {} rows", actual_advance);
            let expected = match terminal.app {
                biscuit_terminal::discovery::detection::TerminalApp::Warp => {
                    image_rows_floor.max(1)
                }
                _ => image_rows_ceil,
            };
            let diff = actual_advance - expected as i64;
            if diff != 0 {
                eprintln!(
                    "MISMATCH:     expected {} rows, got {} (off by {})",
                    expected, actual_advance, diff
                );
            } else {
                eprintln!("match:        cursor advanced exactly as expected");
            }
        }
        eprintln!("---\x1b[0m");
    }

    // Output metadata if requested
    if meta {
        let render_time_ms = start_time.elapsed().as_millis() as u64;
        let absolute_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let file_size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let render_meta = RenderMeta {
            filename: absolute_path.to_string_lossy().to_string(),
            cache_hit: false, // Images are loaded directly, no caching
            file_size_bytes,
            render_time_ms,
        };

        output_render_meta(&render_meta)?;
    }

    Ok(())
}
