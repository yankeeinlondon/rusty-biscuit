use crate::args::LayoutArgs;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::terminal_image::parse_width_spec;
use biscuit_terminal::components::two_column::ColumnWidth;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;

/// Returns the terminal to use for rendering CLI output.
///
/// When `plain` is true, detection still runs but color depth is forced to
/// [`ColorDepth::None`], overriding `FORCE_COLOR` / `CLICOLOR_FORCE`.
/// When `plain` is false, the conventional color-forcing env vars are honored
/// and `NO_COLOR` is left to the normal detection path.
pub fn terminal_for_render(plain: bool) -> Terminal {
    if plain {
        let detected = Terminal::new();
        Terminal {
            color_depth: ColorDepth::None,
            ..detected
        }
    } else {
        detect_terminal_honoring_force_color()
    }
}

/// Returns whether the terminal is in dark mode.
pub fn is_dark_mode() -> bool {
    use biscuit_terminal::discovery::detection::ColorMode;
    let term = Terminal::new();
    matches!(term.color_mode(), ColorMode::Dark | ColorMode::Unknown)
}

/// Constructs a [`Terminal`] honoring the conventional color-forcing
/// env vars (`FORCE_COLOR=1`, `CLICOLOR_FORCE=1`).
///
/// When either is set in the environment, this returns
/// [`Terminal::new_forced`] which sets `color_depth = TrueColor`,
/// `is_tty = true`, `osc_link_support = true`, and `supports_italic
/// = true` while leaving detection-driven fields (`app`, `os`,
/// `image_support`, etc.) intact.
///
/// Otherwise, defers to [`Terminal::new`].
pub fn detect_terminal_honoring_force_color() -> Terminal {
    if std::env::var_os("FORCE_COLOR").is_some() || std::env::var_os("CLICOLOR_FORCE").is_some() {
        Terminal::new_forced()
    } else {
        Terminal::new()
    }
}

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
pub fn apply_image_layout(
    term_image: &mut biscuit_terminal::components::terminal_image::TerminalImage,
    layout: &LayoutArgs,
) {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::utils::layout::{Length, TargetValue};

    if let Some(ml) = layout.margin_left {
        term_image.layout_mut().margin.left = TargetValue::universal(Length::ch(ml));
    }
    if let Some(mr) = layout.margin_right {
        term_image.layout_mut().margin.right = TargetValue::universal(Length::ch(mr));
    }
    if let Some(align) = layout.alignment {
        term_image.layout_mut().alignment = align;
    }
}

/// Apply optional margin and alignment overrides to any `TerminalRenderable` component.
pub fn apply_renderable_layout(
    component: &mut impl biscuit_terminal::components::renderable::TerminalRenderable,
    layout: &LayoutArgs,
) {
    use biscuit_terminal::utils::layout::{Length, TargetValue};

    if let Some(ml) = layout.margin_left {
        component.layout_mut().margin.left = TargetValue::universal(Length::ch(ml));
    }
    if let Some(mr) = layout.margin_right {
        component.layout_mut().margin.right = TargetValue::universal(Length::ch(mr));
    }
    if let Some(align) = layout.alignment {
        component.layout_mut().alignment = align;
    }
}

pub fn parse_column_width(spec: &str) -> color_eyre::Result<ColumnWidth> {
    use biscuit_terminal::components::terminal_image::ImageWidth;
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

/// Output render metadata to stderr as JSON.
pub fn output_render_meta(render_meta: &crate::output::RenderMeta) -> color_eyre::Result<()> {
    eprintln!("{}", serde_json::to_string(render_meta)?);
    Ok(())
}

/// Brief pause after image rendering.
///
/// This is a minimal delay to ensure the terminal has finished processing
/// image data before we print any following text.
pub fn settle_terminal() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    // Small delay for terminal processing
    std::thread::sleep(std::time::Duration::from_millis(10));
}

/// Emit rendered image output and flush immediately.
pub fn emit_image_output(output: &str) -> color_eyre::Result<()> {
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
/// Renders the header and command through [`Prose`] so the output degrades
/// cleanly when color is disabled while preserving the legacy `Command:` label.
pub fn print_example_command_with_terminal(cmd: &str, term: &Terminal) {
    println!();
    println!("{}", Prose::new("<b>Command:</b>").render(term));
    println!(
        "{}",
        Prose::new(format!("<dim>{}</dim>", Prose::escape_text(cmd))).render(term)
    );
}

pub fn print_example_command(cmd: &str) {
    let plain = std::env::args_os().any(|arg| arg == "--plain");
    let term = terminal_for_render(plain);
    print_example_command_with_terminal(cmd, &term);
}

/// Emits a YAML frontmatter block carrying the CLI's `--margin-left` /
/// `--margin-right` values prepended to a Markdown body.
///
/// Returns the body unchanged when no horizontal margins were given on the
/// CLI. Vertical margins (`--margin-top` / `--margin-bottom`) and
/// `--alignment` are intentionally not lowered to Markdown frontmatter —
/// CommonMark has no portable peer for them and the bt CLI emits them as
/// blank lines / through HTML wrappers respectively.
pub fn render_markdown_with_layout_frontmatter(body: &str, layout: &LayoutArgs) -> String {
    let Some(frontmatter) = layout_style_frontmatter(layout) else {
        return body.to_string();
    };
    format!("---\n{frontmatter}---\n\n{body}")
}

/// Builds the YAML body of the `style:` frontmatter block for the given
/// layout flags. Returns `None` when neither `--margin-left` nor
/// `--margin-right` is set, which signals callers to omit the frontmatter
/// envelope entirely.
pub fn layout_style_frontmatter(layout: &LayoutArgs) -> Option<String> {
    if layout.margin_left.is_none() && layout.margin_right.is_none() {
        return None;
    }
    let mut out = String::from("style:\n  page:\n");
    if let Some(left) = layout.margin_left {
        out.push_str(&format!("    margin-left: {left}ch\n"));
    }
    if let Some(right) = layout.margin_right {
        out.push_str(&format!("    margin-right: {right}ch\n"));
    }
    Some(out)
}

/// Strips SGR (Select Graphic Rendition) CSI sequences from `s`.
///
/// This preserves non-color ANSI sequences such as OSC8 hyperlinks
/// and cursor positioning, removing only `ESC [ … m` style codes.
pub fn strip_sgr_sequences(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            let start = i;
            i += 2;
            while i < bytes.len() {
                let b = bytes[i];
                i += 1;
                if b == b'm' {
                    // SGR sequence — discard everything from start to here.
                    break;
                }
                if !(b.is_ascii_digit() || b == b';' || b == b':') {
                    // Not an SGR sequence — keep the bytes we skipped.
                    out.extend_from_slice(&bytes[start..i]);
                    break;
                }
            }
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
