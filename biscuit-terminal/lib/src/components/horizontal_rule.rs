use std::collections::HashMap;

use crate::components::renderable::{BrowserRenderable, Renderable};
use crate::terminal::Terminal;
use crate::utils::layout::{Layout, Margin};

/// Defines the visual style of a horizontal rule.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleStyle {
    /// Simple dashed line: ---
    Dashes,
    /// Dotted line: ···
    Dots,
    /// Wavy line using Unicode characters
    Waves,
    /// Line with star symbols: * * *
    LineStar,
    /// Line with circle symbols: ○ ○ ○
    LineCircle,
    /// Inset line with border effect
    InsetLine,
    /// Curtain rod style with decorative ends
    CurtainRod,
}

/// Defines the placement of a horizontal rule within the available width.
#[derive(Debug, Clone, PartialEq)]
pub enum RulePlacement {
    /// Span the full available width
    Full,
    /// Centered with equal margins on both sides
    Centered,
    /// Aligned to the left edge
    Left,
    /// Aligned to the right edge
    Right,
}

/// Defines the visual weight (thickness) of a horizontal rule.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleWeight {
    /// Thin line (1px equivalent)
    Thin,
    /// Medium line (2px equivalent)
    Medium,
    /// Thick line (3px equivalent)
    Thick,
}

/// A horizontal rule component for terminal and browser rendering.
#[derive(Debug, Clone)]
pub struct HorizontalRule {
    style: RuleStyle,
    placement: RulePlacement,
    weight: RuleWeight,
    width: Option<String>, // CSS-like width specification (e.g., "50%", "200px")
    color: Option<String>, // CSS color specification
    layout: Layout,
}

impl HorizontalRule {
    /// Creates a new horizontal rule with default settings.
    pub fn new() -> Self {
        Self {
            style: RuleStyle::Dashes,
            placement: RulePlacement::Full,
            weight: RuleWeight::Medium,
            width: None,
            color: None,
            layout: Layout::default(),
        }
    }

