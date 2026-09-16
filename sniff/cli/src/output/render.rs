//! Shared rendering helpers for sniff CLI output.
//!
//! This module contains pure formatting functions used across multiple
//! output submodules.

use std::path::Path;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

use super::perf_tree;

/// Format bytes into human-readable units (KB, MB, GB, TB)
pub(crate) fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Format large numbers with comma separators
pub(crate) fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

/// Convert absolute path to relative path from repo root
pub(crate) fn relative_path(path: &Path, repo_root: Option<&Path>) -> String {
    if let Some(root) = repo_root
        && let Ok(rel) = path.strip_prefix(root)
    {
        return rel.display().to_string();
    }
    path.display().to_string()
}

/// Format uptime in seconds to a human-readable string
pub(crate) fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();

    if days > 0 {
        parts.push(format!("{} day{}", days, if days == 1 { "" } else { "s" }));
    }
    if hours > 0 {
        parts.push(format!(
            "{} hour{}",
            hours,
            if hours == 1 { "" } else { "s" }
        ));
    }
    if minutes > 0 || (days == 0 && hours == 0 && secs == 0) {
        parts.push(format!(
            "{} minute{}",
            minutes,
            if minutes == 1 { "" } else { "s" }
        ));
    }
    if secs > 0 && days == 0 && hours == 0 {
        parts.push(format!(
            "{} second{}",
            secs,
            if secs == 1 { "" } else { "s" }
        ));
    }

    if parts.is_empty() {
        "0 seconds".to_string()
    } else {
        parts.join(", ")
    }
}

