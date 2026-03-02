use crate::args::*;
use crate::*;
use biscuit_terminal::components::terminal_image::{TerminalImage, parse_width_spec};
use biscuit_terminal::components::two_column::ColumnWidth;

/// Creates a path completer that filters for image files.
///
/// Formats an axis label for Mermaid quadrant charts.
///
/// If the label contains ` --> `, it's split into left and right parts:
///   "Low --> High" becomes `"Low" --> "High"`
///
/// Otherwise, the entire label is quoted (appears at axis start):
///   "My Label" becomes `"My Label"`
pub fn format_axis_label(label: &str) -> String {
    if let Some((left, right)) = label.split_once(" --> ") {
        format!("\"{}\" --> \"{}\"", left.trim(), right.trim())
    } else {
        format!("\"{}\"", label)
    }
}

/// Apply optional margin and alignment overrides to a `TerminalImage`.
pub fn apply_image_layout(term_image: &mut TerminalImage, layout: &LayoutArgs) {
    if let Some(ml) = layout.margin_left {
        term_image.margin_left = ml;
    }
    if let Some(mr) = layout.margin_right {
        term_image.margin_right = mr;
    }
    if let Some(align) = layout.alignment {
        term_image.alignment = match align {
            layout::Alignment::Left => std::fmt::Alignment::Left,
            layout::Alignment::Center => std::fmt::Alignment::Center,
            layout::Alignment::Right => std::fmt::Alignment::Right,
        };
    }
}

pub fn parse_column_width(spec: &str) -> color_eyre::Result<ColumnWidth> {
    let width = parse_width_spec(spec).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
    match width {
        ImageWidth::Percent(percent) => Ok(ColumnWidth::Percent(percent)),
        ImageWidth::Characters(chars) => Ok(ColumnWidth::Fixed(chars)),
        ImageWidth::Fill => Err(color_eyre::eyre::eyre!(
            "Column width does not support 'fill'. Use a percentage (e.g., 40%) or a character width (e.g., 24 or 24ch)."
        )),
    }
}

/// Emit blank lines for vertical margins around rendered content.
pub fn emit_vertical_margins(
    layout: &LayoutArgs,
    f: impl FnOnce() -> color_eyre::Result<()>,
) -> color_eyre::Result<()> {
    for _ in 0..layout.margin_top.unwrap_or(0) {
        println!();
    }
    f()?;
    for _ in 0..layout.margin_bottom.unwrap_or(0) {
        println!();
    }
    Ok(())
}

/// Render an image to the terminal.
///
/// Supports width specification syntax: "file.jpg|50%" or "file.jpg|80"
/// CLI `--width` flag takes precedence over inline spec.
pub fn render_image(
    image_spec: &str,
    cli_width: Option<&str>,
    layout: &LayoutArgs,
    meta: bool,
    debug: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::discovery::cursor_position::cursor_position;
    use std::time::Instant;

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
    for _ in 0..layout.margin_top.unwrap_or(0) {
        println!();
    }
    emit_image_output(&output)?;
    for _ in 0..layout.margin_bottom.unwrap_or(0) {
        println!();
    }

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

        eprintln!("{}", serde_json::to_string(&render_meta)?);
    }

    Ok(())
}

/// Example data for flowchart --example
/// Note: Each element is joined with newlines in the flowchart body
const FLOWCHART_EXAMPLE: &[&str] = &[
    "A[Start] --> B{Decision}",
    "B -->|Yes| C[Success]",
    "B -->|No| D[Retry]",
    "D --> B",
];
const FLOWCHART_EXAMPLE_CMD: &str = r#"bt flowchart "A[Start] --> B{Decision}" "B -->|Yes| C[Success]" "B -->|No| D[Retry]" "D --> B""#;

/// Display a mermaid diagram and optionally output metadata.
///
/// This helper function:
/// 1. Renders the diagram using the cached renderer
/// 2. Displays it in the terminal
/// 3. Optionally outputs metadata to stderr
///
/// Returns the render metadata (path, cache_hit, file_size, render_time) for further use.
pub fn display_mermaid_diagram(
    renderer: &MermaidRenderer,
    instructions: &str,
    diagram_type: &str,
    width: Option<&str>,
    layout: &LayoutArgs,
    meta: bool,
) -> color_eyre::Result<()> {
    use std::time::Instant;

    let start_time = Instant::now();

    // Render the diagram to a cached PNG file
    let (png_path, cache_hit) = match renderer.render_to_cached_png() {
        Ok((path, hit)) => (path, hit),
        Err(e) => {
            return handle_mermaid_error(e, instructions, diagram_type);
        }
    };

    let render_time_ms = start_time.elapsed().as_millis() as u64;

    // Parse width specification: default to 50% if not specified
    let image_width = match width {
        Some(w) => parse_width_spec(w).map_err(|e| color_eyre::eyre::eyre!("{}", e))?,
        None => ImageWidth::Percent(0.5),
    };

    // Use TerminalImage to display
    let terminal = Terminal::new();
    let mut term_image = TerminalImage::new(&png_path)
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?
        .with_width(image_width);

    // Apply margin and alignment overrides
    apply_image_layout(&mut term_image, layout);

    // Top margin
    for _ in 0..layout.margin_top.unwrap_or(0) {
        println!();
    }

    let output = term_image.render(&terminal);
    emit_image_output(&output)?;

    // Output metadata if requested
    if meta {
        let file_size_bytes = std::fs::metadata(&png_path).map(|m| m.len()).unwrap_or(0);

        let render_meta = RenderMeta {
            filename: png_path.to_string_lossy().to_string(),
            cache_hit,
            file_size_bytes,
            render_time_ms,
        };

        eprintln!("{}", serde_json::to_string(&render_meta)?);
    }

    // Bottom margin
    for _ in 0..layout.margin_bottom.unwrap_or(0) {
        println!();
    }

    // Let terminal settle after image rendering
    settle_terminal();

    Ok(())
}

