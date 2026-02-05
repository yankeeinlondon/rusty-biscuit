//! Topics table output formatting.

use darkmatter::markdown::output::terminal::{for_terminal, TerminalOptions};
use darkmatter::markdown::Markdown;

const TOPIC_COLUMNS: &[(&str, &[&str])] = &[
    ("hardware", &["cpu", "gpu", "memory", "storage"]),
    ("filesystem", &["git", "repo", "language"]),
    (
        "programs",
        &[
            "editors",
            "utilities",
            "language-package-managers",
            "os-package-managers",
            "tts-clients",
            "terminal-apps",
            "audio",
            "agents",
        ],
    ),
];

/// Print the topics table for subsection commands.
pub fn print_topics_table() {
    let headers: Vec<&str> = TOPIC_COLUMNS.iter().map(|(header, _)| *header).collect();
    let max_rows = TOPIC_COLUMNS
        .iter()
        .map(|(_, rows)| rows.len())
        .max()
        .unwrap_or(0);

    let mut lines = Vec::new();
    lines.push(format!("| {} |", headers.join(" | ")));
    lines.push(format!(
        "| {} |",
        headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    ));

    for row_index in 0..max_rows {
        let mut cells = Vec::with_capacity(headers.len());
        for (_, rows) in TOPIC_COLUMNS.iter() {
            let value = rows.get(row_index).copied().unwrap_or("");
            cells.push(value);
        }
        lines.push(format!("| {} |", cells.join(" | ")));
    }

    let markdown = Markdown::from(lines.join("\n"));
    match for_terminal(&markdown, TerminalOptions::default()) {
        Ok(rendered) => print!("{}", rendered),
        Err(_) => println!("{}", markdown.content()),
    }
}
