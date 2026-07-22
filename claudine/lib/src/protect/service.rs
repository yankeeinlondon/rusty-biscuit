use std::borrow::Cow;

use tracing::{debug, info_span};

use crate::error::Result;

use super::catalog::{ProtectPlatform, RuleGroup, ScanSurface};
use super::config::ProtectConfig;
use super::decision::{ProtectDecision, ProtectMatch};
use super::matcher::CompiledCatalog;
use super::path::{
    SensitivePathChecker, all_targets_allowed, extract_target_paths, normalize_path,
};

/// Evaluation request for the protect service.
#[derive(Debug)]
pub enum ProtectRequest<'a> {
    BashCommand { command: Cow<'a, str> },
    WritePath {
        paths: Vec<&'a str>,
        cwd: Option<&'a str>,
    },
    McpResponse { payloads: Vec<Cow<'a, str>> },
}

/// Standalone deny-catalog matcher service.
///
/// No PolicyEngine, no postures, no capability downgrade. Receives a
/// ProtectRequest and returns a deterministic Allow or Block.
#[derive(Debug, Clone)]
pub struct ProtectService {
    catalog: CompiledCatalog,
    config: ProtectConfig,
    path_checker: SensitivePathChecker,
}

impl ProtectService {
    /// Build a protect service from config and platform.
    pub fn new(config: ProtectConfig, platform: ProtectPlatform) -> Result<Self> {
        config.validate()?;
        let catalog = CompiledCatalog::new(&config, platform)?;
        Ok(Self {
            catalog,
            config,
            path_checker: SensitivePathChecker::new(),
        })
    }

    /// Evaluate a request against the deny catalog.
    pub fn evaluate(&self, request: &ProtectRequest) -> ProtectDecision {
        let surface = match request {
            ProtectRequest::BashCommand { .. } => "bash_command",
            ProtectRequest::WritePath { .. } => "write_path",
            ProtectRequest::McpResponse { .. } => "mcp_response",
        };
        let _span =
            info_span!("protect_evaluate", surface, enabled = self.config.enabled,).entered();

        if !self.config.enabled {
            debug!(surface, "protect disabled, allowing");
            return ProtectDecision::allow();
        }

        let decision = match request {
            ProtectRequest::BashCommand { command } => self.evaluate_bash_command(command.as_ref()),
            ProtectRequest::WritePath { paths, cwd } => self.evaluate_write_path(paths, *cwd),
            ProtectRequest::McpResponse { payloads } => self.evaluate_mcp_response(payloads),
        };

        debug!(
            outcome = ?decision.outcome,
            finding_count = if decision.is_blocked() { 1 } else { 0 },
            "protect evaluation complete",
        );

        decision
    }

    fn evaluate_bash_command(&self, command: &str) -> ProtectDecision {
        let command_truncated = command
            .char_indices()
            .nth(80)
            .map_or(command, |(i, _)| &command[..i]);
        let _span = info_span!("protect_bash", command_truncated).entered();
        for group in &self.catalog.command_groups {
            if let Some((m, rule_supports_allow_paths)) = group.find_match(command) {
                if rule_supports_allow_paths
                    && let Some(allow_paths) = self.config.get_allow_paths(group.group)
                {
                    let targets = extract_target_paths(command);
                    if all_targets_allowed(&targets, allow_paths) {
                        debug!(
                            group = %group.group,
                            rule_id = %m.rule_id,
                            "match suppressed by allow_paths",
                        );
                        continue;
                    }
                }
                debug!(
                    group = %group.group,
                    rule_id = %m.rule_id,
                    matched_text = %m.matched_text,
                    "command blocked",
                );
                return ProtectDecision::blocked(m);
            }
        }

        if let Some(custom) = &self.catalog.custom_command_group
            && let Some((m, _)) = custom.find_match(command)
        {
            debug!(
                group = "custom",
                rule_id = %m.rule_id,
                matched_text = %m.matched_text,
                "custom rule blocked command",
            );
            return ProtectDecision::blocked(m);
        }

        ProtectDecision::allow()
    }

    fn evaluate_write_path(&self, paths: &[&str], cwd: Option<&str>) -> ProtectDecision {
        let _span = info_span!("protect_write", path_count = paths.len()).entered();
        if !self.config.is_group_enabled(RuleGroup::SensitivePaths) {
            return ProtectDecision::allow();
        }

        // Any sensitive path blocks the whole write; a benign first entry must
        // not shadow a later sensitive one.
        for path in paths {
            let decision = self.evaluate_single_write_path(path, cwd);
            if decision.is_blocked() {
                return decision;
            }
        }

        ProtectDecision::allow()
    }

    fn evaluate_single_write_path(&self, path: &str, cwd: Option<&str>) -> ProtectDecision {
        // Resolve relative paths against cwd
        let resolved = match cwd {
            Some(cwd) if !path.starts_with('/') && !path.starts_with('~') => {
                normalize_path(&format!("{cwd}/{path}"))
            }
            _ => normalize_path(path),
        };

        // Canonicalize existing ancestors to resolve symlinks
        let resolved = super::path::canonicalize_existing_ancestor(&resolved);
        let resolved_str = resolved.to_string_lossy();

        if self.path_checker.is_sensitive(&resolved_str) {
            // Check allow_paths suppression
            if let Some(allow_paths) = self.config.get_allow_paths(RuleGroup::SensitivePaths)
                && super::path::is_path_allowed(&resolved_str, allow_paths)
            {
                debug!(path = %resolved_str, "sensitive path suppressed by allow_paths");
                return ProtectDecision::allow();
            }
            debug!(path = %resolved_str, "sensitive path blocked");
            return ProtectDecision::blocked(ProtectMatch {
                group: RuleGroup::SensitivePaths,
                rule_id: "sensitive_prefix".to_string(),
                pattern: String::new(),
                matched_text: resolved_str.to_string(),
                surface: ScanSurface::WritePath,
                target_path: Some(resolved_str.to_string()),
                config_key: "protect.rules.sensitive_paths".to_string(),
            });
        }

        ProtectDecision::allow()
    }

    fn evaluate_mcp_response(&self, payloads: &[Cow<str>]) -> ProtectDecision {
        let _span = info_span!("protect_mcp", payload_count = payloads.len()).entered();
        for payload in payloads {
            if let Some(m) = self.catalog.evaluate_mcp(payload) {
                debug!(
                    group = %m.group,
                    rule_id = %m.rule_id,
                    matched_text = %m.matched_text,
                    "MCP payload blocked",
                );
                return ProtectDecision::blocked(m);
            }
        }
        ProtectDecision::allow()
    }
}

#[cfg(test)]
mod tests;
