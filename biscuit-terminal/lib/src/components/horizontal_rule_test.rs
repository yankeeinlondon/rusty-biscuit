#[cfg(test)]
mod tests {
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
    fn test_render_dashes_full() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() > 0);
        assert!(result.chars().all(|c| c == '-'));
    }
    
    #[test]
    fn test_render_dots_full() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dots)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() > 0);
        // Should use Unicode middle dot or period
        assert!(result.chars().all(|c| c == '·' || c == '.'));
    }
    
    #[test]
    fn test_render_waves_full() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Waves)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() > 0);
        // Should use Unicode wave dash or tilde
        assert!(result.chars().all(|c| c == '〜' || c == '~'));
    }
    
    #[test]
    fn test_render_line_star() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineStar)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() > 0);
        // Should alternate between '*' and ' '
        let chars: Vec<char> = result.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if i % 2 == 0 {
                assert_eq!(ch, '*');
            } else {
                assert_eq!(ch, ' ');
            }
        }
    }
    
    #[test]
    fn test_render_line_circle() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::LineCircle)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() > 0);
        // Should alternate between circle and ' '
        let chars: Vec<char> = result.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if i % 2 == 0 {
                assert!(ch == '○' || ch == 'o');
            } else {
                assert_eq!(ch, ' ');
            }
        }
    }
    
    #[test]
    fn test_render_inset_line() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::InsetLine)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() >= 3);
        assert!(result.starts_with('['));
        assert!(result.ends_with(']'));
        assert!(result[1..result.len()-1].chars().all(|c| c == '='));
    }
    
    #[test]
    fn test_render_curtain_rod() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::CurtainRod)
            .placement(RulePlacement::Full);
        let term = Terminal::default();
        let result = hr.render(&term);
        assert!(result.len() >= 5);
        assert!(result.starts_with('「'));
        assert!(result.ends_with('」'));
        assert!(result[1..result.len()-1].chars().all(|c| c == '='));
    }
    
    #[test]
    fn test_render_centered() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Centered);
        let term = Terminal::default();
        let result = hr.render(&term);
        let term_width = term.width() as usize;
        let rule_content = "-".repeat((term_width as f32 * 0.8) as usize);
        let expected_padding = (term_width - rule_content.len()) / 2;
        assert!(result.starts_with(&" ".repeat(expected_padding)));
        assert!(result[expected_padding..].starts_with(&rule_content));
    }
    
    #[test]
    fn test_render_left() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Left);
        let term = Terminal::default();
        let result = hr.render(&term);
        let term_width = term.width() as usize;
        let rule_content = "-".repeat((term_width as f32 * 0.8) as usize);
        assert!(result.starts_with(&rule_content));
    }
    
    #[test]
    fn test_render_right() {
        let hr = HorizontalRule::new()
            .style(RuleStyle::Dashes)
            .placement(RulePlacement::Right);
        let term = Terminal::default();
        let result = hr.render(&term);
        let term_width = term.width() as usize;
        let rule_content = "-".repeat((term_width as f32 * 0.8) as usize);
        let expected_padding = term_width - rule_content.len();
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
        assert!(result.contains(r#"stroke-width="2""#));
    }
    
    #[test]
    fn test_render_to_browser_default() {
        let hr = HorizontalRule::new();
        let result = hr.render_to_browser();
        assert!(result.contains(r#"width="100%""#));
        assert!(result.contains(r#"stroke="currentColor""#));
        assert!(result.contains(r#"stroke-width="2""#));
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
        let layout = hr.layout();
        assert_eq!(layout, &Layout::default());
        
        hr.layout_mut().top_margin = Margin::Chars(2);
        assert_eq!(hr.layout().top_margin, Margin::Chars(2));
    }
    
    #[test]
    fn test_as_any() {
        let hr = HorizontalRule::new();
        let any_ref = hr.as_any();
        let downcast_ref = any_ref.downcast_ref::<HorizontalRule>();
        assert!(downcast_ref.is_some());
        assert_eq!(downcast_ref.unwrap().style, RuleStyle::Dashes);
    }
    
    #[test]
    fn test_browser_renderable_as_any() {
        let hr = HorizontalRule::new();
        let any_ref = hr.as_any();
        let downcast_ref = any_ref.downcast_ref::<HorizontalRule>();
        assert!(downcast_ref.is_some());
        assert_eq!(downcast_ref.unwrap().style, RuleStyle::Dashes);
    }
}