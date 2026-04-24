#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;
    use insta::assert_snapshot;
    
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
        
        let alignments = vec![
            ("full", RuleAlignment::Full),
            ("centered", RuleAlignment::Centered),
            ("left", RuleAlignment::Left),
            ("right", RuleAlignment::Right),
        ];
        
        let weights = vec![
            ("thin", RuleWeight::Thin),
            ("medium", RuleWeight::Medium),
            ("thick", RuleWeight::Thick),
        ];
        
        for (style_name, style) in &styles {
            for (alignment_name, alignment) in &alignments {
                for (weight_name, weight) in &weights {
                    let hr = HorizontalRule::new()
                        .style(style.clone())
                        .alignment(alignment.clone())
                        .weight(weight.clone());
                    
                    let result = hr.render(&term);
                    assert_snapshot!(format!("terminal_{}_{}_{}", style_name, alignment_name, weight_name), result);
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
            .alignment(RuleAlignment::Centered)
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
            .alignment(RuleAlignment::Right)
            .weight(RuleWeight::Thin)
            .width("50ch")
            .color("#00ff00");
        
        let terminal_result2 = hr2.render(&term);
        let browser_result2 = hr2.render_to_browser();
        
        assert_snapshot!("terminal_custom_attributes_2", terminal_result2);
        assert_snapshot!("browser_custom_attributes_2", browser_result2);
    }
}