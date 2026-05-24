use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::utils::layout::{Alignment, WordWrap};
use claudine::reporting::{SessionDetailReport, SessionsReport};

use crate::log;
use crate::table_utils::base_table;

use super::common::{
    format_cost, format_duration, format_errors, format_event_label, format_tokens,
    render_metrics_line, render_provider_link, repo_label, truncate_str,
};

pub(super) fn render_sessions_report(report: &SessionsReport) {
    let term = crate::log::terminal();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Sessions</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );
    render_metrics_line(&report.metrics);

    let mut table = base_table(vec![
        TableColumn::new("Started"),
        TableColumn::new("Session ID"),
        TableColumn::new("Provider"),
        TableColumn::new("Repo"),
        TableColumn::new("Duration").with_alignment(Alignment::Right),
        TableColumn::new("Turns").with_alignment(Alignment::Right),
        TableColumn::new("Tools").with_alignment(Alignment::Right),
        TableColumn::new("Errors").with_alignment(Alignment::Right),
        TableColumn::new("Model"),
    ]);

    for session in &report.sessions {
        let repo = if term.width() > 140 {
            repo_label(session.repo_org.as_deref(), session.repo_name.as_deref())
        } else {
            repo_label(None, session.repo_name.as_deref())
        };

        let errors = session.tool_error_count + session.turn_error_count;
        let session_id = session.session_id.as_deref().unwrap_or("—").to_string();
        table.add_row(vec![
            session
                .started_at
                .format("%Y-%m-%d %H:%M")
                .to_string()
                .into(),
            session_id.into(),
            session.provider.to_string().into(),
            repo.into(),
            format_duration(session.duration_seconds).into(),
            session.turn_count.to_string().into(),
            session.tool_call_count.to_string().into(),
            format_errors(errors).into(),
            session
                .model
                .clone()
                .unwrap_or_else(|| "—".to_string())
                .into(),
        ]);
    }

    log::data(&table.render(&term));
}

