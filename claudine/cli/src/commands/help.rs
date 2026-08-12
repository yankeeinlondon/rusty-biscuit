use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use color_eyre::eyre::Result;

use crate::log;

struct CommandEntry {
    name: &'static str,
    description: &'static str,
    future: bool,
}

struct CommandGroup {
    name: &'static str,
    commands: Vec<CommandEntry>,
}

fn cmd(name: &'static str, description: &'static str) -> CommandEntry {
    CommandEntry {
        name,
        description,
        future: false,
    }
}

fn groups() -> Vec<CommandGroup> {
    vec![
        CommandGroup {
            name: "Shared Resources",
            commands: vec![
                cmd("skills", "List available skills and their scopes"),
                cmd("commands", "List available slash commands and their scopes"),
                cmd(
                    "agents",
                    "List available agent definitions and their scopes",
                ),
                cmd("mcp", "Manage MCP (Model Context Protocol) servers"),
            ],
        },
        CommandGroup {
            name: "Hook Events and Actions",
            commands: vec![
                cmd("hooks", "Show registered hooks for all detected agents"),
                cmd("actions", "Show configured actions and events"),
            ],
        },
        CommandGroup {
            name: "Wrapped Execution",
            commands: vec![
                cmd(
                    "claude",
                    "Wrap Claude Code with Claudine preflight/env handling",
                ),
                cmd(
                    "codex",
                    "Wrap Codex CLI with Claudine preflight/env handling",
                ),
                cmd(
                    "gemini",
                    "Wrap Gemini CLI with Claudine preflight/env handling",
                ),
                cmd("goose", "Wrap Goose with Claudine preflight/env handling"),
                cmd(
                    "kimi",
                    "Wrap Kimi Code with Claudine preflight/env handling",
                ),
                cmd(
                    "opencode",
                    "Wrap OpenCode with Claudine preflight/env handling",
                ),
                cmd(
                    "qwen",
                    "Wrap Qwen Code with Claudine preflight/env handling",
                ),
            ],
        },
        CommandGroup {
            name: "Composition",
            commands: vec![
                cmd("compose", "Compose a Markdown document and send as prompt"),
                cmd(
                    "inline-compose",
                    "Inline composition: generate and replace body",
                ),
                cmd(
                    "sequence",
                    "Run a serial sequence of composition steps from a single document",
                ),
            ],
        },
        CommandGroup {
            name: "Administration",
            commands: vec![
                cmd("config", "Manage Claudine configuration with a TUI"),
                cmd("sync", "Re-sync hook registrations with detected agents"),
                cmd("uninstall", "Remove Claudine hooks from all agents"),
                cmd("providers", "Show provider capability matrix"),
                cmd("logs", "Query and sync Claudine JSONL logs"),
                cmd(
                    "dashboard",
                    "Show the mesh NOW view: live sessions across rendezvous hosts",
                ),
                cmd("completions", "Generate shell completions"),
                cmd(
                    "context",
                    "Show Darkmatter runtime context, expression engine, and side effects",
                ),
                cmd(
                    "errors",
                    "Show the diagnostic error-code contract (codes, dispositions, details)",
                ),
            ],
        },
    ]
}

const NAME_COL_WIDTH: usize = 18;

fn render_group(group: &CommandGroup, term: &biscuit_terminal::terminal::Terminal) -> String {
    let mut out = String::new();

    // Bold group heading via Prose
    out.push_str(&Prose::new(format!("<bold>{}:</bold>", group.name)).render(term));
    out.push('\n');

    for entry in &group.commands {
        let padded_name = format!("{:<width$}", entry.name, width = NAME_COL_WIDTH);
        let line = if entry.future {
            Prose::new(format!(
                "<dim>  {}{}  (future)</dim>",
                padded_name, entry.description
            ))
        } else {
            Prose::new(format!(
                "  <green>{}</green><dim>{}</dim>",
                padded_name, entry.description
            ))
        };
        out.push_str(&line.render(term));
        out.push('\n');
    }

    out
}

/// Render the grouped help display.
pub fn run() -> Result<()> {
    let term = log::terminal();
    let mut output = String::new();

    // Title
    output.push_str(
        &Prose::new(format!(
            "<b><yellow>Claudine</yellow></b>\n<dim><i>{}</i></dim>",
            env!("CARGO_PKG_DESCRIPTION")
        ))
        .render(&term),
    );
    output.push_str("\n\n");

    // Usage
    output.push_str(
        &Prose::new("<dim>Usage:</dim> <b>claudine</b> \\<command\\> [<i><dim>options</dim></i>]")
            .render(&term),
    );
    output.push('\n');

    // Command groups
    for group in &groups() {
        output.push('\n');
        output.push_str(&render_group(group, &term));
    }

    // Options
    output.push('\n');
    output.push_str(&Prose::new("<bold>Options:</bold>").render(&term));
    output.push('\n');
    for (name, desc) in [
        (
            "-v, --verbose",
            "Increase presentation detail for human-facing output",
        ),
        (
            "--debug <LEVEL>",
            "Set Claudine diagnostic tracing (trace, debug, info, warn, error)",
        ),
        ("--plain", "Strip ANSI escape codes from all output"),
        ("-h, --help", "Print help"),
        ("-V, --version", "Print version"),
    ] {
        let padded = format!("{:<width$}", name, width = NAME_COL_WIDTH)
            .replace('<', "\\<")
            .replace('>', "\\>");
        let line = Prose::new(format!("  <green>{}</green><dim>{}</dim>", padded, desc));
        output.push_str(&line.render(&term));
        output.push('\n');
    }

    log::output(&output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group_by_name<'a>(groups: &'a [CommandGroup], name: &str) -> &'a CommandGroup {
        groups
            .iter()
            .find(|g| g.name == name)
            .unwrap_or_else(|| panic!("expected group `{name}` in help output"))
    }

    #[test]
    fn composition_group_lists_compose_inline_compose_and_sequence_in_order() {
        // The Composition group must surface every composition subcommand
        // the CLI exposes (`compose`, `inline-compose`, `sequence`) in the
        // same spec-defined order so `claudine help` never hides a
        // command from discovery.
        let groups = groups();
        let composition = group_by_name(&groups, "Composition");
        let names: Vec<&str> = composition.commands.iter().map(|e| e.name).collect();
        assert_eq!(
            names,
            vec!["compose", "inline-compose", "sequence"],
            "Composition group must list compose, inline-compose, sequence in that order",
        );
    }

    #[test]
    fn sequence_entry_description_matches_clap_doc_comment() {
        // The help text and the clap doc comment for `Sequence` must
        // agree; drift between them causes the grouped help display to
        // diverge from `claudine sequence --help`.
        let groups = groups();
        let composition = group_by_name(&groups, "Composition");
        let sequence = composition
            .commands
            .iter()
            .find(|c| c.name == "sequence")
            .expect("sequence entry present");
        assert_eq!(
            sequence.description,
            "Run a serial sequence of composition steps from a single document",
        );
    }
}
