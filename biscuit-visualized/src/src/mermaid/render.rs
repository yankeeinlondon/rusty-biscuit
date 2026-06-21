use crate::artifact::{OutputFormat, RenderRequest, RenderedArtifact};
use crate::cache::file_cache::MERMAID_BACKEND;
use crate::cache::{FileCache, VisualizationKind};
use crate::raster::{rasterize_svg_to_png, rasterize_svg_to_png_bytes};

use super::config::{MermaidConfig, MermaidTheme};
use super::error::MermaidError;

/// A Mermaid diagram for rendering.
///
/// This struct handles rendering Mermaid diagrams to SVG or PNG format with
/// support for caching and customization.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::artifact::RenderRequest;
/// use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme};
///
/// // Basic usage with default settings
/// let diagram = MermaidDiagram::new("flowchart LR\n    A --> B")
///     .with_theme(MermaidTheme::Forest)
///     .with_title("My Flowchart");
///
/// // Render to PNG (default)
/// let artifact = diagram.render(&RenderRequest::default()).unwrap();
/// println!("Rendered to: {:?}", artifact.path);
///
/// // Get fallback code block for terminals without image support
/// println!("{}", diagram.fallback_code_block());
/// ```
///
/// ## Supported Diagram Types
///
/// - Flowcharts (`flowchart` / `graph`)
/// - Sequence Diagrams (`sequenceDiagram`)
/// - Class Diagrams (`classDiagram`)
/// - State Diagrams (`stateDiagram-v2`)
/// - ER Diagrams (`erDiagram`)
/// - Pie Charts (`pie`)
/// - XY Charts (`xychart`)
/// - Quadrant Charts (`quadrantChart`)
/// - Gantt (`gantt`)
/// - Timeline (`timeline`)
/// - Journey (`journey`)
/// - Mindmap (`mindmap`)
/// - Git Graph (`gitGraph`)
#[derive(Debug, Clone)]
pub struct MermaidDiagram {
    instructions: String,
    title: Option<String>,
    theme: MermaidTheme,
    config: MermaidConfig,
}

impl MermaidDiagram {
    /// Creates a new Mermaid diagram with default settings.
    ///
    /// ## Arguments
    ///
    /// * `instructions` - The Mermaid diagram syntax
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::mermaid::MermaidDiagram;
    ///
    /// let diagram = MermaidDiagram::new("graph LR; A-->B");
    /// ```
    pub fn new(instructions: impl Into<String>) -> Self {
        Self {
            instructions: instructions.into(),
            title: None,
            theme: MermaidTheme::default(),
            config: MermaidConfig::default(),
        }
    }