/// Render a performance report as a `## Performance` section.
///
/// The section carries a hierarchical timing tree rooted at `Total` and, when
/// the report recorded any work counters, a separate count tree rooted at
/// `Counters` below it. Sniff decides what each tree contains — the hierarchy
/// read out of dotted stage and counter names, measured-versus-synthetic
/// values, wall-clock shares, call counts, and the single HOT row.
/// `biscuit-terminal` decides how it reaches the terminal, including width,
/// connectors, unit alignment, glyph fallback, and color degradation, so the
/// detected [`Terminal`] is the only capability input.
///
/// The returned string ends in exactly one newline; the `CliPerf` emit seams
/// print it without adding one.
pub fn render_performance_section(report: &sniff::PerformanceReport) -> String {
    let terminal = Terminal::default();

    let mut out = String::new();
    out.push_str("\n## Performance\n\n");
    out.push_str(
        perf_tree::timing_metrics_tree(report)
            .render(&terminal)
            .trim_end(),
    );

    if let Some(counters) = perf_tree::counter_metrics_tree(report) {
        out.push_str("\n\n");
        out.push_str(counters.render(&terminal).trim_end());
    }

    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use biscuit_terminal::prelude::strip_escape_codes;
    use sniff::PerformanceReport;
    use sniff::performance::PerformanceStage;

    fn perf_report(stages: &[(&str, f64, u64)], counters: &[(&str, u64)]) -> PerformanceReport {
        PerformanceReport {
            total_duration_ms: 1000.0,
            stages: stages
                .iter()
                .map(|(name, ms, calls)| {
                    (
                        (*name).to_string(),
                        PerformanceStage {
                            calls: *calls,
                            total_duration_ms: *ms,
                            max_duration_ms: *ms,
                            last_duration_ms: *ms,
                        },
                    )
                })
                .collect(),
            counters: counters
                .iter()
                .map(|(name, value)| ((*name).to_string(), *value))
                .collect(),
        }
    }

    fn rendered(report: &PerformanceReport) -> String {
        strip_escape_codes(render_performance_section(report))
    }

    #[test]
    fn the_performance_section_keeps_its_heading_and_one_trailing_newline() {
        let section = render_performance_section(&perf_report(
            &[("detect.total", 1000.0, 1), ("hardware.gpu", 40.0, 1)],
            &[],
        ));

        assert!(
            section.starts_with("\n## Performance\n\n"),
            "unexpected head: {:?}",
            section.chars().take(24).collect::<String>()
        );
        assert!(section.ends_with('\n'));
        assert!(!section.ends_with("\n\n"), "section ends with blank lines");
    }

    #[test]
    fn the_timing_tree_replaces_the_flat_stage_list() {
        let section = rendered(&perf_report(
            &[
                ("detect.total", 1000.0, 1),
                ("detect.hardware", 400.0, 1),
                ("hardware.gpu", 40.0, 7),
            ],
            &[],
        ));

        assert!(section.contains("Total"), "{section}");
        // R-9: rows are labelled by the last dotted segment, nested under their
        // parent rather than repeating the full key.
        assert!(section.contains("gpu"), "{section}");
        assert!(!section.contains("hardware.gpu"), "{section}");
        // The retired flat rendering and its headers are gone.
        assert!(!section.contains("Total: "), "{section}");
        assert!(!section.contains("Stages:"), "{section}");
        assert!(!section.contains("ms total"), "{section}");
        // The overlap note travels with the timing tree.
        assert!(section.contains("do not sum to wall-clock time"), "{section}");
    }

    #[test]
    fn the_counter_tree_is_omitted_when_the_report_has_no_counters() {
        let section = rendered(&perf_report(&[("detect.total", 1000.0, 1)], &[]));

        assert!(!section.contains("Counters"), "{section}");
    }

    #[test]
    fn the_counter_tree_follows_the_timing_tree_after_a_blank_line() {
        let section = rendered(&perf_report(
            &[("detect.total", 1000.0, 1), ("hardware.gpu", 40.0, 1)],
            &[("process.spawns", 3), ("git.blob_loads", 27)],
        ));

        let lines: Vec<&str> = section.lines().collect();
        let root = lines
            .iter()
            .position(|line| line.trim_start().starts_with("Counters"))
            .unwrap_or_else(|| panic!("no Counters root in:\n{section}"));

        assert!(root > 0, "Counters is the first line of:\n{section}");
        assert!(
            lines[root - 1].trim().is_empty(),
            "no blank line above Counters in:\n{section}"
        );
        // Counter rows nest by their dotted names and keep their raw values.
        assert!(section.contains("blob_loads"), "{section}");
        assert!(section.contains("27"), "{section}");
        // Counters never enter the timing tree: the timing rows end at the note.
        let timing = lines[..root].join("\n");
        assert!(!timing.contains("blob_loads"), "{timing}");
    }

    #[test]
    fn a_report_with_no_stages_renders_the_root_alone() {
        let section = rendered(&perf_report(&[], &[]));

        assert!(section.contains("Total"), "{section}");
        assert!(!section.contains("HOT"), "{section}");
    }

    #[test]
    fn a_malformed_report_renders_without_panicking() {
        let section = rendered(&PerformanceReport {
            total_duration_ms: f64::NAN,
            stages: BTreeMap::new(),
            counters: BTreeMap::new(),
        });

        assert!(section.contains("Total"), "{section}");
    }

    #[test]
    fn test_format_uptime_zero() {
        assert_eq!(format_uptime(0), "0 minutes");
    }

    #[test]
    fn test_format_uptime_seconds() {
        assert_eq!(format_uptime(30), "30 seconds");
        assert_eq!(format_uptime(1), "1 second");
    }

    #[test]
    fn test_format_uptime_minutes() {
        assert_eq!(format_uptime(60), "1 minute");
        assert_eq!(format_uptime(120), "2 minutes");
        assert_eq!(format_uptime(90), "1 minute, 30 seconds");
    }

    #[test]
    fn test_format_uptime_hours() {
        assert_eq!(format_uptime(3600), "1 hour");
        assert_eq!(format_uptime(3660), "1 hour, 1 minute");
        assert_eq!(format_uptime(7200), "2 hours");
        assert_eq!(format_uptime(7320), "2 hours, 2 minutes");
    }

    #[test]
    fn test_format_uptime_days() {
        assert_eq!(format_uptime(86400), "1 day");
        assert_eq!(format_uptime(86400 + 3600), "1 day, 1 hour");
        assert_eq!(format_uptime(86400 + 3660), "1 day, 1 hour, 1 minute");
        assert_eq!(
            format_uptime(2 * 86400 + 5 * 3600 + 30 * 60),
            "2 days, 5 hours, 30 minutes"
        );
    }

    #[test]
    fn test_format_uptime_long() {
        // 16 days, 13 hours, 26 minutes
        assert_eq!(
            format_uptime(16 * 86400 + 13 * 3600 + 26 * 60),
            "16 days, 13 hours, 26 minutes"
        );
    }
}
