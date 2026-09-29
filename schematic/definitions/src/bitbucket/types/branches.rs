use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::Link;
use super::repos::CommitInfo;

/// A branch from `GET /repositories/{workspace}/{repo_slug}/refs/branches/{name}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Branch {
    /// Branch name.
    pub name: String,

    /// Head commit of the branch.
    pub target: CommitInfo,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_deserialization() {
        let json = r#"{
            "name": "feature/x",
            "type": "branch",
            "target": {
                "hash": "7fd1a60b01f91b314f59955a4e4d4e80d8edf11d",
                "type": "commit",
                "date": "2026-09-27T10:00:00+00:00",
                "message": "Add feature\n",
                "links": {
                    "html": {"href": "https://bitbucket.org/ws/repo/commits/7fd1a60b01f91b314f59955a4e4d4e80d8edf11d"}
                }
            },
            "links": {
                "html": {"href": "https://bitbucket.org/ws/repo/branch/feature/x"}
            },
            "merge_strategies": ["merge_commit", "squash", "fast_forward"],
            "default_merge_strategy": "merge_commit"
        }"#;

        let branch: Branch = serde_json::from_str(json).unwrap();
        assert_eq!(branch.name, "feature/x");
        assert_eq!(
            branch.target.hash.as_deref(),
            Some("7fd1a60b01f91b314f59955a4e4d4e80d8edf11d")
        );
    }
}