    /// Sets the visual style of the rule.
    pub fn style(mut self, style: RuleStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the placement of the rule.
    pub fn placement(mut self, placement: RulePlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the weight (thickness) of the rule.
    pub fn weight(mut self, weight: RuleWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Sets the width of the rule as a CSS-like string.
    pub fn width(mut self, width: impl Into<String>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the color of the rule as a CSS color string.
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl Default for HorizontalRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderable for HorizontalRule {
    fn render(&self, term: &Terminal) -> String {
        // Tier 1: SVG→PNG via resvg with TerminalImage
        // If terminal supports images, we could generate an SVG and render it.
        // For now, we use Tier 2/3 which are more universally compatible.
        
        // Determine the width based on placement, custom width, and terminal width
        let term_width = term.width() as usize;
        let rule_width = self.resolve_width(term_width);
        
        // Clamp to reasonable minimum and maximum
        let rule_width = rule_width.clamp(10, term_width);
        
        // Generate the rule content based on style and terminal capabilities
        let rule_content = self.generate_terminal_content(rule_width, term);
        
        // Apply placement (using character count, not byte length)
        let content_width = rule_content.chars().count();
        match self.placement {
            RulePlacement::Full => rule_content,
            RulePlacement::Centered => {
                let padding = (term_width.saturating_sub(content_width)) / 2;
                format!("{}{}", " ".repeat(padding), rule_content)
            }
            RulePlacement::Left => rule_content,
            RulePlacement::Right => {
                let padding = term_width.saturating_sub(content_width);
                format!("{}{}", " ".repeat(padding), rule_content)
            }
        }
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn is_block_level(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HorizontalRule {
    /// Resolves the width of the rule in characters based on the terminal width
    /// and the CSS-like width specification.
    fn resolve_width(&self, term_width: usize) -> usize {
        match &self.width {
            Some(width_str) => {
                let trimmed = width_str.trim();
                // Handle percentage: "50%"
                if let Some(pct_str) = trimmed.strip_suffix('%')
                    && let Ok(pct) = pct_str.trim().parse::<f32>()
                {
                    return ((term_width as f32 * pct / 100.0) as usize).max(1);
                }
                // Handle character width: "20ch" or "20"
                let num_str = if let Some(s) = trimmed.strip_suffix("ch") {
                    s.trim()
                } else {
                    trimmed
                };
                if let Ok(chars) = num_str.parse::<usize>() {
                    return chars.max(1);
                }
                // Fallback for unparseable width
                term_width
            }
            None => {
                // Default width based on placement
                match self.placement {
                    RulePlacement::Full => term_width,
                    RulePlacement::Centered | RulePlacement::Left | RulePlacement::Right => {
                        (term_width as f32 * 0.8) as usize
                }
            }
        }
    }
    }

    /// Generates terminal content for the horizontal rule based on style and width.
    ///
    /// Uses a 3-tier progressive enhancement:
    /// - Tier 2: Unicode characters for modern terminals
    /// - Tier 3: ASCII characters for restricted environments
    fn generate_terminal_content(&self, width: usize, term: &Terminal) -> String {
        let supports_unicode = self.supports_unicode(term);
        
        match &self.style {
            RuleStyle::Dashes => {
                if supports_unicode {
                    "╌".repeat(width)
                } else {
                    "-".repeat(width)
                }
            }
            RuleStyle::Dots => {
                if supports_unicode {
                    "·".repeat(width)
                } else {
                    ".".repeat(width)
                }
            }
            RuleStyle::Waves => {
                if supports_unicode {
                    "≋".repeat(width)
                } else {
                    "~".repeat(width)
                }
            }
            RuleStyle::LineStar => {
                if supports_unicode {
                    // Pattern: ────★────
                    let line_char = '─';
                    let star = '★';
                    Self::centered_symbol_pattern(width, line_char, star)
                } else {
                    // Pattern: ---[*]---
                    let line_char = '-';
                    let star = '*';
                    Self::centered_symbol_pattern(width, line_char, star)
                }
            }
            RuleStyle::LineCircle => {
                if supports_unicode {
                    // Pattern: ────●────
                    let line_char = '─';
                    let circle = '●';
                    Self::centered_symbol_pattern(width, line_char, circle)
                } else {
                    // Pattern: ---(o)---
                    let line_char = '-';
                    let circle = 'o';
                    Self::centered_symbol_pattern(width, line_char, circle)
                }
            }
            RuleStyle::InsetLine => {
                if width < 4 {
                    "-".repeat(width)
                } else {
                    let inner_width = width - 4;
                    let line = if supports_unicode { "─" } else { "-" };
                    format!("  {}{}  ", line.repeat(inner_width / 2), line.repeat(inner_width - inner_width / 2))
                }
            }
            RuleStyle::CurtainRod => {
                if width < 5 {
                    if supports_unicode {
                        "═".repeat(width)
                    } else {
                        "=".repeat(width)
                    }
                } else {
                    let inner_width = width.saturating_sub(4);
                    let line_char = if supports_unicode { '─' } else { '-' };
                    let left_bracket = if supports_unicode { '「' } else { '[' };
                    let right_bracket = if supports_unicode { '」' } else { ']' };
                    format!("{}{}{}{}", left_bracket, line_char.to_string().repeat(inner_width / 2), line_char.to_string().repeat(inner_width - inner_width / 2), right_bracket)
                }
            }
        }
    }

    /// Creates a centered symbol pattern like ────★────
    fn centered_symbol_pattern(width: usize, line_char: char, symbol: char) -> String {
        if width < 3 {
            return line_char.to_string().repeat(width);
        }
        let symbol_width = 1;
        let remaining = width.saturating_sub(symbol_width);
        let left_pad = remaining / 2;
        let right_pad = remaining - left_pad;
        format!("{}{}{}", line_char.to_string().repeat(left_pad), symbol, line_char.to_string().repeat(right_pad))
    }
    
    /// Checks if the terminal supports Unicode characters.
    fn supports_unicode(&self, term: &Terminal) -> bool {
        // Check terminal capabilities
        // For now, assume Unicode is supported unless terminal explicitly doesn't support it
        term.color_depth != crate::discovery::detection::ColorDepth::None
    }
}

impl BrowserRenderable for HorizontalRule {
    fn render_to_browser(&self) -> String {
        // Generate style-specific SVG
        let stroke_width = match self.weight {
            RuleWeight::Thin => "2",
            RuleWeight::Medium => "4",
            RuleWeight::Thick => "8",
        };
        
        let width_attr = self.width.as_deref().unwrap_or("100%");
        let color_attr = self.color.as_deref().unwrap_or("currentColor");
        let margin_top = self.layout.top_margin.to_css_value("0");
        let margin_bottom = self.layout.bottom_margin.to_css_value("0");
        
        let svg_content = match &self.style {
            RuleStyle::Dashes => {
                format!(
                    r#"<line x1="0" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-dasharray="8,4"/>"#,
                    color_attr, stroke_width
                )
            }
            RuleStyle::Dots => {
                format!(
                    r#"<line x1="0" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-dasharray="2,6"/>"#,
                    color_attr, stroke_width
                )
            }
            RuleStyle::Waves => {
                format!(
                    r#"<path d="M0 20 Q 10 10 20 20 T 40 20 T 60 20 T 80 20 T 100 20 T 120 20 T 140 20 T 160 20 T 180 20 T 200 20" stroke="{}" stroke-width="{}" fill="none" stroke-linecap="round"/>"#,
                    color_attr, stroke_width
                )
            }
            RuleStyle::LineStar => {
                format!(
                    r#"<line x1="0" y1="50%" x2="45%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <path d="M50% 35% L52% 45% L62% 45% L54% 52% L57% 62% L50% 55% L43% 62% L46% 52% L38% 45% L48% 45% Z" fill="{}"/>
  <line x1="55%" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
                    color_attr, stroke_width, color_attr, color_attr, stroke_width
                )
            }
            RuleStyle::LineCircle => {
                format!(
                    r#"<line x1="0" y1="50%" x2="45%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <circle cx="50%" cy="50%" r="8" fill="none" stroke="{}" stroke-width="{}"/>
  <line x1="55%" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
                    color_attr, stroke_width, color_attr, stroke_width, color_attr, stroke_width
                )
            }
            RuleStyle::InsetLine => {
                format!(
                    r#"<line x1="10%" y1="50%" x2="90%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>"#,
                    color_attr, stroke_width
                )
            }
            RuleStyle::CurtainRod => {
                format!(
                    r#"<line x1="5%" y1="50%" x2="95%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
  <circle cx="5%" cy="50%" r="4" fill="{}"/>
  <circle cx="95%" cy="50%" r="4" fill="{}"/>"#,
                    color_attr, stroke_width, color_attr, color_attr
                )
            }
        };
        
        format!(
            r#"<svg width="{}" height="40" xmlns="http://www.w3.org/2000/svg" style="display: block; margin: {} auto {} auto;">
  {}
</svg>"#,
            width_attr,
            margin_top,
            margin_bottom,
            svg_content
        )
    }

    fn render_to_browser_with_inline_variables(&self, variables: &HashMap<String, String>) -> String {
        // Apply CSS variables if provided
        let mut svg = self.render_to_browser();
        
        // Replace any placeholders with actual variables
        for (key, value) in variables {
            svg = svg.replace(&format!("var(--{})", key), value);
        }
        
        svg
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Helper trait extension for Margin to convert to CSS values
trait MarginToCss {
    fn to_css_value(&self, default: &str) -> String;
}

impl MarginToCss for Margin {
    fn to_css_value(&self, _default: &str) -> String {
        match self {
            Margin::Chars(chars) => format!("{}ch", chars),
            Margin::Percent(pct) => format!("{}%", pct),
            Margin::None => "0".to_string(),
            Margin::Offset(base, chars) => {
                // For Offset, we'll use the base margin plus the chars
                let base_value = base.to_css_value("0");
                if base_value == "0" {
                    format!("{}ch", chars)
                } else {
                    // This is a simplification - in a real implementation,
                    // we'd need to parse and combine the values properly
                    format!("{} + {}ch", base_value, chars)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::Terminal;
    use crate::utils::layout::Margin;
    use crate::discovery::detection::ColorDepth;
    use insta::assert_snapshot;
    
    #[test]
    fn test_horizontal_rule_new() {
        let hr = HorizontalRule::new();
        assert_eq!(hr.style, RuleStyle::Dashes);
        assert_eq!(hr.placement, RulePlacement::Full);
        assert_eq!(hr.weight, RuleWeight::Medium);
        assert_eq!(hr.width, None);
        assert_eq!(hr.color, None);
    }
    
    #[test]
    fn test_horizontal_rule_builder_methods() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .placement(RulePlacement::Centered)
            .weight(RuleWeight::Thick)
            .width("50%")
            .color("red");
            
        assert_eq!(hr.style, RuleStyle::Waves);
        assert_eq!(hr.placement, RulePlacement::Centered);
        assert_eq!(hr.weight, RuleWeight::Thick);
        assert_eq!(hr.width, Some("50%".to_string()));
        assert_eq!(hr.color, Some("red".to_string()));
    }
    
    #[test]
    fn test_horizontal_rule_default_impl() {
        let hr1 = HorizontalRule::new();
        let hr2 = HorizontalRule::default();
        assert_eq!(hr1.style, hr2.style);
        assert_eq!(hr1.placement, hr2.placement);
        assert_eq!(hr1.weight, hr2.weight);
        assert_eq!(hr1.width, hr2.width);
        assert_eq!(hr1.color, hr2.color);
    }
    
    #[test]
    fn test_render_dashes_full_unicode() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Unicode terminal should use box drawing char
        assert!(result.chars().all(|c| c == '╌' || c == '-'));
    }
    
    #[test]
    fn test_render_dashes_full_ascii() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Full);
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback should use hyphens
        assert!(result.chars().all(|c| c == '-'));
    }
    
    #[test]
    fn test_render_dots_full_unicode() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dots)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Unicode middle dot or ASCII period
        assert!(result.chars().all(|c| c == '·' || c == '.'));
    }
    
    #[test]
    fn test_render_dots_full_ascii() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dots)
            .placement(RulePlacement::Full);
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback
        assert!(result.chars().all(|c| c == '.'));
    }
    
    #[test]
    fn test_render_waves_full_unicode() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Unicode wave dash or ASCII tilde
        assert!(result.chars().all(|c| c == '≋' || c == '~'));
    }
    
    #[test]
    fn test_render_waves_full_ascii() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .placement(RulePlacement::Full);
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback
        assert!(result.chars().all(|c| c == '~'));
    }
    
    #[test]
    fn test_render_line_star_unicode() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineStar)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Should contain star symbol
        assert!(result.contains('★') || result.contains('*'));
    }
    
    #[test]
    fn test_render_line_star_ascii() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineStar)
            .placement(RulePlacement::Full);
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback
        assert!(result.contains('*'));
    }
    
