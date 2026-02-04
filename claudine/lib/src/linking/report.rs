use std::fmt;
use std::path::PathBuf;

/// A single skill that was linked.
#[derive(Debug)]
pub struct LinkedEntry {
    /// Skill name.
    pub name: String,
    /// Source provider.
    pub source_provider: String,
    /// Target providers that received a link.
    pub target_providers: Vec<String>,
}

/// A single skill in sync across providers.
#[derive(Debug)]
pub struct InSyncEntry {
    /// Skill name.
    pub name: String,
    /// Providers that have identical content.
    pub providers: Vec<String>,
}

/// A single conflict between providers.
#[derive(Debug)]
pub struct ConflictEntry {
    /// Skill name.
    pub name: String,
    /// Each provider's path and hash.
    pub versions: Vec<(String, PathBuf, u64)>,
}

/// A skipped skill.
#[derive(Debug)]
pub struct SkippedEntry {
    /// Skill name.
    pub name: String,
    /// Why it was skipped.
    pub reason: String,
}

/// Report of all linking operations performed.
#[derive(Debug, Default)]
pub struct LinkReport {
    /// Skills that were linked.
    pub linked: Vec<LinkedEntry>,
    /// Skills that were already in sync.
    pub in_sync: Vec<InSyncEntry>,
    /// Skills with conflicts.
    pub conflicts: Vec<ConflictEntry>,
    /// Skills that were skipped.
    pub skipped: Vec<SkippedEntry>,
    /// Skills that were already linked.
    pub already_linked: Vec<String>,
}

impl LinkReport {
    /// Format the report for human-readable output.
    pub fn format_report(&self) -> String {
        let mut out = String::new();

        if !self.linked.is_empty() {
            out.push_str("  Linked:\n");
            for entry in &self.linked {
                let targets = entry.target_providers.join(", ");
                out.push_str(&format!(
                    "    \u{2713} {:<16} {} -> {}\n",
                    entry.name, entry.source_provider, targets
                ));
            }
            out.push('\n');
        }

        if !self.already_linked.is_empty() {
            out.push_str("  Already linked:\n");
            for name in &self.already_linked {
                out.push_str(&format!("    \u{2261} {name}\n"));
            }
            out.push('\n');
        }

        if !self.in_sync.is_empty() {
            out.push_str("  Already in sync:\n");
            for entry in &self.in_sync {
                let providers = entry.providers.join(" \u{2194} ");
                out.push_str(&format!(
                    "    \u{2261} {:<16} {} (identical content)\n",
                    entry.name, providers
                ));
            }
            out.push('\n');
        }

        if !self.skipped.is_empty() {
            out.push_str("  Skipped:\n");
            for entry in &self.skipped {
                out.push_str(&format!(
                    "    \u{25CB} {:<16} {}\n",
                    entry.name, entry.reason
                ));
            }
            out.push('\n');
        }

        if !self.conflicts.is_empty() {
            out.push_str("  Conflicts (different content):\n");
            for entry in &self.conflicts {
                let providers: Vec<String> = entry
                    .versions
                    .iter()
                    .map(|(p, _, h)| format!("{p} ({h:016x})"))
                    .collect();
                out.push_str(&format!(
                    "    \u{2717} {:<16} {}\n",
                    entry.name,
                    providers.join(" \u{2260} ")
                ));
            }
            out.push('\n');
        }

        let total_linked = self.linked.len();
        let total_in_sync = self.in_sync.len() + self.already_linked.len();
        let total_skipped = self.skipped.len();
        let total_conflicts = self.conflicts.len();

        out.push_str(&format!(
            "Summary: {} linked, {} in sync, {} skipped, {} conflicts\n",
            total_linked, total_in_sync, total_skipped, total_conflicts
        ));

        out
    }
}

impl fmt::Display for LinkReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_report())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_report_shows_zeroes() {
        let report = LinkReport::default();
        let output = report.format_report();
        assert!(output.contains("0 linked"));
        assert!(output.contains("0 in sync"));
        assert!(output.contains("0 skipped"));
        assert!(output.contains("0 conflicts"));
    }

    #[test]
    fn linked_entry_formatted_with_checkmark() {
        let report = LinkReport {
            linked: vec![LinkedEntry {
                name: "clap".to_string(),
                source_provider: "claude".to_string(),
                target_providers: vec![
                    "roo".to_string(),
                    "gemini".to_string(),
                ],
            }],
            ..Default::default()
        };

        let output = report.format_report();
        assert!(output.contains("\u{2713}"));
        assert!(output.contains("clap"));
        assert!(output.contains("claude"));
        assert!(output.contains("roo, gemini"));
        assert!(output.contains("1 linked"));
    }

    #[test]
    fn in_sync_entry_formatted() {
        let report = LinkReport {
            in_sync: vec![InSyncEntry {
                name: "tokio".to_string(),
                providers: vec!["claude".to_string(), "roo".to_string()],
            }],
            ..Default::default()
        };

        let output = report.format_report();
        assert!(output.contains("\u{2261}"));
        assert!(output.contains("tokio"));
        assert!(output.contains("identical content"));
        assert!(output.contains("1 in sync"));
    }

    #[test]
    fn conflict_entry_formatted_with_x() {
        let report = LinkReport {
            conflicts: vec![ConflictEntry {
                name: "react".to_string(),
                versions: vec![
                    ("claude".to_string(), PathBuf::from("/a"), 0x111),
                    ("roo".to_string(), PathBuf::from("/b"), 0x222),
                ],
            }],
            ..Default::default()
        };

        let output = report.format_report();
        assert!(output.contains("\u{2717}"));
        assert!(output.contains("react"));
        assert!(output.contains("1 conflicts"));
    }

    #[test]
    fn skipped_entry_formatted() {
        let report = LinkReport {
            skipped: vec![SkippedEntry {
                name: "chrono".to_string(),
                reason: "OpenCode reads Claude directly".to_string(),
            }],
            ..Default::default()
        };

        let output = report.format_report();
        assert!(output.contains("\u{25CB}"));
        assert!(output.contains("chrono"));
        assert!(output.contains("OpenCode reads Claude directly"));
        assert!(output.contains("1 skipped"));
    }

    #[test]
    fn display_trait_matches_format_report() {
        let report = LinkReport {
            linked: vec![LinkedEntry {
                name: "test".to_string(),
                source_provider: "claude".to_string(),
                target_providers: vec!["roo".to_string()],
            }],
            ..Default::default()
        };

        assert_eq!(format!("{report}"), report.format_report());
    }
}
