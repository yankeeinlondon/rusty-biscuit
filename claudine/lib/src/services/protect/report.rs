use super::decision::ProtectMatch;

/// Format a blocked-action message for user output.
pub fn format_blocked_message(m: &ProtectMatch) -> String {
    let mut lines = Vec::new();
    lines.push("[protect] BLOCKED".to_string());
    lines.push(format!("  Group: {}", m.group));
    lines.push(format!("  Rule: {}", m.rule_id));
    lines.push(format!("  Pattern: {}", m.pattern));
    lines.push(format!("  Match: \"{}\"", m.matched_text));
    if let Some(ref path) = m.target_path {
        lines.push(format!("  Path: {path}"));
    }
    lines.push(String::new());
    lines.push("  Disable group:".to_string());
    lines.push(format!("    {} = false", m.config_key));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::protect::catalog::{RuleGroup, ScanSurface};
    use crate::services::protect::decision::ProtectMatch;

    #[test]
    fn format_blocked_message_matches_spec() {
        let m = ProtectMatch {
            group: RuleGroup::FilesystemDestruction,
            rule_id: "rm_root_glob".to_string(),
            pattern: r"(sudo\s+)?rm\s+-rf\s+/\*?".to_string(),
            matched_text: "rm -rf /var/*".to_string(),
            surface: ScanSurface::BashCommand,
            target_path: None,
            config_key: "protect.rules.filesystem_destruction".to_string(),
        };

        let msg = format_blocked_message(&m);
        assert!(msg.contains("[protect] BLOCKED"));
        assert!(msg.contains("Group: filesystem_destruction"));
        assert!(msg.contains("Rule: rm_root_glob"));
        assert!(msg.contains("rm -rf /var/*"));
        assert!(msg.contains("protect.rules.filesystem_destruction = false"));
    }
}