    /// Sets the theme for the diagram.
    pub fn with_theme(mut self, theme: MermaidTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the title for the diagram.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the configuration for the diagram.
    pub fn with_config(mut self, config: MermaidConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns the diagram instructions.
    pub fn instructions(&self) -> &str {
        &self.instructions
    }

    /// Renders the diagram to SVG or PNG format.
    ///
    /// This method:
    /// 1. Checks the cache for a previously rendered artifact
    /// 2. If not cached, renders the diagram using mermaid-rs-renderer
    /// 3. If PNG is requested, rasterizes the SVG
    /// 4. Stores the result in the cache
    /// 5. Returns a `RenderedArtifact` with the path to the rendered file
    ///
    /// ## Arguments
    ///
    /// * `request` - Rendering parameters (format, scale, etc.)
    ///
    /// ## Returns
    ///
    /// A `RenderedArtifact` containing the path to the rendered file and metadata.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The diagram syntax is invalid
    /// - SVG rendering fails
    /// - PNG rasterization fails (when PNG format is requested)
    /// - File I/O operations fail
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
    /// use biscuit_visualized::mermaid::MermaidDiagram;
    ///
    /// let diagram = MermaidDiagram::new("graph LR; A-->B");
    ///
    /// // Render to PNG with default settings
    /// let artifact = diagram.render(&RenderRequest::default())?;
    ///
    /// // Render to SVG
    /// let mut request = RenderRequest::default();
    /// request.format = OutputFormat::Svg;
    /// let artifact = diagram.render(&request)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn render(&self, request: &RenderRequest) -> Result<RenderedArtifact, MermaidError> {
        let options_json = self.render_options_json(request);
        let cache_key = FileCache::cache_key(
            VisualizationKind::Mermaid,
            &self.instructions,
            &options_json,
            MERMAID_BACKEND,
            request.format,
        );

        let cache = FileCache::new();
        if let Some(path) = cache.get(VisualizationKind::Mermaid, &cache_key, request.format) {
            return Ok(RenderedArtifact {
                path,
                format: request.format,
                cache_hit: true,
                alt_text: self.alt_text(),
            });
        }

        let svg = self.render_svg(request)?;
        let path = match request.format {
            OutputFormat::Svg => cache.store(
                VisualizationKind::Mermaid,
                &cache_key,
                OutputFormat::Svg,
                svg.as_bytes(),
            )?,
            OutputFormat::Png => {
                let png_data = match request.target_width {
                    Some(width) => rasterize_svg_to_png(&svg, width)?,
                    None => rasterize_svg_to_png_bytes(&svg, request.scale)?,
                };
                cache.store(
                    VisualizationKind::Mermaid,
                    &cache_key,
                    OutputFormat::Png,
                    &png_data,
                )?
            }
        };

        Ok(RenderedArtifact {
            path,
            format: request.format,
            cache_hit: false,
            alt_text: self.alt_text(),
        })
    }

    fn render_options_json(&self, request: &RenderRequest) -> String {
        let config_value = self
            .config
            .to_json()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        serde_json::to_string(&serde_json::json!({
            "theme": self.theme.as_str(),
            "title": self.title,
            "config": config_value,
            "scale": request.scale.max(1),
            "target_width": request.target_width,
            "transparent_background": request.transparent_background,
        }))
        .unwrap_or_else(|_| "{}".to_string())
    }

    fn render_svg(&self, request: &RenderRequest) -> Result<String, MermaidError> {
        let parsed = mermaid_rs_renderer::parse_mermaid(&self.instructions)
            .map_err(|err| MermaidError::RenderFailed(err.to_string()))?;

        let mut theme = self.build_theme(request.transparent_background);

        // Apply %%{init: {'themeVariables': {'pie1': '#xxx', ...}}}%% overrides
        if let Some(ref init) = parsed.init_config {
            apply_init_theme_overrides(&mut theme, init);
        }

        let layout_config = mermaid_rs_renderer::LayoutConfig::default();
        let layout = mermaid_rs_renderer::compute_layout(&parsed.graph, &theme, &layout_config);
        let svg = mermaid_rs_renderer::render_svg(&layout, &theme, &layout_config);

        Ok(self.apply_svg_overrides(svg))
    }

    fn build_theme(&self, transparent_background: bool) -> mermaid_rs_renderer::Theme {
        let mut theme = match self.theme {
            MermaidTheme::Default => mermaid_rs_renderer::Theme::mermaid_default(),
            MermaidTheme::Dark => dark_theme(),
            MermaidTheme::Forest => forest_theme(),
            MermaidTheme::Neutral => neutral_theme(),
        };

        if let Some(size) = self.config.point_label_font_size {
            theme.font_size = size as f32;
        }

        // HIGHLIGHT git-graph nodes (e.g. the "+N" elision marker) take each
        // branch's *inverse* hue from `git_inv_colors`, which renders as an
        // unrelated color — a blue square on a yellow branch. Mirror the branch
        // palette so every node on a branch shares one color.
        theme.git_inv_colors = theme.git_colors.clone();

        if transparent_background {
            theme.background = "none".to_string();
            theme.edge_label_background = "none".to_string();
        }

        theme
    }

    fn apply_svg_overrides(&self, svg: String) -> String {
        let mut output = svg;

        if let Some(fill) = &self.config.quadrant1_fill {
            output = replace_quadrant_fill(&output, "#ECECFF", fill);
        }
        if let Some(fill) = &self.config.quadrant2_fill {
            output = replace_quadrant_fill(&output, "#f1f1ff", fill);
        }
        if let Some(fill) = &self.config.quadrant3_fill {
            output = replace_quadrant_fill(&output, "#f6f6ff", fill);
        }
        if let Some(fill) = &self.config.quadrant4_fill {
            output = replace_quadrant_fill(&output, "#fbfbff", fill);
        }
        if let Some(radius) = self.config.point_radius {
            output = replace_circle_radius(&output, radius);
        }

        // Fix legend text vertical alignment: mermaid-rs-renderer positions legend
        // text at the vertical center of the color rect but uses the default
        // alphabetic baseline, causing misalignment. Add dominant-baseline="central"
        // to legend text elements (those following legend rect markers).
        output = fix_legend_text_baseline(&output);

        // Fix pie chart text contrast: mermaid-rs-renderer uses a single text color
        // for all slice labels. When a slice is light (e.g. white), light text is
        // unreadable. Adjust each label's fill based on its slice's luminance.
        output = fix_pie_text_contrast(&output);

        output
    }

    fn alt_text(&self) -> String {
        self.title
            .clone()
            .unwrap_or_else(|| "Mermaid diagram".to_string())
    }

    /// Returns a fallback code block representation of the diagram.
    ///
    /// This is useful for environments that don't support image rendering,
    /// such as text-only terminals or markdown viewers.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::mermaid::MermaidDiagram;
    ///
    /// let diagram = MermaidDiagram::new("graph LR; A-->B");
    /// let code_block = diagram.fallback_code_block();
    /// assert!(code_block.starts_with("```mermaid"));
    /// assert!(code_block.ends_with("```"));
    /// ```
    pub fn fallback_code_block(&self) -> String {
        format!("```mermaid\n{}\n```", self.instructions)
    }
}

/// Fixes vertical alignment of legend text relative to color marker rectangles.
///
/// The mermaid-rs-renderer positions legend text at the vertical center of the
/// marker rect (`rect_y + marker_size / 2`) but renders with SVG's default
/// alphabetic baseline. This makes text appear too high relative to the marker.
/// Adding `dominant-baseline="central"` centers the text vertically.
///
/// Targets `<text>` elements that immediately follow legend-style `<rect>` elements
/// (identified by having equal `fill` and `stroke` attributes with small dimensions).
fn fix_legend_text_baseline(svg: &str) -> String {
    let mut output = String::with_capacity(svg.len() + 200);
    let mut remaining = svg;

    // Legend rects have matching fill/stroke and are followed by <text> elements.
    // Pattern: <rect ... fill="X" stroke="X" .../><text ...>
    while let Some(rect_start) = remaining.find("<rect ") {
        let (before, from_rect) = remaining.split_at(rect_start);
        output.push_str(before);

        // Find end of rect tag
        let Some(rect_end) = from_rect.find("/>") else {
            output.push_str(from_rect);
            remaining = "";
            break;
        };

        let rect_tag = &from_rect[..rect_end + 2];
        let after_rect = &from_rect[rect_end + 2..];
        output.push_str(rect_tag);

        // Check if this is a legend marker rect (matching fill and stroke)
        if is_legend_rect(rect_tag) {
            // Look for an immediately following <text> element
            let trimmed = after_rect.trim_start();
            if trimmed.starts_with("<text ") {
                let skipped = after_rect.len() - trimmed.len();
                output.push_str(&after_rect[..skipped]);

                // Find the closing > of the <text ...> opening tag
                if let Some(gt_pos) = trimmed.find('>') {
                    let text_open = &trimmed[..gt_pos];
                    if !text_open.contains("dominant-baseline") {
                        output.push_str(text_open);
                        output.push_str(" dominant-baseline=\"central\">");
                        remaining = &trimmed[gt_pos + 1..];
                        continue;
                    }
                }
                remaining = &after_rect[skipped..];
                continue;
            }
        }

        remaining = after_rect;
    }

    output.push_str(remaining);
    output
}

/// Checks if a `<rect>` tag looks like a legend color marker.
///
/// Legend markers have matching `fill` and `stroke` attributes.
fn is_legend_rect(rect_tag: &str) -> bool {
    let fill = extract_attr(rect_tag, "fill");
    let stroke = extract_attr(rect_tag, "stroke");
    match (fill, stroke) {
        (Some(f), Some(s)) => f == s,
        _ => false,
    }
}

/// Extracts the value of an attribute from an XML tag string.
fn extract_attr<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    let pattern = format!("{}=\"", attr);
    let start = tag.find(&pattern)? + pattern.len();
    let end = start + tag[start..].find('"')?;
    Some(&tag[start..end])
}

