use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Serialize;

use crate::error::Result;

use super::catalog::McpCatalogStore;
use super::defaults::{load_repo_defaults, load_user_defaults};
use super::state::McpProviderStateStore;
use super::types::{McpServer, McpTransport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues
            .iter()
            .all(|issue| issue.severity != ValidationSeverity::Error)
    }

    fn push(
        &mut self,
        severity: ValidationSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
        subject: Option<String>,
    ) {
        self.issues.push(ValidationIssue {
            severity,
            code: code.into(),
            message: message.into(),
            subject,
        });
    }
}

pub fn validate_state(
    catalog: &McpCatalogStore,
    state: &McpProviderStateStore,
    repo_root: Option<&Path>,
) -> Result<ValidationReport> {
    let mut report = ValidationReport::default();
    let mut seen_aliases: HashMap<String, String> = HashMap::new();

    for server in catalog.list_servers() {
        validate_server(&mut report, server);

        for alias in &server.aliases {
            if let Some(existing) = seen_aliases.insert(alias.clone(), server.id.clone()) {
                report.push(
                    ValidationSeverity::Error,
                    "duplicate-alias",
                    format!(
                        "alias `{alias}` is assigned to both `{existing}` and `{}`",
                        server.id
                    ),
                    Some(alias.clone()),
                );
            }

            if catalog.get_server(alias).is_some() {
                report.push(
                    ValidationSeverity::Error,
                    "alias-conflicts-with-id",
                    format!("alias `{alias}` conflicts with an existing server ID"),
                    Some(alias.clone()),
                );
            }
        }
    }

    let user_defaults = load_user_defaults()?;
    validate_defaults(
        &mut report,
        "user",
        &user_defaults.defaults,
        catalog,
        Some("~/.claudine/mcp/defaults.json".into()),
    );

    if let Some(repo_root) = repo_root
        && let Some(repo_defaults) = load_repo_defaults(repo_root)?
    {
        validate_defaults(
            &mut report,
            "repo",
            &repo_defaults.defaults,
            catalog,
            Some(biscuit_file::to_portable_string(
                &repo_root.join(".claudine/mcp.json"),
            )),
        );
    }

    let mut seen_provider_entries = HashSet::new();
    for (provider, scopes) in &state.state().providers {
        for entry in &scopes.user {
            let key = format!("{provider}:user:{}", entry.catalog_id);
            if !seen_provider_entries.insert(key) {
                report.push(
                    ValidationSeverity::Warning,
                    "duplicate-provider-state",
                    format!(
                        "provider-state has multiple user entries for `{provider}` / `{}`",
                        entry.catalog_id
                    ),
                    Some(entry.catalog_id.clone()),
                );
            }
            if catalog.get_server(&entry.catalog_id).is_none() {
                report.push(
                    ValidationSeverity::Warning,
                    "provider-state-missing-catalog-entry",
                    format!(
                        "provider-state points to missing catalog ID `{}` for provider `{provider}`",
                        entry.catalog_id
                    ),
                    Some(entry.catalog_id.clone()),
                );
            }
        }
    }

    for (repo, repo_state) in &state.state().repos {
        for (provider, scopes) in &repo_state.providers {
            for entry in &scopes.repo {
                let key = format!("{provider}:repo:{repo}:{}", entry.catalog_id);
                if !seen_provider_entries.insert(key) {
                    report.push(
                        ValidationSeverity::Warning,
                        "duplicate-provider-state",
                        format!(
                            "provider-state has multiple repo entries for `{provider}` / `{}`",
                            entry.catalog_id
                        ),
                        Some(entry.catalog_id.clone()),
                    );
                }
                if catalog.get_server(&entry.catalog_id).is_none() {
                    report.push(
                        ValidationSeverity::Warning,
                        "provider-state-missing-catalog-entry",
                        format!(
                            "provider-state points to missing catalog ID `{}` for provider `{provider}` in repo `{repo}`",
                            entry.catalog_id
                        ),
                        Some(entry.catalog_id.clone()),
                    );
                }
            }
        }
    }

    Ok(report)
}

fn validate_server(report: &mut ValidationReport, server: &McpServer) {
    match server.transport {
        McpTransport::Stdio => {
            if server.command.as_deref().is_none_or(str::is_empty) {
                report.push(
                    ValidationSeverity::Error,
                    "stdio-missing-command",
                    format!("local MCP server `{}` is missing its command", server.id),
                    Some(server.id.clone()),
                );
            }
            if server.url.is_some() {
                report.push(
                    ValidationSeverity::Warning,
                    "stdio-has-url",
                    format!(
                        "local MCP server `{}` has a URL that will be ignored for stdio transport",
                        server.id
                    ),
                    Some(server.id.clone()),
                );
            }
        }
        McpTransport::Http | McpTransport::Sse => {
            if server.url.as_deref().is_none_or(str::is_empty) {
                report.push(
                    ValidationSeverity::Error,
                    "remote-missing-url",
                    format!("remote MCP server `{}` is missing its URL", server.id),
                    Some(server.id.clone()),
                );
            }
        }
    }
}

fn validate_defaults(
    report: &mut ValidationReport,
    scope: &str,
    defaults: &[String],
    catalog: &McpCatalogStore,
    subject: Option<String>,
) {
    for id in defaults {
        if catalog.get_server(id).is_none() {
            report.push(
                ValidationSeverity::Warning,
                "missing-default",
                format!("{scope} defaults reference missing catalog ID `{id}`"),
                subject.clone(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;

    use super::*;
    use crate::mcp::state::Scope;
    use crate::mcp::types::{McpServerMetadata, McpTransport, ProviderStateEntry};
    use crate::provider::Provider;

    fn make_server(id: &str) -> McpServer {
        McpServer {
            id: id.into(),
            aliases: Vec::new(),
            transport: McpTransport::Stdio,
            command: Some("npx".into()),
            args: vec!["-y".into(), format!("@test/{id}")],
            cwd: None,
            env: HashMap::new(),
            url: None,
            headers: HashMap::new(),
            enabled_tools: Vec::new(),
            disabled_tools: Vec::new(),
            required: false,
            metadata: McpServerMetadata {
                description: None,
                created_from: None,
                fingerprint: String::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            provider_overrides: HashMap::new(),
        }
    }

    #[test]
    fn validation_catches_duplicate_aliases_and_invalid_stdio() {
        let mut catalog = McpCatalogStore::load_from(Path::new("/nonexistent")).unwrap();
        let mut a = make_server("calendar");
        a.aliases.push("shared".into());
        catalog.add_server(a);

        let mut b = make_server("slack");
        b.aliases.push("shared".into());
        b.command = None;
        catalog.add_server(b);

        let state = McpProviderStateStore::load_from(Path::new("/nonexistent")).unwrap();
        let report = validate_state(&catalog, &state, None).unwrap();
        assert!(!report.is_valid());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "duplicate-alias")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "stdio-missing-command")
        );
    }

    #[test]
    fn validation_warns_for_provider_state_missing_catalog_entries() {
        let catalog = McpCatalogStore::load_from(Path::new("/nonexistent")).unwrap();
        let mut state = McpProviderStateStore::load_from(Path::new("/nonexistent")).unwrap();
        state.record_import(
            Provider::Codex,
            &Scope::User,
            ProviderStateEntry {
                catalog_id: "missing".into(),
                native_name: "missing".into(),
                source: "~/.codex/config.toml".into(),
                origin: crate::mcp::types::McpOrigin::Imported,
                last_seen: Utc::now(),
            },
        );

        let report = validate_state(&catalog, &state, None).unwrap();
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "provider-state-missing-catalog-entry")
        );
    }
}