pub(super) fn render_session_detail(report: &SessionDetailReport) {
    use claudine::events::AgenticEvent;

    let term = crate::log::terminal();
    let session = &report.session;
    let p = |markup: &str| Prose::new(markup).render(&term);

    // ── Title ──
    log::data("");
    let display_id = session
        .session_id
        .as_deref()
        .unwrap_or(&session.session_key);
    log::data(&p(&format!(
        "<blue><bold>Session</bold></blue> <dim>▸</dim> <bold>{display_id}</bold>"
    )));

    // ── Identity card ──
    let provider_markup = render_provider_link(&session.provider, session.turn_error_count > 0);
    let model = session.model.as_deref().unwrap_or("—");
    let perm = session.permission_mode.as_deref().unwrap_or("—");
    log::data(&p(&format!(
        "  <dim>Provider</dim>  {provider_markup}    <dim>Model</dim>  {model}    <dim>Permission</dim>  {perm}"
    )));

    // Time
    let started = session.started_at.format("%Y-%m-%d %H:%M:%S");
    let duration = format_duration(session.duration_seconds);
    let has_end = report
        .events
        .last()
        .is_some_and(|e| e.event == AgenticEvent::SessionEnd);
    let end_label = if has_end {
        session.ended_at.format("%H:%M:%S").to_string()
    } else {
        format!(
            "{} <dim>(last event, no session_end)</dim>",
            session.ended_at.format("%H:%M:%S")
        )
    };
    log::data(&p(&format!(
        "  <dim>Started</dim>   {started}    <dim>Ended</dim>  {end_label}    <dim>Duration</dim>  {duration}"
    )));

    // Location
    let repo = repo_label(session.repo_org.as_deref(), session.repo_name.as_deref());
    let branch = session.branch.as_deref().unwrap_or("—");
    log::data(&p(&format!(
        "  <dim>Repo</dim>      {repo}    <dim>Branch</dim>  {branch}"
    )));
    if let Some(cwd) = &session.cwd {
        log::data(&p(&format!("  <dim>CWD</dim>       {cwd}")));
    }
    if session.package_area.is_some() || session.package.is_some() {
        let area = session.package_area.as_deref().unwrap_or("—");
        let pkg = session.package.as_deref().unwrap_or("—");
        log::data(&p(&format!(
            "  <dim>Package</dim>   {area} <dim>/</dim> {pkg}"
        )));
    }

    // ── Activity summary ──
    log::data("");
    let turns = session.turn_count;
    let tools = session.tool_call_count;
    let tool_errs = session.tool_error_count;
    let turn_errs = session.turn_error_count;
    let subagents = session.subagent_count;
    let events = session.event_count;

    let mut activity_parts: Vec<String> = Vec::new();
    activity_parts.push(format!("{turns} turns"));
    if tools > 0 {
        activity_parts.push(format!("{tools} tool calls"));
    }
    if subagents > 0 {
        activity_parts.push(format!("{subagents} subagents"));
    }
    activity_parts.push(format!("{events} events"));

    let mut error_parts: Vec<String> = Vec::new();
    if tool_errs > 0 {
        error_parts.push(format!("<red>{tool_errs} tool errors</red>"));
    }
    if turn_errs > 0 {
        error_parts.push(format!("<red>{turn_errs} turn errors</red>"));
    }

    let activity_line = if error_parts.is_empty() {
        activity_parts.join(", ")
    } else {
        format!("{}, {}", activity_parts.join(", "), error_parts.join(", "))
    };
    log::data(&p(&format!("  {activity_line}")));

    // Usage
    let usage = claudine::reporting::UsageTotals {
        total_input_tokens: session.total_input_tokens,
        total_output_tokens: session.total_output_tokens,
        total_tokens: session.total_tokens,
        total_cache_read_tokens: session.total_cache_read_tokens,
        total_cost_usd: session.total_cost_usd,
    };
    if usage.total_tokens > 0 || usage.total_cost_usd > 0.0 {
        let mut parts = Vec::new();
        if usage.total_tokens > 0 {
            parts.push(format!(
                "<dim>tokens</dim> {}",
                format_tokens(usage.total_tokens)
            ));
        }
        if usage.total_input_tokens > 0 {
            parts.push(format!(
                "<dim>in</dim> {}",
                format_tokens(usage.total_input_tokens)
            ));
        }
        if usage.total_output_tokens > 0 {
            parts.push(format!(
                "<dim>out</dim> {}",
                format_tokens(usage.total_output_tokens)
            ));
        }
        if usage.total_cache_read_tokens > 0 {
            parts.push(format!(
                "<dim>cached</dim> {}",
                format_tokens(usage.total_cache_read_tokens)
            ));
        }
        if usage.total_cost_usd > 0.0 {
            parts.push(format!(
                "<dim>cost</dim> {}",
                format_cost(usage.total_cost_usd)
            ));
        }
        log::data(&p(&format!("  {}", parts.join("  "))));
    }

    // ── Tools breakdown ──
    if !report.tools.is_empty() {
        log::data("");
        log::data(&p("<bold>Tools</bold>"));
        let mut table = base_table(vec![
            TableColumn::new("Tool"),
            TableColumn::new("Calls").with_alignment(Alignment::Right),
            TableColumn::new("Errors").with_alignment(Alignment::Right),
            TableColumn::new("Class"),
        ]);
        for tool in &report.tools {
            table.add_row(vec![
                tool.tool_name.clone().into(),
                tool.call_count.to_string().into(),
                format_errors(tool.error_count).into(),
                format!("{:?}", tool.classification).to_lowercase().into(),
            ]);
        }
        log::data(&table.render(&term));
    }

    // ── Event timeline ──
    if !report.events.is_empty() {
        log::data("");
        log::data(&p("<bold>Timeline</bold>"));

        let mut table = base_table(vec![
            TableColumn::new("").with_fixed_width(3),
            TableColumn::new("Time").with_fixed_width(8),
            TableColumn::new("Event"),
            TableColumn::new("Detail").with_word_wrap(WordWrap::WrapProse(None, None)),
        ]);

        for event in &report.events {
            let icon = event.event.abbrev();
            let time = event.timestamp.format("%H:%M:%S").to_string();
            let event_label = format_event_label(&event.event);

            let mut detail_parts: Vec<String> = Vec::new();

            if let Some(tool) = &event.tool_name {
                detail_parts.push(tool.clone());
            }
            if let Some(agent) = &event.agent_type {
                detail_parts.push(format!("agent:{agent}"));
            }

            if event.total_tokens > 0 || event.cost_usd > 0.0 {
                let mut token_parts = Vec::new();
                if event.total_tokens > 0 {
                    token_parts.push(format_tokens(event.total_tokens));
                }
                if event.cost_usd > 0.0 {
                    token_parts.push(format_cost(event.cost_usd));
                }
                detail_parts.push(token_parts.join(" "));
            }

            if let Some(error) = &event.error {
                detail_parts.push(
                    Prose::new(format!("<red>{}</red>", truncate_str(error, 80)))
                        .render(&crate::log::optimistic_terminal(None)),
                );
            } else if let Some(prompt) = &event.prompt {
                detail_parts.push(
                    Prose::new(format!("<dim>\"{}\"</dim>", truncate_str(prompt, 80)))
                        .render(&crate::log::optimistic_terminal(None)),
                );
            } else if let Some(msg) = &event.notification_message {
                detail_parts.push(
                    Prose::new(format!("<dim>{}</dim>", truncate_str(msg, 80)))
                        .render(&crate::log::optimistic_terminal(None)),
                );
            }

            table.add_row(vec![
                icon.to_string().into(),
                time.into(),
                event_label.into(),
                detail_parts.join("  ").into(),
            ]);
        }

        log::data(&table.render(&term));
    }

    // ── Errors detail ──
    if !report.errors.is_empty() {
        log::data("");
        log::data(&p("<bold>Errors</bold>"));
        for (index, item) in report.errors.iter().enumerate() {
            let mut lines = vec![format!(
                "<dim>─── {} ───</dim> {} <dim>{}</dim>",
                index + 1,
                item.timestamp.format("%H:%M:%S"),
                item.event.as_slug(),
            )];
            if let Some(tool) = &item.tool_name {
                lines.push(format!("  <dim>Tool:</dim>   {tool}"));
            }
            if let Some(model) = &item.model {
                lines.push(format!("  <dim>Model:</dim>  {model}"));
            }
            lines.push(format!(
                "  <red>{}</red>",
                if item.error.is_empty() {
                    "(no details)"
                } else {
                    &item.error
                }
            ));
            if let Some(prompt) = &item.prompt {
                lines.push(format!(
                    "  <dim>Prompt:</dim> {}",
                    truncate_str(prompt, 200)
                ));
            }
            log::data(&p(&lines.join("\n")));
        }
    }
}