/// Applies `%%{init: {'themeVariables': {...}}}%%` overrides to the theme.
///
/// This handles pie slice colors (`pie1`..`pie12`) and other theme variables
/// that `mermaid-rs-renderer`'s parser extracts but only the CLI applies.
fn apply_init_theme_overrides(theme: &mut mermaid_rs_renderer::Theme, init: &serde_json::Value) {
    if let Some(vars) = init.get("themeVariables") {
        // Apply pie slice colors (pie1 through pie12)
        for i in 0..12 {
            let key = format!("pie{}", i + 1);
            if let Some(color) = vars.get(&key).and_then(|v| v.as_str()) {
                theme.pie_colors[i] = color.to_string();
            }
        }

        // Apply other commonly used theme variables
        if let Some(val) = vars.get("primaryColor").and_then(|v| v.as_str()) {
            theme.primary_color = val.to_string();
        }
        if let Some(val) = vars.get("primaryTextColor").and_then(|v| v.as_str()) {
            theme.primary_text_color = val.to_string();
        }
        if let Some(val) = vars.get("primaryBorderColor").and_then(|v| v.as_str()) {
            theme.primary_border_color = val.to_string();
        }
        if let Some(val) = vars.get("lineColor").and_then(|v| v.as_str()) {
            theme.line_color = val.to_string();
        }
        if let Some(val) = vars.get("secondaryColor").and_then(|v| v.as_str()) {
            theme.secondary_color = val.to_string();
        }
        if let Some(val) = vars.get("tertiaryColor").and_then(|v| v.as_str()) {
            theme.tertiary_color = val.to_string();
        }
        if let Some(val) = vars.get("textColor").and_then(|v| v.as_str()) {
            theme.text_color = val.to_string();
        }
        if let Some(val) = vars.get("background").and_then(|v| v.as_str()) {
            theme.background = val.to_string();
        }
        if let Some(val) = vars.get("fontFamily").and_then(|v| v.as_str()) {
            theme.font_family = val.to_string();
        }
        if let Some(val) = vars.get("fontSize").and_then(|v| v.as_f64()) {
            theme.font_size = val as f32;
        }
        // Pie-specific text colors
        if let Some(val) = vars.get("pieTitleTextColor").and_then(|v| v.as_str()) {
            theme.pie_title_text_color = val.to_string();
        }
        if let Some(val) = vars.get("pieSectionTextColor").and_then(|v| v.as_str()) {
            theme.pie_section_text_color = val.to_string();
        }
        if let Some(val) = vars.get("pieLegendTextColor").and_then(|v| v.as_str()) {
            theme.pie_legend_text_color = val.to_string();
        }
        if let Some(val) = vars.get("pieStrokeColor").and_then(|v| v.as_str()) {
            theme.pie_stroke_color = val.to_string();
        }
        if let Some(val) = vars.get("pieOuterStrokeColor").and_then(|v| v.as_str()) {
            theme.pie_outer_stroke_color = val.to_string();
        }
    }
}