    #[test]
    fn test_render_line_circle_unicode() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineCircle)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // Should contain circle symbol
        assert!(result.contains('●') || result.contains('o'));
    }
    
    #[test]
    fn test_render_line_circle_ascii() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineCircle)
            .placement(RulePlacement::Full);
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .build();
        let result = hr.render(&term);
        assert!(!result.is_empty());
        // ASCII fallback
        assert!(result.contains('o'));
    }
    
    #[test]
    fn test_render_inset_line_unicode() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::InsetLine)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() >= 3);
        // Unicode: indented with box drawing chars
        assert!(result.contains('─') || result.contains('-'));
    }
    
    #[test]
    fn test_render_inset_line_ascii() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::InsetLine)
            .placement(RulePlacement::Full);
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .build();
        let result = hr.render(&term);
        assert!(result.len() >= 3);
        // ASCII fallback: indented with hyphens
        assert!(result.contains('-'));
    }
    
    #[test]
    fn test_render_curtain_rod_unicode() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::CurtainRod)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() >= 5);
        // Unicode brackets
        assert!(result.contains('「') || result.contains('」') || result.contains('[') || result.contains(']'));
    }
    
    #[test]
    fn test_render_curtain_rod_ascii() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::CurtainRod)
            .placement(RulePlacement::Full);
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .build();
        let result = hr.render(&term);
        assert!(result.len() >= 5);
        // ASCII brackets
        assert!(result.contains('[') && result.contains(']'));
    }
    
    #[test]
    fn test_render_centered() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Centered);
        let term = Terminal::builder()
            .width(100)
            .build();
        let result = hr.render(&term);
        let term_width = 100_usize;
        let rule_width = (term_width as f32 * 0.8) as usize;
        let rule_content = "╌".repeat(rule_width);
        let expected_padding = (term_width - rule_width) / 2;
        assert!(result.starts_with(&" ".repeat(expected_padding)));
        assert!(result[expected_padding..].starts_with(&rule_content));
    }
    
    #[test]
    fn test_render_left() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Left);
        let term = Terminal::builder()
            .width(100)
            .build();
        let result = hr.render(&term);
        let rule_width = (100_f32 * 0.8) as usize;
        let rule_content = "╌".repeat(rule_width);
        assert!(result.starts_with(&rule_content));
    }
    
    #[test]
    fn test_render_right() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Right);
        let term = Terminal::builder()
            .width(100)
            .build();
        let result = hr.render(&term);
        let term_width = 100_usize;
        let rule_width = (term_width as f32 * 0.8) as usize;
        let rule_content = "╌".repeat(rule_width);
        let expected_padding = term_width - rule_width;
        assert!(result.starts_with(&" ".repeat(expected_padding)));
        assert!(result[expected_padding..].starts_with(&rule_content));
    }
    
    #[test]
    fn test_render_to_browser() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .weight(RuleWeight::Medium)
            .width("50%")
            .color("blue");
        let result = hr.render_to_browser();
        assert!(result.contains(r#"width="50%""#));
        assert!(result.contains(r#"stroke="blue""#));
        assert!(result.contains(r#"stroke-width="4""#));
    }
    
    #[test]
    fn test_render_to_browser_default() {
        let hr = HorizontalRule::new();
        let result = hr.render_to_browser();
        assert!(result.contains(r#"width="100%""#));
        assert!(result.contains(r#"stroke="currentColor""#));
        assert!(result.contains(r#"stroke-width="4""#));
    }
    
    #[test]
    fn test_render_to_browser_with_inline_variables() {
        let hr = HorizontalRule::new().width("var(--rule-width)");
        let mut variables = std::collections::HashMap::new();
        variables.insert("rule-width".to_string(), "75%".to_string());
        let result = hr.render_to_browser_with_inline_variables(&variables);
        assert!(result.contains(r#"width="75%""#));
    }
    
    #[test]
    fn test_layout_accessors() {
        let mut hr = HorizontalRule::new();
        let _layout = hr.layout();
        
        hr.layout_mut().top_margin = Margin::Chars(2);
        assert_eq!(hr.layout().top_margin, Margin::Chars(2));
    }
    
    #[test]
    fn test_as_any() {
        let hr = HorizontalRule::new();
        let any_ref = Renderable::as_any(&hr);
        let downcast_ref = any_ref.downcast_ref::<HorizontalRule>();
        assert!(downcast_ref.is_some());
        assert_eq!(downcast_ref.unwrap().style, RuleStyle::Dashes);
    }
    
    #[test]
    fn test_browser_renderable_as_any() {
        let hr = HorizontalRule::new();
        let any_ref = BrowserRenderable::as_any(&hr);
        let downcast_ref = any_ref.downcast_ref::<HorizontalRule>();
        assert!(downcast_ref.is_some());
        assert_eq!(downcast_ref.unwrap().style, RuleStyle::Dashes);
    }
    
    #[test]
    fn test_resolve_width_percentage() {
        let hr = HorizontalRule::new().width("50%");
        assert_eq!(hr.resolve_width(100), 50);
    }
    
    #[test]
    fn test_resolve_width_chars() {
        let hr = HorizontalRule::new().width("20ch");
        assert_eq!(hr.resolve_width(100), 20);
    }
    
    #[test]
    fn test_resolve_width_raw_number() {
        let hr = HorizontalRule::new().width("30");
        assert_eq!(hr.resolve_width(100), 30);
    }
    
    #[test]
    fn test_resolve_width_default_full() {
        let hr = HorizontalRule::new().placement(RulePlacement::Full);
        assert_eq!(hr.resolve_width(100), 100);
    }
    
    #[test]
    fn test_resolve_width_default_centered() {
        let hr = HorizontalRule::new().placement(RulePlacement::Centered);
        assert_eq!(hr.resolve_width(100), 80);
    }
    
    #[test]
    fn test_all_styles_all_placements_all_weights_unicode() {
        let term = Terminal::default();
        let styles = vec![
            RuleStyle::Dashes,
            RuleStyle::Dots,
            RuleStyle::Waves,
            RuleStyle::LineStar,
            RuleStyle::LineCircle,
            RuleStyle::InsetLine,
            RuleStyle::CurtainRod,
        ];
        let placements = vec![
            RulePlacement::Full,
            RulePlacement::Centered,
            RulePlacement::Left,
            RulePlacement::Right,
        ];
        let weights = vec![
            RuleWeight::Thin,
            RuleWeight::Medium,
            RuleWeight::Thick,
        ];
        
        for style in &styles {
            for placement in &placements {
                for weight in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .placement(placement.clone())
                        .weight(weight.clone());
                    let result = hr.render(&term);
                    assert!(!result.is_empty(), "Failed for style={:?} placement={:?} weight={:?}", style, placement, weight);
                }
            }
        }
    }
    
    #[test]
    fn test_all_styles_all_placements_all_weights_ascii() {
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .build();
        let styles = vec![
            RuleStyle::Dashes,
            RuleStyle::Dots,
            RuleStyle::Waves,
            RuleStyle::LineStar,
            RuleStyle::LineCircle,
            RuleStyle::InsetLine,
            RuleStyle::CurtainRod,
        ];
        let placements = vec![
            RulePlacement::Full,
            RulePlacement::Centered,
            RulePlacement::Left,
            RulePlacement::Right,
        ];
        let weights = vec![
            RuleWeight::Thin,
            RuleWeight::Medium,
            RuleWeight::Thick,
        ];
        
        for style in &styles {
            for placement in &placements {
                for weight in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .placement(placement.clone())
                        .weight(weight.clone());
                    let result = hr.render(&term);
                    assert!(!result.is_empty(), "Failed for style={:?} placement={:?} weight={:?}", style, placement, weight);
                }
            }
        }
    }

    #[test]
    fn test_snapshot_render_all_styles() {
        let term = Terminal::default();

        let styles = vec![
            ("dashes", RuleStyle::Dashes),
            ("dots", RuleStyle::Dots),
            ("waves", RuleStyle::Waves),
            ("line_star", RuleStyle::LineStar),
            ("line_circle", RuleStyle::LineCircle),
            ("inset_line", RuleStyle::InsetLine),
            ("curtain_rod", RuleStyle::CurtainRod),
        ];

        let placements = vec![
            ("full", RulePlacement::Full),
            ("centered", RulePlacement::Centered),
            ("left", RulePlacement::Left),
            ("right", RulePlacement::Right),
        ];

        let weights = vec![
            ("thin", RuleWeight::Thin),
            ("medium", RuleWeight::Medium),
            ("thick", RuleWeight::Thick),
        ];

        for (style_name, style) in &styles {
            for (placement_name, placement) in &placements {
                for (weight_name, weight) in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .placement(placement.clone())
                        .weight(weight.clone());

                    let result = hr.render(&term);
                    assert_snapshot!(format!("terminal_{}_{}_{}", style_name, placement_name, weight_name), result);
                }
            }
        }
    }

    #[test]
    fn test_snapshot_render_to_browser_all_styles() {
        let styles = vec![
            ("dashes", RuleStyle::Dashes),
            ("dots", RuleStyle::Dots),
            ("waves", RuleStyle::Waves),
            ("line_star", RuleStyle::LineStar),
            ("line_circle", RuleStyle::LineCircle),
            ("inset_line", RuleStyle::InsetLine),
            ("curtain_rod", RuleStyle::CurtainRod),
        ];

        let weights = vec![
            ("thin", RuleWeight::Thin),
            ("medium", RuleWeight::Medium),
            ("thick", RuleWeight::Thick),
        ];

        for (style_name, style) in &styles {
            for (weight_name, weight) in &weights {
                let hr = HorizontalRule::new()
                    .style(style.clone())
                    .weight(weight.clone())
                    .width("100%");

                let result = hr.render_to_browser();
                assert_snapshot!(format!("browser_{}_{}", style_name, weight_name), result);
            }
        }
    }

    #[test]
    fn test_snapshot_render_with_custom_attributes() {
        let term = Terminal::default();

        // Test with custom width and color
        let hr1 = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .placement(RulePlacement::Centered)
            .weight(RuleWeight::Thick)
            .width("75%")
            .color("red");

        let terminal_result1 = hr1.render(&term);
        let browser_result1 = hr1.render_to_browser();

        assert_snapshot!("terminal_custom_attributes", terminal_result1);
        assert_snapshot!("browser_custom_attributes", browser_result1);

        // Test with different combinations
        let hr2 = HorizontalRule::new()
            .style(RuleStyle::Dots)
            .placement(RulePlacement::Right)
            .weight(RuleWeight::Thin)
            .width("50ch")
            .color("#00ff00");

        let terminal_result2 = hr2.render(&term);
        let browser_result2 = hr2.render_to_browser();

        assert_snapshot!("terminal_custom_attributes_2", terminal_result2);
        assert_snapshot!("browser_custom_attributes_2", browser_result2);
    }
}
