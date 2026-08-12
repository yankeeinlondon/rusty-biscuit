use super::*;

#[test]
fn firecrawl_double_prefix_collapses_to_single_firecrawl() {
    assert_eq!(
        humanize_tool_name("firecrawl_firecrawl_search"),
        "Firecrawl Search"
    );
}

#[test]
fn google_web_search_maps_to_canonical_label() {
    assert_eq!(humanize_tool_name("google_web_search"), "Google Web Search");
}

#[test]
fn claude_builtins_pass_through() {
    for name in [
        "Bash",
        "Edit",
        "Read",
        "Write",
        "Glob",
        "Grep",
        "WebFetch",
        "WebSearch",
        "Task",
    ] {
        assert_eq!(humanize_tool_name(name), name);
    }
}

#[test]
fn concrete_shells_humanize_with_title_case() {
    // Codex now reports the concrete shell ("zsh", "bash", etc.) as the
    // tool name; the display layer must humanize those to a leading-
    // capital form so the render reads as `Zsh(...)` not `zsh(...)`.
    assert_eq!(humanize_tool_name("zsh"), "Zsh");
    assert_eq!(humanize_tool_name("fish"), "Fish");
    assert_eq!(humanize_tool_name("dash"), "Dash");
}

#[test]
fn mcp_prefix_renders_server_and_tool() {
    assert_eq!(
        humanize_tool_name("mcp__firecrawl__deep_research"),
        "Firecrawl Deep Research"
    );
}

#[test]
fn unknown_snake_case_falls_through_to_title_case() {
    assert_eq!(humanize_tool_name("custom_local_tool"), "Custom Local Tool");
}

#[test]
fn empty_input_returns_empty_string() {
    assert_eq!(humanize_tool_name(""), "");
}