fn replace_quadrant_fill(svg: &str, current_fill: &str, replacement_fill: &str) -> String {
    svg.replacen(
        &format!("fill=\"{}\"", current_fill),
        &format!("fill=\"{}\"", replacement_fill),
        1,
    )
}

fn replace_circle_radius(svg: &str, radius: u32) -> String {
    let replacement = format!(" r=\"{}\"", radius.max(1));
    let mut output = String::with_capacity(svg.len());
    let mut remaining = svg;

    while let Some(circle_start) = remaining.find("<circle") {
        let (before_circle, circle_and_after) = remaining.split_at(circle_start);
        output.push_str(before_circle);

        if let Some(tag_end) = circle_and_after.find('>') {
            let (circle_tag, after_circle) = circle_and_after.split_at(tag_end + 1);
            output.push_str(&circle_tag.replacen(" r=\"5\"", &replacement, 1));
            remaining = after_circle;
        } else {
            output.push_str(circle_and_after);
            remaining = "";
        }
    }

    output.push_str(remaining);
    output
}

/// Fixes pie chart percentage label contrast by adjusting text color per slice.
///
/// The mermaid-rs-renderer uses a single `pie_section_text_color` for all slice
/// labels, which can produce unreadable text on light-colored slices. This
/// post-processes the SVG to set each label's fill based on its slice's relative
/// luminance (WCAG formula), using dark text on light slices.
fn fix_pie_text_contrast(svg: &str) -> String {
    // Collect pie slice fill colors from <path> elements (in order)
    let slice_fills: Vec<&str> = svg
        .match_indices("<path ")
        .filter_map(|(start, _)| {
            let rest = &svg[start..];
            let tag_end = rest.find('>')?;
            let tag = &rest[..tag_end];
            // Pie slice paths have opacity="0.850" — use this to distinguish
            // from other paths (arrows, borders)
            if !tag.contains("opacity=\"0.850\"") {
                return None;
            }
            extract_attr(tag, "fill")
        })
        .collect();

    if slice_fills.is_empty() {
        return svg.to_string();
    }

    // Collect percentage text elements: <text> with dominant-baseline="middle"
    // (as opposed to legend text which uses dominant-baseline="central")
    let text_positions: Vec<usize> = svg
        .match_indices("<text ")
        .filter_map(|(start, _)| {
            let rest = &svg[start..];
            let tag_end = rest.find('>')?;
            let tag = &rest[..tag_end];
            if tag.contains("dominant-baseline=\"middle\"") {
                Some(start)
            } else {
                None
            }
        })
        .collect();

    // Must have matching counts to pair them
    if text_positions.len() != slice_fills.len() {
        return svg.to_string();
    }

    let mut output = svg.to_string();
    // Process in reverse order so earlier positions stay valid
    for (i, &pos) in text_positions.iter().enumerate().rev() {
        let rest = &output[pos..];
        let Some(tag_end) = rest.find('>') else {
            continue;
        };
        let tag = &rest[..tag_end];

        let desired_fill = if is_light_color(slice_fills[i]) {
            "#1a1a1a"
        } else {
            "#e2e8f0"
        };

        // Replace the fill attribute in this text tag
        if let Some(fill_start) = tag.find("fill=\"") {
            let abs_fill_start = pos + fill_start + 6; // after fill="
            let fill_rest = &output[abs_fill_start..];
            if let Some(fill_end) = fill_rest.find('"') {
                output.replace_range(abs_fill_start..abs_fill_start + fill_end, desired_fill);
            }
        }
    }

    output
}

