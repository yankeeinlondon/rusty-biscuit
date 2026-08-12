use super::*;
use biscuit_terminal::prelude::strip_escape_codes;
use claudine::provider::Provider;
use serde_json::json;

/// Build a minimal `DryRunRender` for table assertions.
fn render_with(
    name: Option<&str>,
    description: Option<&str>,
    agent: AgentResolutionState,
    model: Option<&str>,
    yolo: bool,
    area: Option<&str>,
) -> DryRunRender {
    render_with_session(
        name,
        description,
        agent,
        model,
        yolo,
        false,
        SessionInteractivitySource::Default,
        area,
    )
}

/// Build a `DryRunRender` with explicit session interactivity state.
#[allow(clippy::too_many_arguments)]
fn render_with_session(
    name: Option<&str>,
    description: Option<&str>,
    agent: AgentResolutionState,
    model: Option<&str>,
    yolo: bool,
    session_interactive: bool,
    session_source: SessionInteractivitySource,
    area: Option<&str>,
) -> DryRunRender {
    DryRunRender {
        body: "body".to_string(),
        frontmatter: json!({}),
        name: name.map(str::to_string),
        description: description.map(str::to_string),
        agent,
        model: model.map(str::to_string),
        yolo,
        session_interactive,
        session_source,
        area: area.map(str::to_string),
        document_path: PathBuf::from("/tmp/doc.md"),
        deferred_lifecycle_keys: Vec::new(),
        provider_args: Vec::new(),
    }
}

/// Plain (escape-stripped) render of the metadata table for semantic
/// assertions that survive SGR-collapsing across capture surfaces.
fn plain_table(render: &DryRunRender) -> String {
    let term = Terminal::default();
    strip_escape_codes(render_metadata_table(render, &term))
}

/// Plain (escape-stripped) render of just the agent cell, without table
/// borders or word-wrap, so multi-line assertions are stable.
fn plain_agent_cell(state: &AgentResolutionState) -> String {
    let term = Terminal::default();
    strip_escape_codes(render_agent_cell(state, &term))
}

/// Extract the trimmed value cell for a single-row label from the plain
/// metadata table. Returns `None` if the label is not found.
fn plain_row_value(plain: &str, label: &str) -> Option<String> {
    for line in plain.lines() {
        let cells: Vec<&str> = line.split('│').collect();
        if cells.len() >= 3 && cells[1].trim() == label {
            return Some(cells[2].trim().to_string());
        }
    }
    None
}

#[test]
fn table_omits_description_when_absent() {
    let render = render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
    let plain = plain_table(&render);
    assert!(!plain.contains("Description"));
}

#[test]
fn table_shows_description_when_present() {
    let render = render_with(
        Some("doc"),
        Some("a helpful doc"),
        AgentResolutionState::NoAgent,
        None,
        false,
        None,
    );
    let plain = plain_table(&render);
    assert!(plain.contains("Description"));
    assert!(plain.contains("a helpful doc"));
}

#[test]
fn table_omits_area_when_absent() {
    let render = render_with(
        Some("doc"),
        None,
        AgentResolutionState::NoAgent,
        None,
        false,
        None,
    );
    let plain = plain_table(&render);
    assert!(!plain.contains("Area"));
}

#[test]
fn table_shows_area_when_present() {
    let render = render_with(
        Some("doc"),
        None,
        AgentResolutionState::NoAgent,
        None,
        false,
        Some("claudine"),
    );
    let plain = plain_table(&render);
    assert!(plain.contains("Area"));
    assert!(plain.contains("claudine"));
}

#[test]
fn table_omits_provider_args_when_empty() {
    let render = render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
    let plain = plain_table(&render);
    assert!(!plain.contains("Provider args"));
}

#[test]
fn table_shows_redacted_provider_args_when_present() {
    let mut render =
        render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
    render.provider_args = vec!["-c".to_string(), "model_reasoning_effort=low".to_string()];
    let plain = plain_table(&render);
    let value = plain_row_value(&plain, "Provider args").expect("Provider args row should exist");
    assert!(value.contains("-c"));
    assert!(value.contains("model_reasoning_effort=low"));
}

#[test]
fn table_omits_deferred_row_when_no_keys() {
    let render = render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
    let plain = plain_table(&render);
    assert!(!plain.contains("Deferred"));
}

#[test]
fn table_shows_deferred_keys_labeled_event_time() {
    let mut render =
        render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
    render.deferred_lifecycle_keys = vec!["failure".to_string(), "finalize".to_string()];
    let plain = plain_table(&render);
    let value = plain_row_value(&plain, "Deferred").expect("Deferred row should exist");
    assert!(value.contains("event-time"), "row should label event-time: {value}");
    assert!(value.contains("failure"));
    assert!(value.contains("finalize"));
}

