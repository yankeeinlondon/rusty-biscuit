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
            ProtectIntent::ExecuteCommand(cq) => {
                Some(PolicyQuery::ExecuteCommand(cq.clone()))
            }
            ProtectIntent::AccessDomain(dq) => Some(PolicyQuery::AccessDomain(dq.clone())),
            ProtectIntent::UseMcpServer { server } => Some(PolicyQuery::UseMcpServer {
                server: server.clone(),
            }),
            ProtectIntent::UseMcpTool { server, tool } => Some(PolicyQuery::UseMcpTool {
                server: server.clone(),
                tool: tool.clone(),
            }),
            ProtectIntent::SpawnSubagent { name } => Some(PolicyQuery::SpawnSubagent {
                name: name.clone(),
            }),
            ProtectIntent::SwitchMode { target } => Some(PolicyQuery::SwitchMode {
                target: target.clone(),
            }),
            ProtectIntent::ModifyProviderConfig => Some(PolicyQuery::ModifyProviderConfig),
            ProtectIntent::CompletionOutputScan => None,
        }
    }
}