/// Render a flowchart to the terminal.
///
/// Creates a Mermaid flowchart with the given content and renders it
/// using the MermaidRenderer. Default direction is left-right (LR),
/// use `vertical` for top-down (TD).
#[allow(clippy::too_many_arguments)]
pub fn render_flowchart(
    vertical: bool,
    inverse: bool,
    title: Option<&str>,
    width: Option<&str>,
    layout: &LayoutArgs,
    example: bool,
    meta: bool,
    content: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let content: Vec<String> = if example {
        FLOWCHART_EXAMPLE.iter().map(|s| s.to_string()).collect()
    } else {
        content.to_vec()
    };

    let direction = if vertical { "TD" } else { "LR" };
    // Join content with newlines and indentation for proper Mermaid syntax
    let body = content.join("\n    ");

    // Build mermaid instructions with optional title frontmatter
    let instructions = if let Some(title) = title {
        format!(
            "---\ntitle: {}\n---\nflowchart {}\n    {}",
            title, direction, body
        )
    } else {
        format!("flowchart {}\n    {}", direction, body)
    };

    if json {
        let output = serde_json::json!({
            "type": "flowchart",
            "direction": direction,
            "inverse": inverse,
            "title": title,
            "width": width,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer based on inverse flag
    let renderer = if inverse {
        // Inverse: solid background with opposite theme
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        // Default: transparent background with theme matching terminal
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "flowchart", width, layout, meta)?;

    // Print command used if example mode
    if example {
        print_example_command(FLOWCHART_EXAMPLE_CMD);
    }

    Ok(())
}

/// Example data for quadrant --example
const QUADRANT_EXAMPLE: &[&str] = &[
    "Campaign A: [0.3, 0.78]",
    "Campaign B: [0.45, 0.23]",
    "Campaign C: [0.57, 0.69]",
    "Campaign D: [0.78, 0.34]",
    "Campaign E: [0.40, 0.34]",
    "Campaign F: [0.65, 0.78]",
];
const QUADRANT_EXAMPLE_CMD: &str = r#"bt quadrant --title "Campaign Analysis" --x-axis "Low Reach --> High Reach" --y-axis "Low Engagement --> High Engagement" "Campaign A: [0.3, 0.78]" "Campaign B: [0.45, 0.23]" "Campaign C: [0.57, 0.69]" "Campaign D: [0.78, 0.34]" "Campaign E: [0.40, 0.34]" "Campaign F: [0.65, 0.78]""#;

/// Render a quadrant chart to the terminal.
///
/// Creates a Mermaid quadrantChart with the given configuration and data points,
/// then renders it using the MermaidRenderer.
#[allow(clippy::too_many_arguments)]
pub fn render_quadrant(
    x_axis: Option<&str>,
    y_axis: Option<&str>,
    title: Option<&str>,
    top_left: Option<&str>,
    top_right: Option<&str>,
    bottom_left: Option<&str>,
    bottom_right: Option<&str>,
    inverse: bool,
    width: Option<&str>,
    layout: &LayoutArgs,
    point_radius: Option<u32>,
    label_size: Option<u32>,
    theme: QuadrantTheme,
    q1_fill: Option<&str>,
    q2_fill: Option<&str>,
    q3_fill: Option<&str>,
    q4_fill: Option<&str>,
    example: bool,
    meta: bool,
    points: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::{MermaidConfig, MermaidTheme};
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (title, x_axis, y_axis, points): (Option<&str>, Option<&str>, Option<&str>, Vec<String>) =
        if example {
            (
                Some("Campaign Analysis"),
                Some("Low Reach --> High Reach"),
                Some("Low Engagement --> High Engagement"),
                QUADRANT_EXAMPLE.iter().map(|s| s.to_string()).collect(),
            )
        } else {
            (title, x_axis, y_axis, points.to_vec())
        };

    // Build the quadrantChart body
    let mut body_lines = Vec::new();

    // Title goes inside the chart body for quadrantChart (unlike other diagrams)
    if let Some(t) = title {
        body_lines.push(format!("    title \"{}\"", t));
    }

    // Axis labels: if contains " --> ", format as "Left" --> "Right"
    // Otherwise, quote the whole string for a centered label
    if let Some(x) = x_axis {
        body_lines.push(format!("    x-axis {}", format_axis_label(x)));
    }
    if let Some(y) = y_axis {
        body_lines.push(format!("    y-axis {}", format_axis_label(y)));
    }

    // Quadrant descriptions (1=top-left, 2=top-right, 3=bottom-left, 4=bottom-right)
    if let Some(tl) = top_left {
        body_lines.push(format!("    quadrant-1 \"{}\"", tl));
    }
    if let Some(tr) = top_right {
        body_lines.push(format!("    quadrant-2 \"{}\"", tr));
    }
    if let Some(bl) = bottom_left {
        body_lines.push(format!("    quadrant-3 \"{}\"", bl));
    }
    if let Some(br) = bottom_right {
        body_lines.push(format!("    quadrant-4 \"{}\"", br));
    }

    // Data points
    for point in &points {
        body_lines.push(format!("    {}", point));
    }

    let body = body_lines.join("\n");
    let instructions = format!("quadrantChart\n{}", body);

    if json {
        let output = serde_json::json!({
            "type": "quadrant",
            "x_axis": x_axis,
            "y_axis": y_axis,
            "title": title,
            "top_left": top_left,
            "top_right": top_right,
            "bottom_left": bottom_left,
            "bottom_right": bottom_right,
            "inverse": inverse,
            "width": width,
            "point_radius": point_radius,
            "label_size": label_size,
            "theme": theme.as_str(),
            "q1_fill": q1_fill,
            "q2_fill": q2_fill,
            "q3_fill": q3_fill,
            "q4_fill": q4_fill,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Build Mermaid config with styling options
    // Default label size: 18 for <= 6 points, 15 for > 6 points
    let config = {
        let mut cfg = MermaidConfig::new();

        // Apply point styling
        if let Some(r) = point_radius {
            cfg = cfg.with_point_radius(r);
        }
        let effective_label_size = label_size.unwrap_or(if points.len() <= 6 { 18 } else { 15 });
        cfg = cfg.with_point_label_font_size(effective_label_size);

        // Apply theme preset (sets default quadrant colors based on terminal color mode)
        let color_mode = Terminal::color_mode();
        cfg = theme.apply(cfg, color_mode);

        // Apply individual fill overrides (these take precedence over theme)
        if let Some(color) = q1_fill {
            cfg = cfg.with_quadrant_fill(1, color);
        }
        if let Some(color) = q2_fill {
            cfg = cfg.with_quadrant_fill(2, color);
        }
        if let Some(color) = q3_fill {
            cfg = cfg.with_quadrant_fill(3, color);
        }
        if let Some(color) = q4_fill {
            cfg = cfg.with_quadrant_fill(4, color);
        }

        cfg
    };

    // Configure renderer based on inverse flag, applying config for point styling
    let renderer = if inverse {
        // Inverse: solid background with opposite theme
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
            .with_config(config)
    } else {
        // Default: transparent background with theme matching terminal
        MermaidRenderer::for_terminal(&instructions).with_config(config)
    };

    // Display the diagram
    display_mermaid_diagram(
        &renderer,
        &instructions,
        "quadrant chart",
        width,
        layout,
        meta,
    )?;

    // Print command used if example mode
    if example {
        print_example_command(QUADRANT_EXAMPLE_CMD);
    }

    Ok(())
}

/// A parsed pie chart entry with optional color.
pub struct PieEntry {
    /// The Mermaid-formatted data line (e.g., `"Label" : value`)
    pub line: String,
    /// Optional hex color for this slice (e.g., `#3178c6`)
    pub color: Option<String>,
}

/// Parses pie chart data from various input formats.
///
/// Supports three formats:
/// 1. Simple: `"Label: value"` - quotes around label optional
/// 2. Semicolon-delimited: `"Label1: 10; Label2: 20"`
/// 3. Official Mermaid: `"\"Label\" : value"` - with quotes around label
///
/// Each format also supports an optional color suffix:
/// - `"Label: value color: #hex"` or `"Label: value #hex"`
///
/// Returns a vector of parsed entries with their optional colors.
pub fn parse_pie_data(data: &[String]) -> Vec<PieEntry> {
    let mut result = Vec::new();

    for item in data {
        // Check if this is a semicolon-delimited string
        if item.contains(';') {
            // Split by semicolon and process each part
            for part in item.split(';') {
                let part = part.trim();
                if !part.is_empty()
                    && let Some(parsed) = parse_single_pie_entry(part)
                {
                    result.push(parsed);
                }
            }
        } else {
            // Single entry
            if let Some(parsed) = parse_single_pie_entry(item) {
                result.push(parsed);
            }
        }
    }

    result
}

/// Extracts a hex color from the end of a string.
///
/// Looks for patterns like:
/// - `color: #3178c6` or `color:#3178c6`
/// - `#3178c6` (standalone at end)
///
/// Returns (remaining_string, Some(color)) if found, or (original, None) if not.
pub fn extract_color(s: &str) -> (&str, Option<String>) {
    let s = s.trim();

    // Try "color: #hex" or "color:#hex" pattern first
    if let Some(color_idx) = s.to_lowercase().rfind("color:") {
        let before = s[..color_idx].trim();
        let color_part = s[color_idx + 6..].trim(); // Skip "color:"

        if let Some(color) = parse_hex_color(color_part) {
            return (before, Some(color));
        }
    }

    // Try standalone #hex at the end
    // Find the last whitespace and check if what follows is a hex color
    if let Some(last_space) = s.rfind(char::is_whitespace) {
        let potential_color = s[last_space + 1..].trim();
        if let Some(color) = parse_hex_color(potential_color) {
            return (s[..last_space].trim(), Some(color));
        }
    }

    (s, None)
}

/// Parses a hex color string, returning it normalized if valid.
///
/// Accepts: `#rgb`, `#rrggbb`, `#rrggbbaa`
pub fn parse_hex_color(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with('#') {
        return None;
    }

    let hex_part = &s[1..];
    // Valid lengths: 3 (#rgb), 6 (#rrggbb), or 8 (#rrggbbaa)
    if !matches!(hex_part.len(), 3 | 6 | 8) {
        return None;
    }

    // Check all characters are valid hex
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    Some(s.to_string())
}

/// Parses a single pie chart entry into Mermaid format with optional color.
///
/// Handles:
/// - `Label: value` → `"Label" : value`
/// - `"Label" : value` → `"Label" : value` (passthrough)
/// - `"Label": value` → `"Label" : value`
///
/// Also extracts optional color from end:
/// - `Label: value color: #hex`
/// - `Label: value #hex`
pub fn parse_single_pie_entry(entry: &str) -> Option<PieEntry> {
    let entry = entry.trim();
    if entry.is_empty() {
        return None;
    }

    // Extract color from the end first (before parsing the rest)
    let (entry_without_color, color) = extract_color(entry);

    // Check if it's already in official Mermaid format (starts with quote)
    if let Some(stripped) = entry_without_color.strip_prefix('"') {
        // Find the closing quote
        if let Some(close_quote_idx) = stripped.find('"') {
            let label = &stripped[..close_quote_idx];
            let rest = &stripped[close_quote_idx + 1..]; // Skip the closing quote

            // Find the colon and value
            if let Some(colon_idx) = rest.find(':') {
                let value = rest[colon_idx + 1..].trim();
                if !value.is_empty() {
                    return Some(PieEntry {
                        line: format!("\"{}\" : {}", label, value),
                        color,
                    });
                }
            }
        }
        // If parsing failed, try the simple format below
    }

    // Simple format: Label: value
    if let Some(colon_idx) = entry_without_color.find(':') {
        let label = entry_without_color[..colon_idx].trim();
        let value = entry_without_color[colon_idx + 1..].trim();

        if !label.is_empty() && !value.is_empty() {
            // Remove surrounding quotes if present
            let label = label.trim_matches('"');
            return Some(PieEntry {
                line: format!("\"{}\" : {}", label, value),
                color,
            });
        }
    }

    None
}

/// Builds the Mermaid init directive for pie chart colors.
///
/// If any entries have colors, generates:
/// `%%{init: {'themeVariables': {'pie1': '#color', 'pie2': '#color', ...}}}%%`
pub fn build_pie_init_directive(entries: &[PieEntry]) -> Option<String> {
    let color_vars: Vec<String> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            entry
                .color
                .as_ref()
                .map(|c| format!("'pie{}': '{}'", i + 1, c))
        })
        .collect();

    if color_vars.is_empty() {
        None
    } else {
        Some(format!(
            "%%{{init: {{'themeVariables': {{{}}}}}}}%%",
            color_vars.join(", ")
        ))
    }
}

/// Example data for pie-chart --example
const PIE_CHART_EXAMPLE: &[&str] = &["TypeScript: 45 #3178C6", "Rust: 35 #A72145", "Python: 20"];
const PIE_CHART_EXAMPLE_CMD: &str =
    r#"bt pie-chart "TypeScript: 45 #3178C6" "Rust: 35 #A72145" "Python: 20""#;

/// Render a pie chart to the terminal.
///
/// Creates a Mermaid pie chart with the given data and renders it
/// using the MermaidRenderer.
#[allow(clippy::too_many_arguments)]
pub fn render_pie_chart(
    inverse: bool,
    title: Option<&str>,
    width: Option<&str>,
    layout: &LayoutArgs,
    show_data: bool,
    example: bool,
    meta: bool,
    data: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let data: Vec<String> = if example {
        PIE_CHART_EXAMPLE.iter().map(|s| s.to_string()).collect()
    } else {
        data.to_vec()
    };

    // Parse the input data into Mermaid format (with optional colors)
    let parsed_entries = parse_pie_data(&data);

    if parsed_entries.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No valid data points provided. Use format: \"Label: value\""
        ));
    }

    // Build the init directive for custom colors (if any)
    let init_directive = build_pie_init_directive(&parsed_entries);

    // Build the pie chart body
    let show_data_str = if show_data { " showData" } else { "" };
    let title_line = title
        .map(|t| format!("    title {}", t))
        .unwrap_or_default();

    let data_lines: String = parsed_entries
        .iter()
        .map(|e| format!("    {}", e.line))
        .collect::<Vec<_>>()
        .join("\n");

    // Combine all parts: init directive (optional) + pie declaration + title (optional) + data
    let mut instructions_parts = Vec::new();

    if let Some(ref init) = init_directive {
        instructions_parts.push(init.clone());
    }

    if title_line.is_empty() {
        instructions_parts.push(format!("pie{}\n{}", show_data_str, data_lines));
    } else {
        instructions_parts.push(format!(
            "pie{}\n{}\n{}",
            show_data_str, title_line, data_lines
        ));
    }

    let instructions = instructions_parts.join("\n");

    if json {
        let output = serde_json::json!({
            "type": "pie-chart",
            "inverse": inverse,
            "title": title,
            "width": width,
            "show_data": show_data,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer based on inverse flag
    let renderer = if inverse {
        // Inverse: solid background with opposite theme
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        // Default: transparent background with theme matching terminal
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "pie chart", width, layout, meta)?;

    // Print command used if example mode
    if example {
        print_example_command(PIE_CHART_EXAMPLE_CMD);
    }

    Ok(())
}

/// Example data for git-graph --example
const GIT_GRAPH_EXAMPLE: &[&str] = &[
    "commit",
    "commit",
    "branch feature",
    "checkout feature",
    "commit",
    "commit",
    "checkout main",
    "commit",
    "merge feature",
    "commit",
];
const GIT_GRAPH_EXAMPLE_CMD: &str = r#"bt git-graph "commit" "commit" "branch feature" "checkout feature" "commit" "commit" "checkout main" "commit" "merge feature" "commit""#;

/// Render a git graph to the terminal.
///
/// Creates a Mermaid gitGraph with the given commands and renders it
/// using the MermaidRenderer.
#[allow(clippy::too_many_arguments)]
pub fn render_git_graph(
    inverse: bool,
    title: Option<&str>,
    width: Option<&str>,
    layout: &LayoutArgs,
    example: bool,
    meta: bool,
    commands: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let commands: Vec<String> = if example {
        GIT_GRAPH_EXAMPLE.iter().map(|s| s.to_string()).collect()
    } else {
        commands.to_vec()
    };

    let body = commands
        .iter()
        .map(|cmd| format!("    {}", cmd))
        .collect::<Vec<_>>()
        .join("\n");

    // Build mermaid instructions with optional title frontmatter
    let instructions = if let Some(title) = title {
        format!("---\ntitle: {}\n---\ngitGraph\n{}", title, body)
    } else {
        format!("gitGraph\n{}", body)
    };

    if json {
        let output = serde_json::json!({
            "type": "git-graph",
            "inverse": inverse,
            "title": title,
            "width": width,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer based on inverse flag
    let renderer = if inverse {
        // Inverse: solid background with opposite theme
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        // Default: transparent background with theme matching terminal
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "git-graph", width, layout, meta)?;

    // Print command used if example mode
    if example {
        print_example_command(GIT_GRAPH_EXAMPLE_CMD);
    }

    Ok(())
}

/// XY chart type selector
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XyChartType {
    Bar,
    Line,
}

/// Example data for bar-chart --example
const BAR_CHART_EXAMPLE: &[&str] = &["12", "28", "45", "38", "22", "55"];
const BAR_CHART_EXAMPLE_CMD: &str = "bt bar-chart --title \"Monthly Revenue\" --x-axis \"Jan,Feb,Mar,Apr,May,Jun\" --y-axis \"$ (thousands)\" 12 28 45 38 22 55";

/// Example data for line-chart --example
const LINE_CHART_EXAMPLE: &[&str] = &["20", "22", "19", "23", "25", "21", "24"];
const LINE_CHART_EXAMPLE_CMD: &str = "bt line-chart --title \"Weekly Temperature\" --x-axis \"Mon,Tue,Wed,Thu,Fri,Sat,Sun\" --y-axis \"°C\" 20 22 19 23 25 21 24";

/// Render an XY chart (bar or line) to the terminal.
///
/// Uses Mermaid's xychart-beta syntax.
#[allow(clippy::too_many_arguments)]
pub fn render_xy_chart(
    chart_type: XyChartType,
    title: Option<&str>,
    x_axis: Option<&str>,
    y_axis: Option<&str>,
    width: Option<&str>,
    layout: &LayoutArgs,
    horizontal: bool,
    show_data_label: bool,
    aspect_ratio: Option<f32>,
    add_line: bool,
    add_bar: bool,
    inverse: bool,
    example: bool,
    meta: bool,
    data: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (data, use_example_labels): (Vec<String>, bool) = if example {
        let example_data = match chart_type {
            XyChartType::Bar => BAR_CHART_EXAMPLE,
            XyChartType::Line => LINE_CHART_EXAMPLE,
        };
        (example_data.iter().map(|s| s.to_string()).collect(), true)
    } else {
        (data.to_vec(), false)
    };

    // Parse input data
    let values = parse_xy_data(&data)?;

    if values.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No valid data values provided. Use format: \"1 2 3\" or \"[1,2,3]\" or \"1,2,3\""
        ));
    }

    // Get example titles/labels for example mode
    let (eff_title, eff_x_axis, eff_y_axis) = if use_example_labels {
        match chart_type {
            XyChartType::Bar => (
                Some("Monthly Revenue"),
                Some("Jan,Feb,Mar,Apr,May,Jun"),
                Some("$ (thousands)"),
            ),
            XyChartType::Line => (
                Some("Weekly Temperature"),
                Some("Mon,Tue,Wed,Thu,Fri,Sat,Sun"),
                Some("°C"),
            ),
        }
    } else {
        (title, x_axis, y_axis)
    };

    // Build init directive for configuration
    let aspect = aspect_ratio.unwrap_or(1.5);
    let init_config = format!(
        "%%{{init: {{\"xychart\": {{\"showTitle\": {}, \"xAxis\": {{\"showLabel\": {}}}, \"yAxis\": {{\"showLabel\": {}}}}}}}}}%%",
        eff_title.is_some(),
        eff_x_axis.is_some(),
        eff_y_axis.is_some()
    );

    // Build chart declaration
    let orientation = if horizontal { "horizontal" } else { "" };
    let chart_decl = format!("xychart-beta {}", orientation).trim().to_string();

    // Build x-axis line
    let x_axis_line = if let Some(labels) = eff_x_axis {
        // Check if it contains commas (categories) or is just a label
        if labels.contains(',') {
            let cats: Vec<&str> = labels.split(',').map(|s| s.trim()).collect();
            format!("    x-axis [{}]", cats.join(", "))
        } else {
            format!("    x-axis \"{}\"", labels)
        }
    } else {
        // Generate default labels based on data count
        let default_labels: Vec<String> = (1..=values.len()).map(|i| i.to_string()).collect();
        format!("    x-axis [{}]", default_labels.join(", "))
    };

    // Build y-axis line
    let y_axis_line = if let Some(label) = eff_y_axis {
        format!("    y-axis \"{}\"", label)
    } else {
        String::new()
    };

    // Build title line
    let title_line = eff_title
        .map(|t| format!("    title \"{}\"", t))
        .unwrap_or_default();

    // Build data series
    let data_str: String = values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let primary_series = match chart_type {
        XyChartType::Bar => format!("    bar [{}]", data_str),
        XyChartType::Line => format!("    line [{}]", data_str),
    };

    let secondary_series = if add_line && chart_type == XyChartType::Bar {
        format!("\n    line [{}]", data_str)
    } else if add_bar && chart_type == XyChartType::Line {
        format!("\n    bar [{}]", data_str)
    } else {
        String::new()
    };

    // Combine all parts
    let mut parts = vec![init_config, chart_decl];
    if !title_line.is_empty() {
        parts.push(title_line);
    }
    parts.push(x_axis_line);
    if !y_axis_line.is_empty() {
        parts.push(y_axis_line);
    }
    parts.push(primary_series);
    if !secondary_series.is_empty() {
        parts.push(secondary_series.trim().to_string());
    }

    let instructions = parts.join("\n");

    if json {
        let output = serde_json::json!({
            "type": match chart_type {
                XyChartType::Bar => "bar-chart",
                XyChartType::Line => "line-chart",
            },
            "inverse": inverse,
            "title": eff_title,
            "x_axis": eff_x_axis,
            "y_axis": eff_y_axis,
            "horizontal": horizontal,
            "show_data_label": show_data_label,
            "aspect_ratio": aspect,
            "add_line": add_line,
            "add_bar": add_bar,
            "values": values,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer based on inverse flag
    let renderer = if inverse {
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    let chart_name = match chart_type {
        XyChartType::Bar => "bar chart",
        XyChartType::Line => "line chart",
    };
    display_mermaid_diagram(&renderer, &instructions, chart_name, width, layout, meta)?;

    // Print command used if example mode
    if example {
        let cmd = match chart_type {
            XyChartType::Bar => BAR_CHART_EXAMPLE_CMD,
            XyChartType::Line => LINE_CHART_EXAMPLE_CMD,
        };
        print_example_command(cmd);
    }

    Ok(())
}

/// Parse XY chart data from various input formats.
///
/// Supports:
/// - JSON array: "[1, 8, 7, 5]"
/// - Comma-separated: "1,8,7,5" or "1, 8, 7, 5"
/// - Space-separated arguments: "1" "8" "7" "5"
pub fn parse_xy_data(data: &[String]) -> color_eyre::Result<Vec<f64>> {
    let mut values = Vec::new();

    for item in data {
        let trimmed = item.trim();

        // Try JSON array first
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len() - 1];
            for part in inner.split(',') {
                let v: f64 = part.trim().parse().map_err(|_| {
                    color_eyre::eyre::eyre!("Invalid number in JSON array: '{}'", part.trim())
                })?;
                values.push(v);
            }
            continue;
        }

        // Try comma-separated
        if trimmed.contains(',') {
            for part in trimmed.split(',') {
                let v: f64 = part
                    .trim()
                    .parse()
                    .map_err(|_| color_eyre::eyre::eyre!("Invalid number: '{}'", part.trim()))?;
                values.push(v);
            }
            continue;
        }

        // Single value
        let v: f64 = trimmed
            .parse()
            .map_err(|_| color_eyre::eyre::eyre!("Invalid number: '{}'", trimmed))?;
        values.push(v);
    }

    Ok(values)
}

/// Example data for timeline --example
const TIMELINE_EXAMPLE: &[&str] = &[
    "2002: LinkedIn",
    "2004: Facebook",
    "2005: YouTube",
    "2006: Twitter",
    "2010: Instagram",
    "2011: Snapchat",
];
const TIMELINE_EXAMPLE_CMD: &str = "bt timeline --title \"Social Media History\" \"2002: LinkedIn\" \"2004: Facebook\" \"2005: YouTube\" \"2006: Twitter\" \"2010: Instagram\" \"2011: Snapchat\"";

/// Render a timeline diagram to the terminal.
#[allow(clippy::too_many_arguments)]
pub fn render_timeline(
    title: Option<&str>,
    width: Option<&str>,
    layout: &LayoutArgs,
    sections: &[String],
    inverse: bool,
    example: bool,
    meta: bool,
    events: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (events, eff_title): (Vec<String>, Option<&str>) = if example {
        (
            TIMELINE_EXAMPLE.iter().map(|s| s.to_string()).collect(),
            Some("Social Media History"),
        )
    } else {
        (events.to_vec(), title)
    };

    if events.is_empty() && sections.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No events provided. Use format: \"YYYY: Event description\""
        ));
    }

    // Validate event format
    for event in &events {
        if !event.contains(':') {
            return Err(color_eyre::eyre::eyre!(
                "Invalid event format '{}'. Expected 'YYYY: Description'",
                event
            ));
        }
    }

    // Build the timeline
    let mut lines = vec!["timeline".to_string()];

    if let Some(t) = eff_title {
        lines.push(format!("    title {}", t));
    }

    // If no sections, add all events directly
    if sections.is_empty() {
        for event in &events {
            lines.push(format!("    {}", event));
        }
    } else {
        // With sections, we need to interleave section headers and events
        // For now, put all events under the first section if sections are provided
        // Users can use multiple --section flags for grouping
        for (i, section) in sections.iter().enumerate() {
            lines.push(format!("    section {}", section));
            // Put a portion of events under each section
            let events_per_section = events.len().div_ceil(sections.len());
            let start = i * events_per_section;
            let end = ((i + 1) * events_per_section).min(events.len());
            for event in events.get(start..end).unwrap_or(&[]) {
                lines.push(format!("        {}", event));
            }
        }
    }

    let instructions = lines.join("\n");

    if json {
        let output = serde_json::json!({
            "type": "timeline",
            "inverse": inverse,
            "title": eff_title,
            "sections": sections,
            "events": events,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer
    let renderer = if inverse {
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "timeline", width, layout, meta)?;

    if example {
        print_example_command(TIMELINE_EXAMPLE_CMD);
    }

    Ok(())
}

/// Example data for state-diagram --example
const STATE_DIAGRAM_EXAMPLE: &[&str] = &[
    "[*] --> Idle",
    "Idle --> Running: start",
    "Running --> Idle: stop",
    "Running --> Error: failure",
    "Error --> Idle: reset",
    "Idle --> [*]: shutdown",
];
const STATE_DIAGRAM_EXAMPLE_CMD: &str = "bt state-diagram --title \"Process States\" \"[*] --> Idle\" \"Idle --> Running: start\" \"Running --> Idle: stop\" \"Running --> Error: failure\" \"Error --> Idle: reset\" \"Idle --> [*]: shutdown\"";

/// Render a state diagram to the terminal.
#[allow(clippy::too_many_arguments)]
pub fn render_state_diagram(
    title: Option<&str>,
    width: Option<&str>,
    layout: &LayoutArgs,
    inverse: bool,
    example: bool,
    meta: bool,
    transitions: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (transitions, eff_title): (Vec<String>, Option<&str>) = if example {
        (
            STATE_DIAGRAM_EXAMPLE
                .iter()
                .map(|s| s.to_string())
                .collect(),
            Some("Process States"),
        )
    } else {
        (transitions.to_vec(), title)
    };

    if transitions.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No transitions provided. Use format: \"State1 --> State2\" or \"[*] --> State\""
        ));
    }

    // Build the state diagram
    let mut lines = vec!["stateDiagram-v2".to_string()];

    // Add title if provided (using note or direction for now, title isn't directly supported)
    // Actually, stateDiagram doesn't have a title directive, we'll skip it for the diagram itself
    // but include it in JSON output

    for transition in &transitions {
        lines.push(format!("    {}", transition));
    }

    let instructions = lines.join("\n");

    if json {
        let output = serde_json::json!({
            "type": "state-diagram",
            "inverse": inverse,
            "title": eff_title,
            "transitions": transitions,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer
    let renderer = if inverse {
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(
        &renderer,
        &instructions,
        "state diagram",
        width,
        layout,
        meta,
    )?;

    if example {
        print_example_command(STATE_DIAGRAM_EXAMPLE_CMD);
    }

    Ok(())
}

/// Example data for erd --example
/// Note: Mermaid ERD requires each attribute on its own line inside the entity block
const ERD_EXAMPLE_ENTITIES: &[&str] = &[
    "Customer {\n        int id PK\n        string name\n        string email\n    }",
    "Order {\n        int id PK\n        date orderDate\n        int customerId FK\n    }",
    "Product {\n        int id PK\n        string name\n        decimal price\n    }",
    "OrderItem {\n        int orderId FK\n        int productId FK\n        int quantity\n    }",
];
const ERD_EXAMPLE_RELATIONSHIPS: &[&str] = &[
    "Customer ||--o{ Order : places",
    "Order ||--|{ OrderItem : contains",
    "Product ||--o{ OrderItem : \"ordered in\"",
];
const ERD_EXAMPLE_CMD: &str = "bt erd --title \"E-Commerce Schema\" \\\n  --entity \"Customer { int id PK }\" \\\n  --entity \"Order { int id PK }\" \\\n  \"Customer ||--o{ Order : places\"";

/// Render an ERD to the terminal.
#[allow(clippy::too_many_arguments)]
pub fn render_erd(
    title: Option<&str>,
    width: Option<&str>,
    layout: &LayoutArgs,
    entities: &[String],
    inverse: bool,
    example: bool,
    meta: bool,
    relationships: &[String],
    json: bool,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidTheme;
    use std::io::Write;

    let _ = std::io::stdout().flush();

    // Use example data if --example flag is set
    let (entities, relationships, eff_title): (Vec<String>, Vec<String>, Option<&str>) = if example
    {
        (
            ERD_EXAMPLE_ENTITIES.iter().map(|s| s.to_string()).collect(),
            ERD_EXAMPLE_RELATIONSHIPS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            Some("E-Commerce Schema"),
        )
    } else {
        (entities.to_vec(), relationships.to_vec(), title)
    };

    if relationships.is_empty() && entities.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No relationships or entities provided. Use format: \"Entity1 ||--o{{ Entity2 : label\""
        ));
    }

    // Build the ERD
    let mut lines = vec!["erDiagram".to_string()];

    // Add title if provided
    if let Some(t) = eff_title {
        // ERD doesn't have native title support, but we can add it as a note
        // For now, we'll just skip it in the diagram
        let _ = t; // suppress unused warning
    }

    // Add entity definitions
    for entity in &entities {
        lines.push(format!("    {}", entity));
    }

    // Add relationships
    for rel in &relationships {
        lines.push(format!("    {}", rel));
    }

    let instructions = lines.join("\n");

    if json {
        let output = serde_json::json!({
            "type": "erd",
            "inverse": inverse,
            "title": eff_title,
            "entities": entities,
            "relationships": relationships,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Configure renderer
    let renderer = if inverse {
        let theme = MermaidTheme::for_color_mode(Terminal::color_mode()).inverse();
        MermaidRenderer::new(&instructions)
            .with_theme(theme)
            .with_transparent_background(false)
    } else {
        MermaidRenderer::for_terminal(&instructions)
    };

    // Display the diagram
    display_mermaid_diagram(&renderer, &instructions, "ERD", width, layout, meta)?;

    if example {
        print_example_command(ERD_EXAMPLE_CMD);
    }

    Ok(())
}

/// Handle Mermaid rendering errors with user-friendly output.
///
/// Parses mmdc errors to extract syntax information and formats
/// them nicely without JavaScript callstacks.
pub fn handle_mermaid_error(
    error: biscuit_terminal::components::mermaid::MermaidRenderError,
    instructions: &str,
    diagram_type: &str,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidRenderError;

    // Check for NO_COLOR
    let no_color = std::env::var("NO_COLOR").is_ok();
    let red = if no_color { "" } else { "\x1b[31m" };
    let bold = if no_color { "" } else { "\x1b[1m" };
    let dim = if no_color { "" } else { "\x1b[2m" };
    let reset = if no_color { "" } else { "\x1b[0m" };

    match error {
        MermaidRenderError::MmdcExecutionFailed { stderr, .. } => {
            // Check if this is a parse/syntax error
            if stderr.contains("Parse error") || stderr.contains("Expecting") {
                // Add breathing room before error
                eprintln!();
                eprintln!("{}{}Error:{} Mermaid Syntax Error\n", red, bold, reset);

                // Extract useful lines from stderr (skip JS callstack and useless line numbers)
                for line in stderr.lines() {
                    // Include:
                    // - Context lines showing actual mermaid code (starts with ...)
                    // - Error pointer lines (contains ^ and dashes)
                    // - "Expecting" lines describing what was expected
                    // Skip: "Error: Parse error on line X:", JS callstack lines
                    let is_context_line = line.starts_with("...");
                    let is_pointer_line =
                        line.contains("^") && line.chars().filter(|c| *c == '-').count() > 3;
                    let is_expecting_line =
                        line.starts_with("Expecting") || line.contains("Expecting '");

                    if is_context_line || is_pointer_line || is_expecting_line {
                        eprintln!("{}", line);
                    }
                }

                // Show the mermaid block that was defined
                eprintln!(
                    "\n{}Mermaid {} was defined as:{}\n",
                    dim, diagram_type, reset
                );
                eprintln!("```mermaid\n{}\n```", instructions);
            } else {
                // Non-syntax error, show the full error (with breathing room)
                eprintln!();
                eprintln!("{}{}Error:{} {}", red, bold, reset, stderr);
            }
        }
        MermaidRenderError::MmdcNotFound => {
            eprintln!(
                "{}{}Error:{} mmdc CLI not found.\n\nInstall with: npm install -g @mermaid-js/mermaid-cli",
                red, bold, reset
            );
        }
        MermaidRenderError::NpmNotFound => {
            eprintln!(
                "{}{}Error:{} npm not found.\n\nInstall Node.js and npm to render Mermaid diagrams.",
                red, bold, reset
            );
        }
        _ => {
            eprintln!("{}{}Error:{} {}", red, bold, reset, error);
        }
    }

    // Return error to get non-zero exit code
    std::process::exit(1);
}

/// Render prose content with styling tokens to the terminal.
pub fn render_prose(
    content: &[String],
    no_wrap: bool,
    layout: &LayoutArgs,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::Renderable;
    use biscuit_terminal::utils::layout::{Margin, WordWrap};

    // Join all content pieces with spaces
    let text = content.join(" ");

    if text.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No content provided. Usage: bt prose \"Hello {{bold}}world{{reset}}!\""
        ));
    }

    // Unescape common escape sequences (shell passes literal \n, \t, etc.)
    let text = text
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r");

    // Build the Prose component
    let mut prose = Prose::new(&text);

    // Configure word wrapping
    if no_wrap {
        prose = prose.with_word_wrap(WordWrap::None);
    } else {
        prose = prose.with_word_wrap(WordWrap::WrapProse(None, None));
    }

    // Configure margins
    if let Some(left) = layout.margin_left {
        prose = prose.with_left_margin(Margin::Chars(left));
    }
    if let Some(right) = layout.margin_right {
        prose = prose.with_right_margin(Margin::Chars(right));
    }

    // Configure alignment
    if let Some(align) = layout.alignment {
        prose = prose.alignment(align);
    }

    // Render using fallback_render for terminal-aware output
    let term = Terminal::new();
    let output = prose.render(&term);

    emit_vertical_margins(layout, || {
        println!("{}", output);
        Ok(())
    })
}

/// Render prose content inside a block quote.
pub fn render_quote(
    content: &[String],
    attribution: Option<&str>,
    layout: &LayoutArgs,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::block_quote::BlockQuote;
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
    use biscuit_terminal::utils::layout::Margin;
    use std::rc::Rc;

    // Join all content pieces with spaces
    let text = content.join(" ");

    if text.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No content provided. Usage: bt quote \"To be or not to be\" --attribution \"Shakespeare\""
        ));
    }

    // Unescape common escape sequences (shell passes literal \n, \t, etc.)
    let text = text
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r");

    // Build the Prose component for the content
    let prose = Prose::new(&text);

    // Build the BlockQuote with the Prose content
    let mut quote = BlockQuote::new(RenderableContent::Component(Rc::new(prose)), attribution);

    // Configure margins
    if let Some(left) = layout.margin_left {
        quote = quote.left_margin(Margin::Chars(left));
    }
    if let Some(right) = layout.margin_right {
        quote = quote.right_margin(Margin::Chars(right));
    }

    // Configure alignment
    if let Some(align) = layout.alignment {
        quote = quote.alignment(align);
    }

    // Render using fallback_render for terminal-aware output
    let term = Terminal::new();
    let output = quote.render(&term);

    emit_vertical_margins(layout, || {
        println!("{}", output);
        Ok(())
    })
}

/// Render a bulleted list with hanging indents.
pub fn render_list(
    items: &[String],
    bullet: &str,
    no_hanging_indent: bool,
    layout: &LayoutArgs,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::list::UnorderedList;
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
    use biscuit_terminal::utils::layout::Margin;
    use std::rc::Rc;

    if items.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No items provided. Usage: bt list \"First item\" \"Second item\" \"Third item\""
        ));
    }

    // Convert each item to a Prose component wrapped in RenderableContent
    let prose_items: Vec<RenderableContent> = items
        .iter()
        .map(|item| {
            // Unescape common escape sequences (shell passes literal \n, \t, etc.)
            let text = item
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r");

            let prose = Prose::new(&text);
            RenderableContent::Component(Rc::new(prose))
        })
        .collect();

    // Build the UnorderedList with custom bullet
    let mut list = UnorderedList::from(prose_items).with_bullet(bullet);

    // Disable hanging indent if requested
    if no_hanging_indent {
        list = list.without_hanging_indent();
    }

    // Configure margins
    if let Some(left) = layout.margin_left {
        list = list.left_margin(Margin::Chars(left));
    }
    if let Some(right) = layout.margin_right {
        list = list.right_margin(Margin::Chars(right));
    }

    // Configure alignment
    if let Some(align) = layout.alignment {
        list = list.alignment(align);
    }

    // Render using fallback_render for terminal-aware output
    let term = Terminal::new();
    let output = list.render(&term);

    emit_vertical_margins(layout, || {
        println!("{}", output);
        Ok(())
    })
}

