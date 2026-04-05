use crate::permissions::query::{CommandQuery, DomainQuery, PathQuery, PolicyQuery};

/// Intent extracted from an event observation for policy evaluation.
#[derive(Debug, Clone)]
pub enum ProtectIntent {
    ReadPath(PathQuery),
    WritePath(PathQuery),
    TraversePath(PathQuery),
    ExecuteCommand(CommandQuery),
    AccessDomain(DomainQuery),
    UseMcpServer { server: String },
    UseMcpTool { server: String, tool: String },
    SpawnSubagent { name: Option<String> },
    SwitchMode { target: Option<String> },
    ModifyProviderConfig,
    CompletionOutputScan,
}

impl ProtectIntent {
    /// Convert this intent into a `PolicyQuery` for snapshot evaluation.
    ///
    /// Returns `None` for intents that have no corresponding policy query
    /// (e.g., `CompletionOutputScan` is Protect-internal).
    pub fn to_policy_query(&self) -> Option<PolicyQuery> {
        match self {
            ProtectIntent::ReadPath(pq) => Some(PolicyQuery::ReadPath(pq.clone())),
            ProtectIntent::WritePath(pq) => Some(PolicyQuery::WritePath(pq.clone())),
            ProtectIntent::TraversePath(pq) => Some(PolicyQuery::TraversePath(pq.clone())),
            ProtectIntent::ExecuteCommand(cq) => Some(PolicyQuery::ExecuteCommand(cq.clone())),
            ProtectIntent::AccessDomain(dq) => Some(PolicyQuery::AccessDomain(dq.clone())),
            ProtectIntent::UseMcpServer { server } => Some(PolicyQuery::UseMcpServer {
                server: server.clone(),
            }),
            ProtectIntent::UseMcpTool { server, tool } => Some(PolicyQuery::UseMcpTool {
                server: server.clone(),
                tool: tool.clone(),
            }),
            ProtectIntent::SpawnSubagent { name } => {
                Some(PolicyQuery::SpawnSubagent { name: name.clone() })
            }
            ProtectIntent::SwitchMode { target } => Some(PolicyQuery::SwitchMode {
                target: target.clone(),
            }),
            ProtectIntent::ModifyProviderConfig => Some(PolicyQuery::ModifyProviderConfig),
            ProtectIntent::CompletionOutputScan => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_scan_does_not_map_to_policy_query() {
        assert!(
            ProtectIntent::CompletionOutputScan
                .to_policy_query()
                .is_none()
        );
    }

    #[test]
    fn mcp_tool_intent_maps_to_policy_query() {
        let query = ProtectIntent::UseMcpTool {
            server: "filesystem".to_string(),
            tool: "read_file".to_string(),
        }
        .to_policy_query();

        match query {
            Some(PolicyQuery::UseMcpTool { server, tool }) => {
                assert_eq!(server, "filesystem");
                assert_eq!(tool, "read_file");
            }
            other => panic!("expected MCP tool query, got {other:?}"),
        }
    }
}
