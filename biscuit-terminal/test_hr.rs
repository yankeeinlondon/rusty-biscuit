use biscuit_terminal::components::{HorizontalRule, RuleStyle, RulePlacement, RuleWeight};
use biscuit_terminal::terminal::Terminal;

fn main() {
    let hr = HorizontalRule::new()
        .style(RuleStyle::Dashes)
        .placement(RulePlacement::Full)
        .weight(RuleWeight::Medium);
    
    let term = Terminal::default();
    println!("Terminal render: {}", hr.render(&term));
    println!("Browser render: {}", hr.render_to_browser());
}