#[test]
fn agent_falls_back_to_interactive() {
    let render = render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
    let plain = plain_table(&render);
    assert!(plain.contains("interactive"));
}

#[test]
fn agent_shows_resolved_provider() {
    let render = render_with(
        Some("doc"),
        None,
        AgentResolutionState::Selected {
            provider: Provider::Claude,
        },
        None,
        false,
        None,
    );
    let plain = plain_table(&render);
    let agent_value = plain_row_value(&plain, "Agent").expect("Agent row should exist");
    assert!(agent_value.contains(&Provider::Claude.to_string()));
    assert!(!agent_value.contains("interactive"));
}

#[test]
fn model_falls_back_to_default() {
    let render = render_with(
        Some("doc"),
        None,
        AgentResolutionState::NoAgent,
        None,
        false,
        None,
    );
    let plain = plain_table(&render);
    let model_value = plain_row_value(&plain, "Model").expect("Model row should exist");
    assert!(model_value.contains("default"));
}

#[test]
fn model_shows_resolved_model() {
    let render = render_with(
        Some("doc"),
        None,
        AgentResolutionState::NoAgent,
        Some("opus-4.8"),
        false,
        None,
    );
    let plain = plain_table(&render);
    let model_value = plain_row_value(&plain, "Model").expect("Model row should exist");
    assert!(model_value.contains("opus-4.8"));
    assert!(!model_value.contains("default"));
}

#[test]
fn yolo_true_and_false() {
    let yes = plain_table(&render_with(
        Some("doc"),
        None,
        AgentResolutionState::NoAgent,
        None,
        true,
        None,
    ));
    assert!(yes.contains("true"));
    let no = plain_table(&render_with(
        Some("doc"),
        None,
        AgentResolutionState::NoAgent,
        None,
        false,
        None,
    ));
    assert!(no.contains("false"));
}

#[test]
fn session_row_renders_for_each_source() {
    for (interactive, source, expected) in [
        (true, SessionInteractivitySource::InteractiveFlag, "interactive (--interactive)"),
        (true, SessionInteractivitySource::Frontmatter, "interactive (frontmatter)"),
        (false, SessionInteractivitySource::NoInteractiveFlag, "non-interactive (--no-interactive)"),
        (false, SessionInteractivitySource::Default, "non-interactive (default)"),
    ] {
        let render = render_with_session(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            interactive,
            source,
            None,
        );
        let plain = plain_table(&render);
        assert!(
            plain.contains("Session"),
            "Session row should appear for source {source}"
        );
        assert!(
            plain.contains(expected),
            "expected '{expected}' for source {source}; got:\n{plain}"
        );
    }
}

#[test]
fn session_row_no_interactive_overrides_frontmatter_true() {
    let render = render_with_session(
        Some("doc"),
        None,
        AgentResolutionState::NoAgent,
        None,
        false,
        false,
        SessionInteractivitySource::NoInteractiveFlag,
        None,
    );
    let plain = plain_table(&render);
    assert!(plain.contains("Session"));
    assert!(plain.contains("non-interactive (--no-interactive)"));
}

#[test]
fn document_uses_name_when_set() {
    let render = render_with(
        Some("My Document"),
        None,
        AgentResolutionState::NoAgent,
        None,
        false,
        None,
    );
    let plain = plain_table(&render);
    assert!(plain.contains("My Document"));
}

#[test]
fn document_uses_path_when_name_absent() {
    let render = render_with(
        None,
        None,
        AgentResolutionState::NoAgent,
        None,
        false,
        None,
    );
    let plain = plain_table(&render);
    // No `name` ⇒ the document label is derived from the path.
    assert!(plain.contains("doc.md"));
}

#[test]
fn relative_or_abs_renders_windows_shaped_paths_portably() {
    assert_eq!(
        relative_or_abs(Path::new(r"C:\repo\prompts\doc.md")),
        "C:/repo/prompts/doc.md"
    );
}

#[test]
fn frontmatter_renders_yaml_for_keys() {
    let fm = json!({ "name": "demo", "agent": "claude" });
    let term = Terminal::default();
    let plain = strip_escape_codes(render_frontmatter(&fm, &term));
    assert!(plain.contains("name:"));
    assert!(plain.contains("demo"));
    assert!(plain.contains("agent:"));
}

#[test]
fn frontmatter_has_one_ch_left_margin() {
    let fm = json!({ "key": "value" });
    let term = Terminal::default();
    let plain = strip_escape_codes(render_frontmatter(&fm, &term));
    // Every non-empty line should start with a single space (the 1ch margin).
    for line in plain.lines() {
        if !line.trim().is_empty() {
            assert!(
                line.starts_with(' '),
                "line should have 1ch left margin: {line:?}"
            );
        }
    }
}