/// Render two columns of text side by side.
pub fn render_columns(
    left: &str,
    right: &str,
    gap: u32,
    left_width: Option<&str>,
    layout: &LayoutArgs,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::renderable::Renderable;
    use biscuit_terminal::utils::layout::Margin;

    let left_text = left
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r");
    let right_text = right
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r");

    let mut columns = TwoColumn::new(left_text, right_text).with_gap(gap);

    if let Some(spec) = left_width {
        columns = columns.with_left_width(parse_column_width(spec)?);
    }

    if let Some(left) = layout.margin_left {
        columns = columns.left_margin(Margin::Chars(left));
    }
    if let Some(right) = layout.margin_right {
        columns = columns.right_margin(Margin::Chars(right));
    }
    if let Some(align) = layout.alignment {
        columns = columns.alignment(align);
    }

    let term = Terminal::new();
    let output = columns.render(&term);

    emit_vertical_margins(layout, || {
        println!("{}", output);
        Ok(())
    })
}

/// Render a directory tree.
/// Options for rendering a directory tree.
#[derive(Debug, Clone, Default)]
pub struct DirOptions {
    pub show_size: bool,
    pub show_token: bool,
    pub show_modified: bool,
    pub show_updated: bool,
}

pub fn render_dir(
    path: &str,
    depth: Option<u32>,
    filter: &[String],
    skip_root: bool,
    layout: &LayoutArgs,
    options: &DirOptions,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::filesystem::FileSystem;
    use biscuit_terminal::components::renderable::Renderable;
    use biscuit_terminal::utils::layout::Margin;

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

    // Apply metrics
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

    // Apply layout properties to the FileSystem component
    if let Some(left) = layout.margin_left {
        fs = fs.left_margin(Margin::Chars(left));
    }
    if let Some(right) = layout.margin_right {
        fs = fs.right_margin(Margin::Chars(right));
    }
    if let Some(align) = layout.alignment {
        fs = fs.alignment(align);
    }

    fs.ensure_tree_built();

    let term = Terminal::new();
    let output = fs.render(&term);

    // Vertical margins on stderr so they don't pollute piped output
    let top = layout.margin_top.unwrap_or(1);
    for _ in 0..top {
        eprintln!();
    }

    // Print tree content, trimming any trailing blank/whitespace-only lines
    let output = output.trim_end();
    println!("{output}");

    let bottom = layout.margin_bottom.unwrap_or(0);
    for _ in 0..bottom {
        eprintln!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_column_width() {
        assert!(matches!(
            parse_column_width("50").unwrap(),
            ColumnWidth::Fixed(50)
        ));
        assert!(matches!(
            parse_column_width("80ch").unwrap(),
            ColumnWidth::Fixed(80)
        ));

        let width = parse_column_width("40%").unwrap();
        if let ColumnWidth::Percent(p) = width {
            assert!((p - 0.4).abs() < f32::EPSILON);
        } else {
            panic!("Expected Percent");
        }

        assert!(parse_column_width("invalid").is_err());
        assert!(parse_column_width("150%").is_err());
    }

    #[test]
    fn test_extract_color() {
        assert_eq!(
            extract_color("Label: 10 #ff0000"),
            ("Label: 10", Some("#ff0000".to_string()))
        );
        assert_eq!(
            extract_color("Label: 10 color: #00ff00"),
            ("Label: 10", Some("#00ff00".to_string()))
        );
        assert_eq!(extract_color("Label: 10"), ("Label: 10", None));
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#fff"), Some("#fff".to_string()));
        assert_eq!(parse_hex_color("#ff0000"), Some("#ff0000".to_string()));
        assert_eq!(parse_hex_color("ff0000"), None);
        assert_eq!(parse_hex_color("invalid"), None);
    }

    #[test]
    fn test_parse_single_pie_entry() {
        let entry = parse_single_pie_entry("Dogs: 386").unwrap();
        assert_eq!(entry.line, "\"Dogs\" : 386");
        assert_eq!(entry.color, None);

        let entry = parse_single_pie_entry("Cats: 85 #00ff00").unwrap();
        assert_eq!(entry.line, "\"Cats\" : 85");
        assert_eq!(entry.color, Some("#00ff00".to_string()));

        let entry = parse_single_pie_entry("\"Some Label\" : 10.5").unwrap();
        assert_eq!(entry.line, "\"Some Label\" : 10.5");
        assert_eq!(entry.color, None);
    }
}