/// Returns true if a hex color is perceptually light (relative luminance > 0.5).
fn is_light_color(hex: &str) -> bool {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return false;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;

    // sRGB to linear
    let to_linear = |c: f64| -> f64 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    let luminance = 0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b);
    luminance > 0.4
}

fn dark_theme() -> mermaid_rs_renderer::Theme {
    let mut theme = mermaid_rs_renderer::Theme::modern();
    theme.primary_color = "#1e293b".to_string();
    theme.primary_text_color = "#e2e8f0".to_string();
    theme.primary_border_color = "#64748b".to_string();
    theme.line_color = "#94a3b8".to_string();
    theme.secondary_color = "#334155".to_string();
    theme.tertiary_color = "#0f172a".to_string();
    theme.edge_label_background = "rgba(15,23,42,0.92)".to_string();
    theme.cluster_background = "#111827".to_string();
    theme.cluster_border = "#475569".to_string();
    theme.background = "#020617".to_string();
    theme.sequence_actor_fill = "#0f172a".to_string();
    theme.sequence_actor_border = "#64748b".to_string();
    theme.sequence_actor_line = "#94a3b8".to_string();
    theme.sequence_note_fill = "#422006".to_string();
    theme.sequence_note_border = "#f59e0b".to_string();
    theme.sequence_activation_fill = "#1e293b".to_string();
    theme.sequence_activation_border = "#64748b".to_string();
    theme.text_color = "#e2e8f0".to_string();
    theme.pie_title_text_color = "#e2e8f0".to_string();
    theme.pie_section_text_color = "#e2e8f0".to_string();
    theme.pie_legend_text_color = "#e2e8f0".to_string();
    theme.pie_stroke_color = "#cbd5e1".to_string();
    theme.pie_outer_stroke_color = "#64748b".to_string();
    theme
}