#[test]
fn frontmatter_heading_contains_bold_and_italic() {
    let term = Terminal::default();
    let rendered = render_frontmatter_heading(&term);
    let plain = strip_escape_codes(&rendered);
    assert!(plain.contains("Frontmatter"), "heading should contain 'Frontmatter': {plain}");
    assert!(plain.contains("resolved"), "heading should contain 'resolved': {plain}");
}

#[test]
fn hr_renders_non_empty() {
    let term = Terminal::default();
    let rendered = render_hr(&term);
    let plain = strip_escape_codes(&rendered);
    assert!(!plain.trim().is_empty(), "horizontal rule should not be empty");
}

#[test]
fn frontmatter_yaml_round_trips_a_value() {
    let original = json!({
        "name": "demo",
        "tags": ["a", "b"],
        "nested": { "k": "v" },
    });
    let yaml = frontmatter_to_yaml(&original);
    let parsed: Value =
        biscuit_file::serde_yaml_ng::from_str(&yaml).expect("yaml round-trip parses");
    assert_eq!(parsed, original);
}

// -- Agent-resolution state rendering (Phase 4) --------------------------

#[test]
fn agent_no_agent_renders_unordered_list() {
    let plain = plain_agent_cell(&AgentResolutionState::NoAgent);
    assert!(plain.contains("didn't specify the Agent"));
    assert!(plain.contains("didn't suggest any Agents"));
    assert!(plain.contains("interactive"));
}

#[test]
fn agent_selected_renders_provider_name() {
    let plain = plain_agent_cell(&AgentResolutionState::Selected {
        provider: Provider::Codex,
    });
    assert!(plain.contains(&Provider::Codex.to_string()));
    assert!(!plain.contains("Invalid Agent"));
    assert!(!plain.contains("interactive"));
}

#[test]
fn agent_single_invalid_renders_error_and_explanation() {
    let plain = plain_agent_cell(&AgentResolutionState::SingleInvalid {
        hint: "unknown-provider".into(),
    });
    assert!(plain.contains("Invalid Agent"));
    assert!(plain.contains("unknown-provider"));
    assert!(plain.contains("Frontmatter"));
}

#[test]
fn agent_single_not_installed_renders_warning_and_explanation() {
    let plain = plain_agent_cell(&AgentResolutionState::SingleNotInstalled {
        provider: Provider::Gemini,
    });
    assert!(plain.contains("Agent Not Installed"));
    assert!(plain.contains(&Provider::Gemini.to_string()));
    assert!(plain.contains("not installed on this host"));
}

#[test]
fn agent_list_multiple_installed_renders_choice_header_and_list() {
    let plain = plain_agent_cell(&AgentResolutionState::ListMultipleInstalled {
        installed: vec![Provider::Claude, Provider::Codex],
        not_installed: vec![Provider::Gemini],
        invalid: vec![],
    });
    assert!(plain.contains("choose interactively between suggested Agents"));
    assert!(plain.contains(&Provider::Claude.to_string()));
    assert!(plain.contains(&Provider::Codex.to_string()));
    assert!(plain.contains(&Provider::Gemini.to_string()));
}

#[test]
fn agent_list_multiple_installed_dimms_not_installed() {
    // Semantic test: the plain text contains the not-installed provider;
    // L2 tests assert the actual dim SGR.
    let plain = plain_agent_cell(&AgentResolutionState::ListMultipleInstalled {
        installed: vec![Provider::Claude],
        not_installed: vec![Provider::Gemini],
        invalid: vec![],
    });
    assert!(plain.contains(&Provider::Gemini.to_string()));
}

#[test]
fn agent_list_multiple_installed_shows_invalid_suggestions() {
    let plain = plain_agent_cell(&AgentResolutionState::ListMultipleInstalled {
        installed: vec![Provider::Claude],
        not_installed: vec![],
        invalid: vec!["bad-agent".into()],
    });
    assert!(plain.contains("NOT"));
    assert!(plain.contains("valid Agents"));
    assert!(plain.contains("bad-agent"));
}

#[test]
fn agent_list_one_installed_renders_auto_select_header() {
    let plain = plain_agent_cell(&AgentResolutionState::ListOneInstalled {
        selected: Provider::Claude,
        not_installed: vec![Provider::Gemini],
        invalid: vec![],
    });
    assert!(plain.contains(&Provider::Claude.to_string()));
    assert!(plain.contains("without the need for interactive prompting"));
    assert!(plain.contains("only"));
    assert!(plain.contains("is installed on this host"));
}

