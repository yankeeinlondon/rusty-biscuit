//! `MermaidMode::Automatic` through the public page path: on a terminal that
//! shows no images, an eligible diagram is drawn as text, any other diagram
//! keeps its code block, and the page's fallback policy can forbid the text.
//! Browser output treats `Automatic` like `Image`.

use biscuit_terminal::terminal::Terminal;
use biscuit_visualized::components::{MERMAID_FALLBACK_ENV, MermaidFallbackPolicy, TextFallback};
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::MermaidMode;
use regex::Regex;
use test_toolkit::EnvGuard;

const FLOWCHART: &str = "flowchart LR\n    Alpha --> Beta\n";
const PIE: &str = "pie title Pets\n    \"Dogs\" : 386\n    \"Cats\" : 85\n";

fn fenced(source: &str) -> Markdown {
    format!("```mermaid\n{source}```\n").into()
}

/// Pins the conditions the assertions depend on: the page's terminal
/// capabilities come from the process, so graphics are forced off, and
/// `MERMAID_FALLBACK` is cleared so the default policy applies.
fn no_image_process() -> [EnvGuard; 2] {
    [EnvGuard::set_safe("BISCUIT_GRAPHICS", "none"), EnvGuard::remove_safe(MERMAID_FALLBACK_ENV)]
}

fn plain(output: &str) -> String {
    Regex::new(r"\x1b\[[0-9;]*m").unwrap().replace_all(output, "").into_owned()
}

fn terminal(source: &str, configure: impl FnOnce(DarkmatterPage) -> DarkmatterPage) -> String {
    let term = Terminal::new_optimistic(80);
    let page = configure(DarkmatterPage::new(&term).with_mermaid_mode(MermaidMode::Automatic));
    plain(&page.render(&fenced(source)).expect("terminal render"))
}

#[test]
fn automatic_mermaid_mode_draws_an_eligible_flowchart_as_text() {
    let _env = no_image_process();

    let drawn = terminal(FLOWCHART, |page| page);
    assert!(!drawn.contains("Alpha --> Beta"), "the source must give way to a drawing:\n{drawn}");
    assert!(drawn.contains("Alpha") && drawn.contains("Beta"), "{drawn}");

    let code = terminal(FLOWCHART, |page| page.with_mermaid_mode(MermaidMode::Off));
    assert!(code.contains("Alpha --> Beta"), "Off keeps the code block:\n{code}");
}

#[test]
fn automatic_mermaid_mode_keeps_an_ineligible_pie_as_code() {
    let _env = no_image_process();

    let output = terminal(PIE, |page| page);
    assert!(output.contains("pie title Pets"), "{output}");
    assert!(output.contains("\"Dogs\" : 386"), "{output}");
}

#[test]
fn automatic_mermaid_fallback_policy_never_keeps_the_flowchart_source() {
    let _env = no_image_process();

    let policy = MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Never);
    let output = terminal(FLOWCHART, |page| page.with_mermaid_fallback_policy(policy));
    assert!(output.contains("Alpha --> Beta"), "{output}");
}

#[test]
fn automatic_mermaid_mode_renders_the_image_mode_browser_page() {
    let term = Terminal::new_optimistic(100);
    let md = fenced(FLOWCHART);
    let page = |mode| DarkmatterPage::new(&term).with_mermaid_mode(mode).render_to_browser(&md).unwrap();

    let automatic = page(MermaidMode::Automatic);
    assert!(automatic.contains("<svg"), "{automatic}");
    assert_eq!(automatic, page(MermaidMode::Image));
}