fn forest_theme() -> mermaid_rs_renderer::Theme {
    let mut theme = mermaid_rs_renderer::Theme::mermaid_default();
    theme.primary_color = "#dcfce7".to_string();
    theme.primary_text_color = "#14532d".to_string();
    theme.primary_border_color = "#16a34a".to_string();
    theme.line_color = "#166534".to_string();
    theme.secondary_color = "#bbf7d0".to_string();
    theme.tertiary_color = "#ecfdf5".to_string();
    theme.edge_label_background = "#f7fee7".to_string();
    theme.cluster_background = "#d1fae5".to_string();
    theme.cluster_border = "#34d399".to_string();
    theme.background = "#f0fdf4".to_string();
    theme.sequence_actor_fill = "#dcfce7".to_string();
    theme.sequence_actor_border = "#16a34a".to_string();
    theme.sequence_actor_line = "#166534".to_string();
    theme.sequence_note_fill = "#fef3c7".to_string();
    theme.sequence_note_border = "#d97706".to_string();
    theme.sequence_activation_fill = "#bbf7d0".to_string();
    theme.sequence_activation_border = "#16a34a".to_string();
    theme.text_color = "#14532d".to_string();
    theme
}

fn neutral_theme() -> mermaid_rs_renderer::Theme {
    let mut theme = mermaid_rs_renderer::Theme::modern();
    theme.primary_color = "#f5f5f5".to_string();
    theme.primary_text_color = "#262626".to_string();
    theme.primary_border_color = "#737373".to_string();
    theme.line_color = "#525252".to_string();
    theme.secondary_color = "#e5e5e5".to_string();
    theme.tertiary_color = "#ffffff".to_string();
    theme.edge_label_background = "#fafafa".to_string();
    theme.cluster_background = "#ededed".to_string();
    theme.cluster_border = "#a3a3a3".to_string();
    theme.background = "#fafafa".to_string();
    theme.sequence_actor_fill = "#f5f5f5".to_string();
    theme.sequence_actor_border = "#737373".to_string();
    theme.sequence_actor_line = "#525252".to_string();
    theme.sequence_note_fill = "#f5f5f4".to_string();
    theme.sequence_note_border = "#a8a29e".to_string();
    theme.sequence_activation_fill = "#e7e5e4".to_string();
    theme.sequence_activation_border = "#737373".to_string();
    theme.text_color = "#171717".to_string();
    theme
}