#[test]
fn agent_list_one_installed_with_invalid_shows_invalid_list() {
    let plain = plain_agent_cell(&AgentResolutionState::ListOneInstalled {
        selected: Provider::Claude,
        not_installed: vec![],
        invalid: vec!["bad-agent".into(), "worse-agent".into()],
    });
    assert!(plain.contains("NOT"));
    assert!(plain.contains("valid Agents"));
    assert!(plain.contains("bad-agent"));
    assert!(plain.contains("worse-agent"));
}

#[test]
fn agent_zero_installed_list_renders_header_and_dimmed_suggestions() {
    let plain = plain_agent_cell(&AgentResolutionState::ZeroInstalledList {
        not_installed: vec![Provider::Gemini, Provider::Goose],
        invalid: vec![],
    });
    assert!(plain.contains("None of the suggested agents are installed/valid"));
    assert!(plain.contains(&Provider::Gemini.to_string()));
    assert!(plain.contains(&Provider::Goose.to_string()));
}

#[test]
fn agent_zero_installed_list_with_invalid_shows_invalid_list() {
    let plain = plain_agent_cell(&AgentResolutionState::ZeroInstalledList {
        not_installed: vec![Provider::Gemini],
        invalid: vec!["bad-agent".into()],
    });
    assert!(plain.contains("None of the suggested agents are installed/valid"));
    assert!(plain.contains(&Provider::Gemini.to_string()));
    assert!(plain.contains("NOT"));
    assert!(plain.contains("valid Agents"));
    assert!(plain.contains("bad-agent"));
}

#[test]
fn agent_cell_preserves_single_table_row() {
    // Every agent state must keep its breakdown inside the single Agent
    // value cell; no extra metadata rows are added.
    let render = render_with(
        Some("doc"),
        None,
        AgentResolutionState::ListMultipleInstalled {
            installed: vec![Provider::Claude, Provider::Codex],
            not_installed: vec![Provider::Gemini],
            invalid: vec!["bad".into()],
        },
        None,
        false,
        None,
    );
    let plain = plain_table(&render);
    // Count lines that contain the Agent row label after stripping table
    // border characters. There should be exactly one.
    let agent_label_lines: Vec<&str> = plain
        .lines()
        .filter(|l| {
            let cleaned: String = l.chars().filter(|&c| c != '│').collect();
            cleaned.trim().starts_with("Agent")
        })
        .collect();
    assert_eq!(
        agent_label_lines.len(),
        1,
        "Agent label should appear exactly once (single row); plain:\n{plain}"
    );
}

#[test]
fn agent_cell_multiline_preserves_table_alignment() {
    // A multi-line Agent cell must keep the table's two-column alignment:
    // every `│` separator appears at the same positions across all rows.
    let render = render_with(
        Some("doc"),
        None,
        AgentResolutionState::NoAgent,
        None,
        false,
        None,
    );
    let plain = plain_table(&render);
    let lines: Vec<&str> = plain.lines().collect();

    // Find the table region by its border characters.
    let table_start = lines
        .iter()
        .position(|l| l.contains('┌'))
        .expect("table top border should exist");
    let table_end = lines
        .iter()
        .skip(table_start)
        .position(|l| l.contains('└'))
        .map(|i| table_start + i)
        .expect("table bottom border should exist");

    let mut expected_positions: Option<Vec<usize>> = None;
    for line in &lines[table_start..=table_end] {
        let positions: Vec<usize> = line
            .char_indices()
            .filter(|(_, c)| *c == '│')
            .map(|(i, _)| i)
            .collect();
        if positions.len() >= 2 {
            if let Some(ref expected) = expected_positions {
                assert_eq!(
                    positions, *expected,
                    "table column separators misaligned.\nline: {line:?}\nplain:\n{plain}"
                );
            } else {
                expected_positions = Some(positions);
            }
        }
    }
}

#[test]
fn table_preserves_one_ch_left_margin() {
    // The table is rendered with `Edges::x(Length::ch(1))`, so every
    // table line must start with a single space (the 1ch left offset).
    let render = render_with(
        Some("doc"),
        None,
        AgentResolutionState::NoAgent,
        None,
        false,
        None,
    );
    let plain = plain_table(&render);
    let lines: Vec<&str> = plain.lines().collect();

    let table_start = lines
        .iter()
        .position(|l| l.contains('┌'))
        .expect("table top border should exist");
    let table_end = lines
        .iter()
        .skip(table_start)
        .position(|l| l.contains('└'))
        .map(|i| table_start + i)
        .expect("table bottom border should exist");

    for line in &lines[table_start..=table_end] {
        assert!(
            line.starts_with(' '),
            "every table line must have a 1ch left margin (start with space).\n\
             line: {line:?}\nplain:\n{plain}"
        );
    }
}
