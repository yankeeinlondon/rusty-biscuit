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
        // Tier 1: SVG→PNG via resvg with TerminalImage (not implemented yet - placeholder)
        // For now, we'll implement Tier 2 and Tier 3
        
        // Determine the width based on placement and terminal width
        let term_width = term.width() as usize;
        let rule_width = match self.placement {
            RulePlacement::Full => term_width,
            RulePlacement::Centered | RulePlacement::Left | RulePlacement::Right => {
                // Default to 80% of terminal width for non-full placement
                (term_width as f32 * 0.8) as usize
            }
        };
        
        // Clamp to reasonable minimum and maximum
        let rule_width = rule_width.clamp(10, term_width);
        
        // Generate the rule content based on style
        let rule_content = self.generate_terminal_content(rule_width, term);
        
        // Apply placement
        match self.placement {
            RulePlacement::Full => rule_content,
            RulePlacement::Centered => {
                let padding = (term_width.saturating_sub(rule_content.len())) / 2;
                format!("{}{}", " ".repeat(padding), rule_content)
            }
            RulePlacement::Left => rule_content,
            RulePlacement::Right => {
                let padding = term_width.saturating_sub(rule_content.len());
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HorizontalRule {
    /// Generates terminal content for the horizontal rule based on style and width.
    fn generate_terminal_content(&self, width: usize, term: &Terminal) -> String {
        match &self.style {
            RuleStyle::Dashes => "-".repeat(width),
            RuleStyle::Dots => {
                // Use Unicode middle dot if supported, fallback to period
                if self.supports_unicode_dots(term) {
                    "·".repeat(width)
                } else {
                    ".".repeat(width)
                }
            }
            RuleStyle::Waves => {
                // Use Unicode wave dash if supported, fallback to tilde
                if self.supports_unicode_waves(term) {
                    "〜".repeat(width)
                } else {
                    "~".repeat(width)
                }
            }
            RuleStyle::LineStar => {
                // Create pattern: * * * (star followed by space)
                let mut result = String::new();
                for i in 0..width {
                    if i % 2 == 0 {
                        result.push('*');
                    } else {
                        result.push(' ');
                    }
                }
                result
            }
            RuleStyle::LineCircle => {
                // Use Unicode white circle if supported, fallback to 'o'
                if self.supports_unicode_circles(term) {
                    let mut result = String::new();
                    for i in 0..width {
                        if i % 2 == 0 {
                            result.push('○');
                        } else {
                            result.push(' ');
                        }
                    }
                    result
                } else {
                    let mut result = String::new();
                    for i in 0..width {
                        if i % 2 == 0 {
                            result.push('o');
                        } else {
                            result.push(' ');
                        }
                    }
                    result
                }
            }
            RuleStyle::InsetLine => {
                // Create inset effect with different characters at edges
                if width < 3 {
                    "=".repeat(width)
                } else {
                    let mut result = String::new();
                    result.push('[');
                    result.push_str(&"=".repeat(width - 2));
                    result.push(']');
                    result
                }
            }
            RuleStyle::CurtainRod => {
                // Create decorative ends with simple line in middle
                if width < 5 {
                    "=".repeat(width)
                } else {
                    let mut result = String::new();
                    result.push('「');
                    result.push_str(&"=".repeat(width - 4));
                    result.push('」');
                    result
                }
            }
        }
    }
    
    /// Checks if the terminal supports Unicode dots (middle dot character).
    fn supports_unicode_dots(&self, _term: &Terminal) -> bool {
        // For now, assume basic Unicode support
        // In a real implementation, this would check terminal capabilities
        true
    }
    
    /// Checks if the terminal supports Unicode waves (wave dash character).
    fn supports_unicode_waves(&self, _term: &Terminal) -> bool {
        // For now, assume basic Unicode support
        true
    }
    
    /// Checks if the terminal supports Unicode circles (white circle character).
    fn supports_unicode_circles(&self, _term: &Terminal) -> bool {
        // For now, assume basic Unicode support
        true
    }
}

impl BrowserRenderable for HorizontalRule {
    fn render_to_browser(&self) -> String {
        // Generate SVG with currentColor for stroke
        let stroke_width = match self.weight {
            RuleWeight::Thin => "1",
            RuleWeight::Medium => "2",
            RuleWeight::Thick => "3",
        };
        
        let width_attr = self.width.as_deref().unwrap_or("100%");
        let color_attr = self.color.as_deref().unwrap_or("currentColor");
        
        // Create SVG line element
        format!(
            r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg" style="display: block; margin: {} auto {} auto;">
  <line x1="0" y1="50%" x2="100%" y2="50%" stroke="{}" stroke-width="{}" stroke-linecap="round"/>
</svg>"#,
            width_attr,
            stroke_width,
            self.layout.top_margin.to_css_value("0"),
            self.layout.bottom_margin.to_css_value("0"),
            color_attr,
            stroke_width
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
mod test {
    use super::*;
    use crate::terminal::Terminal;
    use crate::utils::layout::{Layout, Margin};
    
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
    
    // ... other tests would go here
}

#[cfg(test)]
mod horizontal_rule_snapshot {
    use super::*;
    use crate::terminal::Terminal;
    use insta::assert_snapshot;
    
    #[test]
    fn test_snapshot_render_all_styles() {
        let term = Terminal::default();
        
        let styles = vec![
            ("dashes", RuleStyle::Dashes),
        ];
        
        let placements = vec![
            ("full", RulePlacement::Full),
        ];
        
        let weights = vec![
            ("medium", RuleWeight::Medium),
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
}